//! Settings > Keyboard: Option as Alt (macOS) and every shortcut of keys.json:
//! change one (press the new keys), remove one, or restore the shipped ones.

use super::*;
use controls::{button, card, icon_button, radio};
use gpui::Keystroke;

const X: &[u8] = include_bytes!("../../../defaults/mods/workspace-chrome/x.svg");

/// The shortcut being changed: its section and the keys it has now.
#[derive(Clone, Debug, PartialEq)]
pub struct Capture {
    pub section: String,
    pub old: String,
}

#[derive(Default)]
pub struct State {
    pub capture: Option<Capture>,
    /// "Restore" was pressed once and waits for its confirmation.
    pub restore: bool,
}

/// The chord in the spelling keys.json uses (modifier order as GPUI's `parse`
/// round-trips it; the platform key is `cmd` everywhere).
pub fn chord(k: &Keystroke) -> String {
    let m = &k.modifiers;
    let mut out = String::new();
    for (on, name) in [(m.function, "fn-"), (m.control, "ctrl-"), (m.alt, "alt-"), (m.platform, "cmd-"), (m.shift, "shift-")] {
        if on {
            out.push_str(name);
        }
    }
    out + &k.key
}

/// A bare modifier press doesn't make a chord.
pub fn is_modifier(key: &str) -> bool {
    matches!(key, "shift" | "control" | "alt" | "cmd" | "platform" | "function")
}

/// Sections in the order shown: global, terminal, then the rest by name.
pub fn order(mut sections: Vec<String>) -> Vec<String> {
    let rank = |n: &str| match n {
        "global" => 0,
        "terminal" => 1,
        _ => 2,
    };
    sections.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a.cmp(b)));
    sections.dedup();
    sections
}

/// What an action is called: the command of an object binding, else the text.
pub fn action_name(action: &str) -> String {
    match serde_json::from_str::<Value>(action) {
        Ok(Value::Object(o)) => o.get("command").and_then(Value::as_str).unwrap_or(action).to_owned(),
        _ => action.to_owned(),
    }
}

/// Moves a binding of `section` from the keys `old` to `new`, keeping its action as it is.
/// False when the file has no such binding.
pub fn move_binding(o: &mut Object, section: &str, old: &str, new: &str) -> bool {
    let Some(Value::Object(s)) = o.get_mut(section) else { return false };
    let Some(action) = s.remove(old) else { return false };
    s.insert(new.to_owned(), action);
    true
}

pub fn remove_binding(o: &mut Object, section: &str, old: &str) {
    if let Some(Value::Object(s)) = o.get_mut(section) {
        s.remove(old);
    }
}

/// The bindings keys.json holds now (those the person can change).
fn file_bindings() -> Object {
    std::fs::read_to_string(crate::keys::path()).ok().and_then(|t| serde_json::from_str::<Object>(&t).ok()).unwrap_or_default()
}

fn held(file: &Object, section: &str, keys: &str) -> bool {
    file.get(section).and_then(|s| s.get(keys)).is_some()
}

impl SettingsView {
    fn edit_keys(&mut self, window: &mut Window, cx: &mut Context<Self>, change: impl FnOnce(&mut Object)) {
        if let Err(failure) = store::edit_file("keys.json", crate::keys::DEFAULTS, change) {
            self.toast(failure, window, cx);
        }
        cx.notify();
    }

    /// The chord pressed while a shortcut is being changed.
    pub(super) fn capture_key(&mut self, e: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(c) = self.keys.capture.clone() else { return };
        let key = e.keystroke.key.as_str();
        if key == "escape" {
            self.keys.capture = None;
            return;
        }
        if is_modifier(key) {
            return;
        }
        let new = chord(&e.keystroke);
        if new == c.old {
            self.keys.capture = None;
            return;
        }
        if let Some((_, _, action)) = crate::keys::listing().into_iter().find(|(s, k, _)| *s == c.section && *k == new) {
            let n = json!({ "level": "error", "title": i18n::tf("settings.keys_taken", &[("action", &action_name(&action))]) });
            self.notify(n, window, cx);
            return;
        }
        self.keys.capture = None;
        self.edit_keys(window, cx, |o| {
            let _ = move_binding(o, &c.section, &c.old, &new);
        });
    }

    /// Puts the shipped keys.json back, keeping the person's as `keys.json.restored-<time>`.
    fn restore_keys(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let path = crate::keys::path();
        if path.exists() {
            let _ = std::fs::rename(&path, aside(&path, "restored"));
        }
        if let Err(e) = crate::persist::write_atomic(&path, crate::keys::DEFAULTS) {
            let failure = store::Failure { title: i18n::tf("settings.file_not_saved", &[("name", "keys.json")]), body: Some(e.to_string()) };
            self.toast(failure, window, cx);
        }
        self.keys.restore = false;
        cx.notify();
    }

    pub(super) fn keyboard(&mut self, cx: &mut Context<Self>) -> Div {
        let mut page = div().w_full();
        if cfg!(target_os = "macos") {
            let now = self.person.get("option_as_alt").and_then(Value::as_str).unwrap_or("auto").to_owned();
            let choices = [
                ("auto", i18n::t("settings.automatic_alt_on_us_keyboards")),
                ("on", "Alt".to_owned()),
                ("off", i18n::t("settings.types_characters")),
                ("left", i18n::t("settings.left_option_is_alt")),
                ("right", i18n::t("settings.right_option_is_alt")),
            ];
            let mut list = div().w_full().flex().flex_col().gap(settings::z(8.0)).mb(settings::z(18.0));
            for (value, label) in choices {
                list = list.child(radio(format!("st-option-{value}"), label, now == value, self.act(cx, move |this, w, cx| this.change(w, cx, |q| drop(q.insert("option_as_alt".into(), json!(value)))))));
            }
            page = page.child(controls::heading(i18n::t("settings.option_key"))).child(controls::note(i18n::t("settings.use_option_for_terminal_shortcuts"))).child(list);
        }
        page = page.child(controls::heading(i18n::t("settings.shortcuts"))).child(controls::note(i18n::t("settings.keys_note")));
        let confirm = self.keys.restore;
        page = page.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(settings::z(8.0))
                .pb(settings::z(6.0))
                .child(button(
                    "st-keys-restore",
                    if confirm { i18n::t("rs.settings.keys_restore_confirm") } else { i18n::t("rs.settings.keys_restore") },
                    self.act(cx, |this, w, cx| if this.keys.restore { this.restore_keys(w, cx) } else { this.keys.restore = true }),
                ))
                .when(confirm, |d| d.child(button("st-keys-restore-cancel", i18n::t("settings.cancel"), self.act(cx, |this, _, _| this.keys.restore = false))))
        );
        page = page.child(div().pb(settings::z(12.0)).text_size(settings::z(12.5)).line_height(settings::z(18.0)).text_color(c("muted")).child(i18n::t("rs.settings.keys_restore_about")));
        let file = file_bindings();
        let listing = crate::keys::listing();
        let sections = order(listing.iter().map(|(s, _, _)| s.clone()).collect());
        let capture = self.keys.capture.clone();
        for section in sections {
            let mut group = card();
            for (i, (_, keys, action)) in listing.iter().filter(|(s, _, _)| *s == section).enumerate() {
                let editable = held(&file, &section, keys);
                let asking = capture.as_ref().is_some_and(|c| c.section == section && c.old == *keys);
                let (sec, old) = (section.clone(), keys.clone());
                let chip = div()
                    .flex_none()
                    .min_w(settings::z(96.0))
                    .flex()
                    .justify_center()
                    .px(settings::z(8.0))
                    .py(settings::z(4.0))
                    .rounded(settings::z(6.0))
                    .border_1()
                    .border_color(if asking { c("accent") } else { c("border") })
                    .bg(if asking { c("selected") } else { c("background") })
                    .text_size(settings::z(12.5))
                    .text_color(if asking { c("foreground") } else { c("muted") })
                    .child(if asking { i18n::t("settings.keys_press") } else { keys.clone() });
                let mut line = div()
                    .id(SharedString::from(format!("st-key-{section}-{i}")))
                    .w_full()
                    .min_h(settings::z(40.0))
                    .px(settings::z(10.0))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(settings::z(10.0))
                    .child(div().flex_1().min_w_0().truncate().text_size(settings::z(13.0)).text_color(c("foreground")).child(action_name(action)))
                    .child(chip);
                if editable {
                    line = line
                        .child(button(
                            format!("st-key-change-{section}-{i}"),
                            i18n::t("settings.keys_change"),
                            self.act(cx, move |this, w, cx| {
                                let same = this.keys.capture.as_ref().is_some_and(|c| c.section == sec && c.old == old);
                                this.keys.capture = if same { None } else { Some(Capture { section: sec.clone(), old: old.clone() }) };
                                this.focus.focus(w, cx);
                            }),
                        ))
                        .child(icon_button(format!("st-key-del-{section}-{i}"), X, {
                            let (sec, old) = (section.clone(), keys.clone());
                            self.act(cx, move |this, w, cx| {
                                let (sec, old) = (sec.clone(), old.clone());
                                this.edit_keys(w, cx, move |o| remove_binding(o, &sec, &old))
                            })
                        }));
                } else {
                    line = line.child(div().w(settings::z(76.0 + 32.0 + 8.0)).flex_none());
                }
                group = group.child(line);
            }
            page = page.child(div().pt(settings::z(8.0)).pb(settings::z(6.0)).text_size(settings::z(12.0)).font_weight(FontWeight::SEMIBOLD).text_color(c("muted")).child(section.clone())).child(group);
        }
        page
    }
}

#[cfg(test)]
mod tests {
    use super::{Object, action_name, chord, is_modifier, move_binding, order, remove_binding};
    use gpui::Keystroke;
    use serde_json::Value;

    fn obj(text: &str) -> Object {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn the_chord_is_spelled_as_keys_json_writes_it() {
        let k = Keystroke::parse("shift-cmd-p").unwrap();
        assert_eq!(chord(&k), "cmd-shift-p");
        assert_eq!(chord(&Keystroke::parse("ctrl-alt-left").unwrap()), "ctrl-alt-left");
        assert_eq!(chord(&Keystroke::parse("escape").unwrap()), "escape");
    }

    #[test]
    fn a_bare_modifier_is_no_chord() {
        for k in ["shift", "control", "alt", "cmd", "platform", "function"] {
            assert!(is_modifier(k));
        }
        assert!(!is_modifier("p"));
    }

    #[test]
    fn sections_go_global_terminal_then_by_name() {
        let got = order(["mod:b", "terminal", "global", "layout:a", "global"].map(String::from).to_vec());
        assert_eq!(got, ["global", "terminal", "layout:a", "mod:b"]);
    }

    #[test]
    fn a_binding_moves_with_its_action_and_an_object_stays_an_object() {
        let mut o = obj(r#"{"global": {"cmd-k": "x.y", "cmd-j": {"command": "layout.resize", "args": {"px": 1}}}}"#);
        assert!(move_binding(&mut o, "global", "cmd-j", "cmd-l"));
        assert_eq!(o["global"]["cmd-l"]["command"], "layout.resize");
        assert!(o["global"].get("cmd-j").is_none());
        assert!(!move_binding(&mut o, "global", "cmd-nope", "cmd-m"));
        assert!(!move_binding(&mut o, "terminal", "cmd-k", "cmd-m"));
        remove_binding(&mut o, "global", "cmd-k");
        assert_eq!(Value::Object(o), serde_json::from_str::<Value>(r#"{"global": {"cmd-l": {"command": "layout.resize", "args": {"px": 1}}}}"#).unwrap());
    }

    #[test]
    fn an_object_action_is_named_by_its_command() {
        assert_eq!(action_name(r#"{"command":"layout.resize","args":{}}"#), "layout.resize");
        assert_eq!(action_name("tab.new"), "tab.new");
        assert_eq!(action_name("send:\u{1b}b"), "send:\u{1b}b");
    }
}
