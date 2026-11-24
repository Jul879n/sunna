//! Settings > General: language, keeping the computer awake, resuming agents,
//! the folder search roots, and where worktrees go and how files open.

use super::*;
use controls::{button, row, segmented, switch};

/// Where new worktrees go when settings.json names no folder.
pub const DIRECTORY: &str = "~/.herdr/worktrees";

/// `worktrees.directory`, keeping the rest of `worktrees`.
pub fn set_directory(q: &mut Object, value: &str) {
    let mut w = q.get("worktrees").and_then(Value::as_object).cloned().unwrap_or_default();
    w.insert("directory".into(), json!(value));
    q.insert("worktrees".into(), Value::Object(w));
}

/// `folders.roots`, keeping the rest of `folders`; an empty list means home.
pub fn set_roots(q: &mut Object, roots: Vec<String>) {
    let mut f = q.get("folders").and_then(Value::as_object).cloned().unwrap_or_default();
    f.insert("roots".into(), json!(if roots.is_empty() { vec!["~".to_owned()] } else { roots }));
    q.insert("folders".into(), Value::Object(f));
}

pub fn roots(p: &Object) -> Vec<String> {
    p.get("folders")
        .and_then(|f| f.get("roots"))
        .and_then(Value::as_array)
        .map(|l| l.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
        .unwrap_or_else(|| vec!["~".to_owned()])
}

/// `menu.open_in` with `name` added once.
pub fn add_open_in(name: &str) {
    let mut list = open_in();
    if !list.iter().any(|n| n == name) {
        list.push(name.to_owned());
        mods::set_shared("menu.open_in".into(), json!(list));
    }
}

pub fn open_in() -> Vec<String> {
    store::get("menu.open_in").as_array().map(|l| l.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect()).unwrap_or_default()
}

pub fn directory(p: &Object) -> String {
    p.get("worktrees").and_then(|w| w.get("directory")).and_then(Value::as_str).unwrap_or(DIRECTORY).to_owned()
}

/// A removable item: its text and an x.
fn removable(id: String, text: String, remove: Act) -> Stateful<Div> {
    div()
        .id(SharedString::from(format!("{id}-row")))
        .flex()
        .flex_row()
        .items_center()
        .gap(settings::z(8.0))
        .child(div().text_size(settings::z(13.0)).text_color(c("muted")).child(text))
        .child(
            div()
                .id(SharedString::from(format!("{id}-x")))
                .size(settings::z(32.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(settings::z(6.0))
                .cursor_pointer()
                .hover(|s| s.bg(c("raised")))
                .tab_index(0)
                .focus_visible(controls::ring)
                .on_click(move |_, w, cx| remove(w, cx))
                .child(svg().data(include_bytes!("../../../defaults/mods/workspace-chrome/x.svg").as_slice()).size(settings::z(13.0)).text_color(c("muted"))),
        )
}

impl SettingsView {
    pub(super) fn general(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let p = self.person.clone();
        let language = p.get("language").and_then(Value::as_str).map(str::to_owned);
        let lang = segmented(
            "lang",
            vec![(None, i18n::t("settings.language_system")), (Some("es".to_owned()), i18n::t("settings.language_es")), (Some("en".to_owned()), i18n::t("settings.language_en"))],
            &language,
            {
                let view = cx.entity().downgrade();
                move |value: Option<String>, w, cx| {
                    let _ = view.update(cx, |this, cx| this.change(w, cx, |q| match &value {
                        Some(v) => drop(q.insert("language".into(), json!(v))),
                        None => drop(q.remove("language")),
                    }));
                }
            },
        );
        let resume = p.get("resume_agents") != Some(&json!(false));
        let resume_switch = switch("st-resume", resume, self.act(cx, move |this, w, cx| this.change(w, cx, |q| {
            if resume { q.insert("resume_agents".into(), json!(false)); } else { q.remove("resume_agents"); }
        })));
        let roots_now = roots(&p);
        let mut roots_col = div().flex().flex_col().items_end().gap(settings::z(6.0));
        for (i, r) in roots_now.iter().enumerate() {
            let list = roots_now.clone();
            roots_col = roots_col.child(removable(format!("st-root-{i}"), r.clone(), self.act(cx, move |this, w, cx| {
                let next: Vec<String> = list.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, v)| v.clone()).collect();
                this.change(w, cx, |q| set_roots(q, next));
            })));
        }
        let list = roots_now.clone();
        roots_col = roots_col.child(button("st-root-add", i18n::t("settings.add_folder"), {
            let view = cx.entity().downgrade();
            controls::act(move |w, cx| {
                let list = list.clone();
                let _ = view.update(cx, |this, cx| this.pick_folder(i18n::t("settings.add_folder_prompt"), w, cx, move |this, folder, w, cx| {
                    if !list.contains(&folder) {
                        let mut next = list;
                        next.push(folder);
                        this.change(w, cx, |q| set_roots(q, next));
                    }
                }));
            })
        }));
        let dir = directory(&p);
        let dir_field = self.text_field(Field::Directory, &dir, 240.0, window, cx);
        let choose = button("st-wt-dir-pick", i18n::t("settings.orca_choose"), {
            let view = cx.entity().downgrade();
            controls::act(move |w, cx| {
                let _ = view.update(cx, |this, cx| this.pick_folder(i18n::t("settings.orca_worktrees_dir"), w, cx, |this, folder, w, cx| this.change(w, cx, |q| set_directory(q, &folder))));
            })
        });
        let apps = open_in();
        let mut apps_col = div().flex().flex_col().items_end().gap(settings::z(6.0));
        for (i, name) in apps.iter().enumerate() {
            let all = apps.clone();
            apps_col = apps_col.child(removable(format!("st-app-{i}"), name.clone(), self.act(cx, move |_, _, _| {
                let next: Vec<&String> = all.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, v)| v).collect();
                mods::set_shared("menu.open_in".into(), json!(next));
            })));
        }
        let open_in_field = self.text_field(Field::OpenIn, "", 200.0, window, cx);
        let add = button("st-app-add", i18n::t("settings.orca_add"), self.act(cx, |this, w, cx| {
            if let Some(f) = this.fields.iter_mut().find(|f| f.id == Field::OpenIn) {
                let typed = f.input.read(cx).value().to_string();
                f.text = Some(typed);
            }
            this.commit(Field::OpenIn, w, cx);
        }));
        apps_col = apps_col.child(div().flex().flex_row().items_center().gap(settings::z(8.0)).child(open_in_field).child(add));

        let mut page = div()
            .w_full()
            .child(controls::heading(i18n::t("settings.general")))
            .child(row(i18n::t("settings.language"), Some(i18n::t("settings.language_about")), lang));
        if !cfg!(windows) {
            let now = serde_json::to_value(settings::get().keep_awake).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_else(|| "off".into());
            let choices = vec![("off".to_owned(), i18n::t("awake.menu.off")), ("auto".to_owned(), i18n::t("awake.menu.auto")), ("on".to_owned(), i18n::t("awake.menu.on"))];
            let view = cx.entity().downgrade();
            let awake = segmented("awake", choices, &now, move |mode: String, w, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.run("app.keep_awake", json!({ "mode": mode }), w, cx);
                    cx.notify();
                });
            });
            page = page.child(row(i18n::t("settings.keep_awake"), Some(i18n::t("settings.keep_awake_about")), awake));
        }
        page.child(row(i18n::t("settings.resume_agents"), Some(i18n::t("settings.resume_agents_about")), resume_switch))
            .child(row(i18n::t("settings.folder_roots"), Some(i18n::t("settings.folder_roots_about")), roots_col))
            .child(row(
                i18n::t("settings.orca_worktrees_dir"),
                Some(i18n::t("settings.orca_worktrees_dir_about")),
                div().flex().flex_row().items_center().gap(settings::z(8.0)).child(dir_field).child(choose),
            ))
            .child(row(i18n::t("settings.orca_nest"), Some(i18n::t("settings.orca_nest_about")), self.shared_switch(cx, "st-wt-nest", "worktree.nest")))
            .child(row(i18n::t("settings.orca_external"), Some(i18n::t("settings.orca_external_about")), self.shared_switch(cx, "st-wt-external", "worktree.external")))
            .child(row(
                i18n::t("settings.orca_files_open"),
                Some(i18n::t("settings.orca_files_open_about")),
                self.shared_pick(cx, "files-open", "files.open", &[("click", "settings.orca_open_click"), ("double", "settings.orca_open_double")]),
            ))
            .child(row(i18n::t("settings.orca_open_in"), Some(i18n::t("settings.orca_open_in_about")), apps_col))
    }
}

#[cfg(test)]
mod tests {
    use super::{DIRECTORY, Object, directory, roots, set_directory, set_roots};
    use serde_json::Value;

    fn obj(text: &str) -> Object {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn directory_keeps_the_rest_of_worktrees() {
        let mut q = obj(r#"{"worktrees": {"other": 1}, "zoom": 1.1}"#);
        set_directory(&mut q, "/w");
        assert_eq!(Value::Object(q), serde_json::from_str::<Value>(r#"{"worktrees":{"other":1,"directory":"/w"},"zoom":1.1}"#).unwrap());
    }

    #[test]
    fn roots_default_to_home_and_never_end_empty() {
        assert_eq!(roots(&obj("{}")), vec!["~"]);
        let mut q = obj(r#"{"folders": {"keep": true, "roots": ["/a"]}}"#);
        set_roots(&mut q, vec![]);
        assert_eq!(Value::Object(q), serde_json::from_str::<Value>(r#"{"folders":{"keep":true,"roots":["~"]}}"#).unwrap());
    }

    #[test]
    fn directory_defaults() {
        assert_eq!(directory(&obj("{}")), DIRECTORY);
        assert_eq!(directory(&obj(r#"{"worktrees": {"directory": "/x"}}"#)), "/x");
    }
}
