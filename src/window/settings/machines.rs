//! Settings > Machines: the computers reached over SSH (`hc machine ls/update/
//! rename/rm`), polled while the page is open. Adding one opens the
//! `machine-add` mod.

use super::*;
use controls::{button, primary, row};
use std::collections::HashMap;
use std::time::Duration;

const LIMIT: Duration = Duration::from_secs(120);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Update,
    Rename,
    Remove,
}

impl Action {
    fn arg(self) -> &'static str {
        match self {
            Action::Update => "update",
            Action::Rename => "rename",
            Action::Remove => "rm",
        }
    }
}

pub struct State {
    pub list: Option<Vec<Value>>,
    pub error: Option<String>,
    loading: bool,
    busy: HashMap<String, Action>,
    results: HashMap<String, (String, bool)>,
    edit: Option<(String, Action)>,
    notice: Option<String>,
    copied: Option<String>,
    pub rename: Entity<InputState>,
    pub rename_text: String,
}

impl State {
    pub fn new(window: &mut Window, cx: &mut Context<SettingsView>) -> Self {
        let rename = cx.new(|cx| InputState::new(window, cx));
        rename.update(cx, |s, _| s.set_editor_style(controls::input_style()));
        cx.subscribe_in(&rename, window, |this, input, ev: &InputEvent, w, cx| match ev {
            InputEvent::Change => this.machines.rename_text = input.read(cx).value().to_string(),
            InputEvent::PressEnter { .. } => {
                if let Some((name, Action::Rename)) = this.machines.edit.clone() {
                    let text = this.machines.rename_text.clone();
                    this.machine_action(&name, Action::Rename, Some(text), w, cx);
                }
            }
            _ => {}
        })
        .detach();
        Self { list: None, error: None, loading: false, busy: HashMap::new(), results: HashMap::new(), edit: None, notice: None, copied: None, rename, rename_text: String::new() }
    }
}

/// The machines `hc machine ls --json` printed, the temporary ones left out.
pub fn parse_list(stdout: &str) -> Result<Vec<Value>, String> {
    let v: Value = serde_json::from_str(stdout.trim()).map_err(|_| i18n::t("settings.couldn_t_read_machines"))?;
    let list = v.get("machines").and_then(Value::as_array).ok_or_else(|| i18n::t("settings.couldn_t_read_machines"))?;
    Ok(list.iter().filter(|m| m.get("temporary") != Some(&json!(true)) && m.get("temporary").is_none_or(|t| t.is_null() || t == &json!(false))).cloned().collect())
}

struct Version {
    core: [u64; 3],
    pre: Option<String>,
}

fn version(v: &str) -> Option<Version> {
    let v = v.split('+').next()?;
    let (core, pre) = match v.split_once('-') {
        Some((c, p)) => (c, Some(p.to_owned())),
        None => (v, None),
    };
    let mut parts = core.split('.');
    let mut n = [0u64; 3];
    for slot in &mut n {
        *slot = parts.next()?.parse().ok()?;
    }
    parts.next().is_none().then_some(Version { core: n, pre })
}

/// Whether the machine's core `remote` is older than ours (semver order, with prereleases).
pub fn older_core(remote: &str, local: &str) -> bool {
    let (Some(a), Some(b)) = (version(remote), version(local)) else { return false };
    if a.core != b.core {
        return a.core < b.core;
    }
    let (Some(x), Some(y)) = (&a.pre, &b.pre) else { return a.pre.is_some() && b.pre.is_none() };
    let (x, y): (Vec<&str>, Vec<&str>) = (x.split('.').collect(), y.split('.').collect());
    for i in 0..x.len().max(y.len()) {
        let (p, q) = (x.get(i), y.get(i));
        if p == q {
            continue;
        }
        let (Some(p), Some(q)) = (p, q) else { return p.is_none() };
        let num = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
        return match (num(p), num(q)) {
            (true, true) => p.parse::<u64>().unwrap_or(0) < q.parse::<u64>().unwrap_or(0),
            (a, b) if a != b => a,
            _ => p < q,
        };
    }
    false
}

/// What a finished action says: (text, shown as a result of that machine?).
pub fn outcome(action: Action, machine: &str, new_name: &str, code: i32, stdout: &str, stderr: &str) -> Result<String, String> {
    let payload: Option<Value> = serde_json::from_str(if stdout.trim().is_empty() { stderr.trim() } else { stdout.trim() }).ok();
    if code != 0 {
        let why = payload.as_ref().and_then(|p| p["error"]["message"].as_str()).map(str::to_owned);
        let raw = if stderr.trim().is_empty() { stdout.trim() } else { stderr.trim() };
        return Err(why.unwrap_or_else(|| if raw.is_empty() { i18n::t("settings.it_didn_t_work") } else { raw.to_owned() }));
    }
    Ok(match action {
        Action::Update => match payload.as_ref().and_then(|p| p["version"].as_str()) {
            Some(v) => i18n::tf("settings.updated_machine_to_hc_version", &[("machine", machine), ("version", v)]),
            None => i18n::tf("settings.updated_machine_its_terminals_kept", &[("machine", machine)]),
        },
        Action::Rename => i18n::tf("settings.renamed_machine_to_name", &[("machine", machine), ("name", new_name)]),
        Action::Remove => i18n::tf("settings.removed_machine_its_terminals_keep", &[("machine", machine)]),
    })
}

fn state_label(state: &str) -> String {
    let key = match state {
        "connected" => "settings.connected_2",
        "online" => "settings.online",
        "connecting" => "settings.connecting",
        "reconnecting" => "settings.reconnecting",
        "offline" => "settings.offline",
        "unknown" => "settings.unknown",
        "needs_login" => "settings.needs_login",
        other => return other.replace(['_', '-'], " "),
    };
    i18n::t(key)
}

impl SettingsView {
    /// Leaving the page drops a rename or removal that was being asked.
    pub(super) fn machines_cancel_edit(&mut self) {
        self.machines.edit = None;
    }

    /// Reads the machine list in the background (one read at a time).
    pub(super) fn load_machines(&mut self, cx: &mut Context<Self>) {
        if self.machines.loading {
            return;
        }
        self.machines.loading = true;
        cx.spawn(async move |this, cx| {
            let r = cx.background_executor().spawn(async { mods::run_blocking("hc", &["machine".into(), "ls".into(), "--json".into()], None, LIMIT) }).await;
            let _ = this.update(cx, |this, cx| {
                let m = &mut this.machines;
                m.loading = false;
                match r {
                    Ok((0, out, _)) => match parse_list(&out) {
                        Ok(list) => (m.list, m.error) = (Some(list), None),
                        Err(e) => m.error = Some(e),
                    },
                    Ok((_, out, err)) => m.error = Some(if err.trim().is_empty() { out.trim().to_owned() } else { err.trim().to_owned() }).filter(|s| !s.is_empty()).or_else(|| Some(i18n::t("settings.couldn_t_read_machines"))),
                    Err(e) => m.error = Some(e),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn machine_action(&mut self, machine: &str, action: Action, new_name: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let m = &mut self.machines;
        if m.busy.contains_key(machine) {
            return;
        }
        if action == Action::Rename && new_name.as_deref().is_none_or(|n| n.trim().is_empty() || n.trim() == machine) {
            return;
        }
        m.busy.insert(machine.to_owned(), action);
        m.results.remove(machine);
        m.notice = None;
        m.edit = None;
        self.focus.focus(window, cx);
        let (machine, new_name) = (machine.to_owned(), new_name.unwrap_or_default().trim().to_owned());
        let mut args: Vec<String> = vec!["machine".into(), action.arg().into(), machine.clone()];
        if action == Action::Rename {
            args.push(new_name.clone());
        }
        cx.spawn(async move |this, cx| {
            let r = cx.background_executor().spawn(async move { mods::run_blocking("hc", &args, None, LIMIT) }).await;
            let _ = this.update(cx, |this, cx| {
                let result = match r {
                    Ok((code, out, err)) => outcome(action, &machine, &new_name, code, &out, &err),
                    Err(e) => Err(e),
                };
                let m = &mut this.machines;
                m.busy.remove(&machine);
                match (result, action) {
                    (Ok(text), Action::Update) => drop(m.results.insert(machine, (text, true))),
                    (Ok(text), _) => m.notice = Some(text),
                    (Err(text), _) => drop(m.results.insert(machine, (text, false))),
                }
                this.load_machines(cx);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn machine_card(&mut self, m: &Value, local: &str, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let name = m["name"].as_str().unwrap_or_default().to_owned();
        let status = m["state"].as_str().unwrap_or("unknown");
        let dot = match status {
            "connected" | "online" => c("ansi2"),
            "connecting" | "reconnecting" => c("working"),
            _ => c("blocked"),
        };
        let hc = m["version"].as_str().filter(|v| !v.is_empty()).map_or_else(|| i18n::t("settings.hc_version_unknown"), |v| format!("hc {v}"));
        let about = format!("{} · {} · {hc}", m["target"].as_str().unwrap_or_default(), state_label(status));
        let busy = self.machines.busy.get(&name).copied();
        let edit = self.machines.edit.clone().filter(|(n, _)| *n == name).map(|(_, a)| a);
        let z = settings::z;

        let mut controls_row = div().flex().flex_row().items_center().gap(z(6.0));
        if let Some(action) = busy {
            let label = match action {
                Action::Update => "settings.updating",
                Action::Rename => "settings.renaming",
                Action::Remove => "settings.removing",
            };
            controls_row = controls_row.child(div().text_size(z(12.5)).text_color(tone("working")).child(i18n::t(label)));
        } else {
            if m["version"].as_str().is_some_and(|v| older_core(v, local)) {
                let n = name.clone();
                controls_row = controls_row.child(button(format!("st-m-update-{name}"), i18n::t("settings.update"), self.act(cx, move |this, w, cx| this.machine_action(&n, Action::Update, None, w, cx))));
            }
            for (action, label) in [(Action::Rename, "settings.rename"), (Action::Remove, "settings.remove")] {
                let n = name.clone();
                controls_row = controls_row.child(button(
                    format!("st-m-{}-{name}", action.arg()),
                    i18n::t(label),
                    self.act(cx, move |this, w, cx| {
                        this.machines.edit = Some((n.clone(), action));
                        if action == Action::Rename {
                            this.machines.rename_text = n.clone();
                            let input = this.machines.rename.clone();
                            input.update(cx, |s, cx| {
                                s.set_value(n.clone(), w, cx);
                                s.focus(w, cx);
                                s.select_all(w, cx);
                            });
                        }
                    }),
                ));
            }
        }

        let mut card = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(z(8.0))
            .px(z(16.0))
            .py(z(12.0))
            .mb(z(8.0))
            .rounded(z(10.0))
            .border_1()
            .border_color(c("border"))
            .bg(c("surface"))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(z(12.0))
                    .child(div().w(z(8.0)).h(z(8.0)).flex_none().rounded_full().bg(dot))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(z(3.0))
                            .child(div().text_size(z(14.0)).text_color(c("foreground")).child(name.clone()))
                            .child(div().text_size(z(12.5)).text_color(tone("dim")).child(about)),
                    )
                    .child(controls_row),
            );
        if m["keeps_running_after_logout"] == json!(false) {
            card = card.child(
                div()
                    .id(SharedString::from(format!("st-m-logout-{name}")))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(z(8.0))
                    .text_size(z(12.5))
                    .text_color(tone("blocked"))
                    .child(i18n::t("settings.agents_stop_when_you_disconnect"))
                    .child(button(format!("st-m-logout-fix-{name}"), i18n::t("settings.add_a_machine"), self.act(cx, |this, w, cx| this.run("mod.open", json!({ "name": "machine-add" }), w, cx)))),
            );
        }
        if status == "needs_login" {
            let command = format!("hc machine login {name}");
            let copied = self.machines.copied.as_deref() == Some(command.as_str());
            let copy = command.clone();
            card = card
                .child(div().text_size(z(12.5)).text_color(tone("blocked")).child(i18n::t("settings.needs_a_login_run")))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(z(8.0))
                        .child(div().flex_1().min_w_0().text_size(z(12.5)).font_family("JetBrains Mono").text_color(c("foreground")).child(command))
                        .child(button(
                            format!("st-m-login-copy-{name}"),
                            if copied { i18n::t("settings.copied") } else { i18n::t("settings.copy") },
                            self.act(cx, move |this, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(copy.clone()));
                                this.machines.copied = Some(copy.clone());
                            }),
                        )),
                );
        }
        if busy == Some(Action::Update) {
            card = card.child(div().text_size(z(12.5)).text_color(tone("dim")).child(i18n::t("settings.copying_this_computer_s_herdr_core_its")));
        }
        if busy.is_none() {
            let cancel = |this: &Self, cx: &mut Context<Self>, id: String| button(id, i18n::t("settings.cancel"), this.act(cx, |this, w, cx| {
                this.machines.edit = None;
                this.focus.focus(w, cx);
            }));
            match edit {
                Some(Action::Remove) => {
                    let n = name.clone();
                    card = card
                        .child(div().text_size(z(12.5)).text_color(tone("muted")).child(i18n::tf("settings.remove_name_its_terminals_keep", &[("name", &name)])))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap(z(6.0))
                                .child(cancel(self, cx, format!("st-m-cancel-{name}")))
                                .child(primary(format!("st-m-confirm-{name}"), i18n::t("settings.remove_machine"), self.act(cx, move |this, w, cx| this.machine_action(&n, Action::Remove, None, w, cx)))),
                        );
                }
                Some(Action::Rename) => {
                    let n = name.clone();
                    let focused = self.machines.rename.read(cx).focus_handle(cx).is_focused(window);
                    let field = controls::field_ring(&self.machines.rename, 260.0, focused);
                    card = card
                        .child(div().text_size(z(12.0)).text_color(tone("dim")).child(i18n::t("settings.new_name_addresses_using_the")))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(z(6.0))
                                .child(field)
                                .child(cancel(self, cx, format!("st-m-cancel-{name}")))
                                .child(primary(
                                    format!("st-m-save-{name}"),
                                    i18n::t("settings.save"),
                                    self.act(cx, move |this, w, cx| {
                                        let text = this.machines.rename_text.clone();
                                        this.machine_action(&n, Action::Rename, Some(text), w, cx)
                                    }),
                                )),
                        );
                }
                _ => {}
            }
        }
        if let Some((text, ok)) = self.machines.results.get(&name) {
            card = card.child(div().text_size(z(12.5)).text_color(if *ok { c("ansi2") } else { c("blocked") }).child(text.clone()));
        }
        card
    }

    pub(super) fn machines_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let z = settings::z;
        let local = crate::update::status()["core"]["version"].as_str().unwrap_or_default().to_owned();
        let hc = if local.is_empty() { i18n::t("settings.hc_version_unknown") } else { format!("hc {local}") };
        let mut page = div()
            .w_full()
            .child(controls::heading(i18n::t("settings.machines")))
            .child(controls::note(i18n::t("settings.your_computers_reached_over_ssh")))
            .child(row(
                i18n::t("settings.this_computer"),
                Some(format!("{} · {hc}", i18n::t("settings.connected"))),
                div().w(z(8.0)).h(z(8.0)).rounded_full().bg(c("ansi2")),
            ));
        let list = self.machines.list.clone();
        for m in list.iter().flatten() {
            page = page.child(self.machine_card(m, &local, window, cx));
        }
        let status = |text: String, color: &str| div().py(z(12.0)).text_size(z(12.5)).text_color(c(color)).child(text);
        match (&list, &self.machines.error) {
            (None, None) => page = page.child(status(i18n::t("settings.reading_machines"), "dim")),
            (Some(l), _) if l.is_empty() => page = page.child(status(i18n::t("settings.no_other_machines_yet"), "dim")),
            _ => {}
        }
        if let Some(e) = &self.machines.error {
            page = page.child(status(e.clone(), "blocked"));
        }
        if let Some(n) = &self.machines.notice {
            page = page.child(status(n.clone(), "ansi2"));
        }
        page.child(div().pt(z(16.0)).flex().child(button("st-m-add", i18n::t("settings.add_a_machine"), self.act(cx, |this, w, cx| this.run("mod.open", json!({ "name": "machine-add" }), w, cx)))))
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, older_core, outcome, parse_list, state_label};
    use crate::i18n;

    #[test]
    fn the_list_leaves_temporary_machines_out() {
        let list = parse_list(r#"{"machines":[{"name":"a"},{"name":"t","temporary":true},{"name":"b","temporary":false}]}"#).unwrap();
        assert_eq!(list.iter().map(|m| m["name"].as_str().unwrap()).collect::<Vec<_>>(), ["a", "b"]);
        assert!(parse_list("nope").is_err());
        assert!(parse_list(r#"{"machines":3}"#).is_err());
    }

    #[test]
    fn older_core_follows_semver() {
        assert!(older_core("0.1.20", "0.1.21"));
        assert!(!older_core("0.1.21", "0.1.21"));
        assert!(!older_core("0.2.0", "0.1.30"));
        assert!(older_core("0.1.21-rc.1", "0.1.21"));
        assert!(!older_core("0.1.21", "0.1.21-rc.1"));
        assert!(older_core("0.1.21-rc.1", "0.1.21-rc.2"));
        assert!(older_core("0.1.21-rc.2", "0.1.21-rc.10"));
        assert!(older_core("0.1.21-1", "0.1.21-alpha"));
        assert!(older_core("0.1.21-rc", "0.1.21-rc.1"));
        assert!(!older_core("weird", "0.1.21"));
        assert!(!older_core("0.1.20+build", "0.1.20"));
    }

    #[test]
    fn an_update_says_the_version_and_a_failure_says_why() {
        let ok = outcome(Action::Update, "box", "", 0, r#"{"version":"0.1.22"}"#, "").unwrap();
        assert!(ok.contains("box") && ok.contains("0.1.22"));
        let err = outcome(Action::Rename, "box", "x", 1, "", r#"{"error":{"message":"name taken"}}"#).unwrap_err();
        assert_eq!(err, "name taken");
        assert_eq!(outcome(Action::Remove, "box", "", 2, "", "  boom \n").unwrap_err(), "boom");
        assert_eq!(outcome(Action::Remove, "box", "", 2, "", "").unwrap_err(), i18n::t("settings.it_didn_t_work"));
    }

    #[test]
    fn states_read_in_words() {
        assert_eq!(state_label("needs_login"), i18n::t("settings.needs_login"));
        assert_eq!(state_label("half-open_thing"), "half open thing");
    }
}
