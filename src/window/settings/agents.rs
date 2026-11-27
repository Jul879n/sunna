//! Settings > Agents: starting every agent without permission prompts
//! (settings.json `yolo`), and the command each installed agent starts with
//! (programs.json `programs.<key>.run`).

use super::*;
use controls::{card, row, switch};
use gpui_base::input::{InputEvent, InputState};

/// An installed agent this computer can start.
#[derive(Clone, Debug, PartialEq)]
pub struct Agent {
    pub key: String,
    pub name: String,
    /// The command it starts with now.
    pub base: Vec<String>,
    /// The flags that skip its permission prompts (none: it has no such mode).
    pub yolo: Vec<String>,
}

#[derive(Default)]
pub struct State {
    pub known: Option<Vec<(Agent, Entity<InputState>)>>,
    pub failed: bool,
    pub loading: bool,
    pub focused: Option<String>,
}

fn words(v: &Value) -> Vec<String> {
    v.as_array().map(|l| l.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect()).unwrap_or_default()
}

/// The installed ones among what `known_agents` answers.
pub fn installed(list: &Value) -> Vec<Agent> {
    list.as_array()
        .map(|l| {
            l.iter()
                .filter(|a| a["installed"] == json!(true))
                .filter_map(|a| Some(Agent { key: a["key"].as_str()?.to_owned(), name: a["name"].as_str().unwrap_or_default().to_owned(), base: words(&a["base"]), yolo: words(&a["yolo"]) }))
                .collect()
        })
        .unwrap_or_default()
}

/// `programs.<key>.run` set to `run`, or dropped when it is empty (the shipped command again).
pub fn set_run(o: &mut Object, key: &str, run: &[String]) {
    let mut programs = o.get("programs").and_then(Value::as_object).cloned().unwrap_or_default();
    let mut entry = programs.get(key).and_then(Value::as_object).cloned().unwrap_or_default();
    if run.is_empty() {
        entry.remove("run");
    } else {
        entry.insert("run".into(), json!(run));
    }
    programs.insert(key.to_owned(), Value::Object(entry));
    o.insert("programs".into(), Value::Object(programs));
}

/// `yolo.enabled` flipped, keeping the per-agent choices.
pub fn set_all(q: &mut Object, on: bool) {
    let mut y = q.get("yolo").and_then(Value::as_object).cloned().unwrap_or_default();
    y.insert("enabled".into(), json!(on));
    q.insert("yolo".into(), Value::Object(y));
}

/// `yolo.agents.<key>` set, keeping the rest.
pub fn set_one(q: &mut Object, key: &str, on: bool) {
    let mut y = q.get("yolo").and_then(Value::as_object).cloned().unwrap_or_default();
    let mut agents = y.get("agents").and_then(Value::as_object).cloned().unwrap_or_default();
    agents.insert(key.to_owned(), json!(on));
    y.insert("agents".into(), Value::Object(agents));
    q.insert("yolo".into(), Value::Object(y));
}

impl SettingsView {
    /// Asks which agents are installed (off the window thread), once per visit.
    fn load_agents(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.agents.known.is_some() || self.agents.loading {
            return;
        }
        self.agents.loading = true;
        self.agents.failed = false;
        let list = crate::programs::startable();
        let (tx, rx) = async_channel::bounded(1);
        std::thread::spawn(move || {
            let _ = tx.send_blocking(crate::mods::known_agents(None, list));
        });
        cx.spawn_in(window, async move |this, cx| {
            let answer = rx.recv().await.ok().and_then(Result::ok);
            let _ = this.update_in(cx, |this, window, cx| {
                this.agents.loading = false;
                match answer {
                    Some(list) => {
                        let mut known = Vec::new();
                        for a in installed(&list) {
                            let input = cx.new(|cx| InputState::new(window, cx).placeholder(a.base.join(" ")));
                            input.update(cx, |s, cx| {
                                s.set_editor_style(controls::input_style());
                                s.set_value(a.base.join(" "), window, cx);
                            });
                            let (key, base) = (a.key.clone(), a.base.join(" "));
                            cx.subscribe_in(&input, window, move |this, input, ev: &InputEvent, window, cx| match ev {
                                InputEvent::Focus => this.agents.focused = Some(key.clone()),
                                InputEvent::Blur | InputEvent::PressEnter { .. } => {
                                    if matches!(ev, InputEvent::Blur) && this.agents.focused.as_deref() == Some(key.as_str()) {
                                        this.agents.focused = None;
                                    }
                                    let text = input.read(cx).value().to_string();
                                    if text.split_whitespace().collect::<Vec<_>>().join(" ") != base {
                                        let run: Vec<String> = text.split_whitespace().map(str::to_owned).collect();
                                        if let Err(f) = store::edit_file("programs.json", "{}", |o| set_run(o, &key, &run)) {
                                            this.toast(f, window, cx);
                                        }
                                        this.agents.known = None;
                                    }
                                    cx.notify();
                                }
                                _ => {}
                            })
                            .detach();
                            known.push((a, input));
                        }
                        this.agents.known = Some(known);
                    }
                    None => this.agents.failed = true,
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn set_hooks(&mut self, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        match crate::agent_hooks::set(on) {
            Ok(()) => cx.notify(),
            Err(e) => self.notify(json!({ "level": "error", "title": e }), window, cx),
        }
    }

    pub(super) fn agents(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        self.load_agents(window, cx);
        let yolo = self.person.get("yolo").and_then(Value::as_object).cloned().unwrap_or_default();
        let all = yolo.get("enabled").and_then(Value::as_bool).unwrap_or(true);
        let mut page = div().w_full().child(controls::heading(i18n::t("settings.agents"))).child(row(
            i18n::t("settings.full_access"),
            Some(i18n::t("settings.full_access_about")),
            switch("st-yolo", all, self.act(cx, move |this, w, cx| this.change(w, cx, |q| set_all(q, !all)))),
        ));
        let hooked = crate::agent_hooks::is_installed();
        let hook_button = if hooked {
            controls::button("st-hooks-off", i18n::t("settings.agent_hooks_remove"), self.act(cx, |this, w, cx| this.set_hooks(false, w, cx)))
        } else {
            controls::button("st-hooks-on", i18n::t("settings.agent_hooks_install"), self.act(cx, |this, w, cx| this.set_hooks(true, w, cx)))
        };
        page = page.child(row(
            i18n::t(if hooked { "settings.agent_hooks_on" } else { "settings.agent_hooks_off" }),
            Some(i18n::t("settings.agent_hooks_about")),
            hook_button,
        ));
        if self.agents.failed {
            page = page.child(controls::note(i18n::t("settings.agents_failed")));
        } else if self.agents.known.is_none() {
            page = page.child(controls::note(i18n::t("settings.agents_checking")));
        }
        let known = self.agents.known.clone().unwrap_or_default();
        let mut group = card();
        for (a, input) in known {
            let on = yolo.get("agents").and_then(|m| m.get(&a.key)).and_then(Value::as_bool).unwrap_or(all);
            let key = a.key.clone();
            let focused = self.agents.focused.as_deref() == Some(a.key.as_str());
            group = group.child(
                div()
                    .id(SharedString::from(format!("st-agent-{}", a.key)))
                    .w_full()
                    .min_h(settings::z(44.0))
                    .px(settings::z(10.0))
                    .py(settings::z(6.0))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(settings::z(12.0))
                    .child(div().w(settings::z(130.0)).flex_none().text_size(settings::z(14.0)).text_color(c("foreground")).child(a.name.clone()))
                    .child(controls::field_fill(&input, focused))
                    .child(div().w(settings::z(190.0)).flex_none().truncate().text_size(settings::z(12.0)).text_color(tone("muted")).child(a.yolo.join(" ")))
                    .child(if a.yolo.is_empty() {
                        div().w(settings::z(44.0)).flex_none()
                    } else {
                        div().flex_none().child(switch(format!("st-yolo-{}", a.key), on, self.act(cx, move |this, w, cx| this.change(w, cx, |q| set_one(q, &key, !on)))))
                    }),
            );
        }
        if self.agents.known.as_ref().is_some_and(|k| !k.is_empty()) {
            page = page.child(group);
        }
        page
    }
}

#[cfg(test)]
mod tests {
    use super::{Object, installed, set_all, set_one, set_run};
    use serde_json::{Value, json};

    fn obj(text: &str) -> Object {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn only_installed_agents_are_listed() {
        let list = json!([
            {"key": "claude", "name": "Claude Code", "installed": true, "base": ["claude"], "yolo": ["--x"]},
            {"key": "pi", "name": "Pi", "installed": false, "base": ["pi"], "yolo": []},
            {"key": "goose", "name": "goose", "installed": true, "base": ["goose"]}
        ]);
        let got = installed(&list);
        assert_eq!(got.iter().map(|a| a.key.as_str()).collect::<Vec<_>>(), ["claude", "goose"]);
        assert_eq!(got[0].yolo, ["--x"]);
        assert!(got[1].yolo.is_empty());
    }

    #[test]
    fn a_command_is_kept_per_agent_and_empty_means_the_shipped_one() {
        let mut o = obj(r#"{"programs": {"claude": {"run": ["c"], "icon": "i"}}}"#);
        set_run(&mut o, "claude", &["claude".into(), "--x".into()]);
        assert_eq!(o["programs"]["claude"]["run"], json!(["claude", "--x"]));
        assert_eq!(o["programs"]["claude"]["icon"], "i");
        set_run(&mut o, "claude", &[]);
        assert!(o["programs"]["claude"].get("run").is_none());
        set_run(&mut o, "pi", &["pi".into()]);
        assert_eq!(o["programs"]["pi"]["run"], json!(["pi"]));
    }

    #[test]
    fn the_permission_switches_keep_each_other_and_the_rest() {
        let mut q = obj(r#"{"zoom": 1.1}"#);
        set_all(&mut q, false);
        set_one(&mut q, "claude", true);
        set_one(&mut q, "codex", false);
        assert_eq!(Value::Object(q.clone()), serde_json::from_str::<Value>(r#"{"zoom":1.1,"yolo":{"enabled":false,"agents":{"claude":true,"codex":false}}}"#).unwrap());
        set_all(&mut q, true);
        assert_eq!(q["yolo"]["agents"]["claude"], true);
        assert_eq!(q["yolo"]["enabled"], true);
    }
}
