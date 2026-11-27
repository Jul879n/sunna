//! Quick commands (Orca's idea): named commands the person keeps and sends to the focused terminal
//! with one action, from Settings > Terminal, the command palette (⌘K) or `terminal.quick {name}`.
//!
//! They live in `shared("quick_commands")`: a list of `{ "name", "command", "enter" }` (`enter`,
//! default true, presses Enter after the text; false only types it, to finish it by hand). The
//! text goes in as a paste, so a command of several lines reaches the shell as one block.
//! Commands: `terminal.quick`, `quick.set`, `quick.edit`, `quick.delete`.

use super::dialog::{self, Act, Dialog};
use super::*;
use crate::i18n;
use gpui_base::input::{InputEvent, InputState, Textarea, TextareaState};

pub(super) const KEY: &str = "quick_commands";
pub(super) const ID: &str = "quick-command";
const WIDTH: f32 = 520.0;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Quick {
    pub name: String,
    pub command: String,
    pub enter: bool,
}

/// Why a quick command cannot be saved as written.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Problem {
    NoName,
    NoCommand,
    Taken,
}

impl Problem {
    pub fn text(self) -> String {
        i18n::t(match self {
            Problem::NoName => "quick.error_no_name",
            Problem::NoCommand => "quick.error_no_command",
            Problem::Taken => "quick.error_taken",
        })
    }
}

/// The list as kept: entries without a name or a command are dropped, a repeated name (ignoring case) too.
pub(super) fn parse(v: &Value) -> Vec<Quick> {
    let mut out: Vec<Quick> = Vec::new();
    for e in v.as_array().into_iter().flatten() {
        let name = e.get("name").and_then(Value::as_str).unwrap_or_default().trim().to_owned();
        let command = e.get("command").and_then(Value::as_str).unwrap_or_default().to_owned();
        let enter = e.get("enter").and_then(Value::as_bool).unwrap_or(true);
        if name.is_empty() || command.trim().is_empty() || find(&out, &name).is_some() {
            continue;
        }
        out.push(Quick { name, command, enter });
    }
    out
}

pub(super) fn to_json(list: &[Quick]) -> Value {
    Value::Array(list.iter().map(|q| json!({ "name": q.name, "command": q.command, "enter": q.enter })).collect())
}

pub(super) fn load() -> Vec<Quick> {
    mods::shared(KEY).map(|v| parse(&v)).unwrap_or_default()
}

fn save(list: &[Quick]) {
    mods::set_shared(KEY.into(), to_json(list));
}

pub(super) fn find<'a>(list: &'a [Quick], name: &str) -> Option<&'a Quick> {
    let name = name.trim().to_lowercase();
    list.iter().find(|q| q.name.to_lowercase() == name)
}

/// Can `name` and `command` be saved, when `editing` is the name being edited (it may keep its own name)?
pub(super) fn check(name: &str, command: &str, list: &[Quick], editing: Option<&str>) -> Result<(), Problem> {
    if name.trim().is_empty() {
        return Err(Problem::NoName);
    }
    if command.trim().is_empty() {
        return Err(Problem::NoCommand);
    }
    let same = |a: &str, b: &str| a.trim().to_lowercase() == b.trim().to_lowercase();
    if list.iter().any(|q| same(&q.name, name) && !editing.is_some_and(|e| same(e, &q.name))) {
        return Err(Problem::Taken);
    }
    Ok(())
}

/// Replaces the entry `old` in place (or adds at the end).
pub(super) fn upsert(list: &mut Vec<Quick>, old: Option<&str>, q: Quick) {
    match old.and_then(|o| list.iter().position(|e| e.name.to_lowercase() == o.trim().to_lowercase())) {
        Some(i) => list[i] = q,
        None => list.push(q),
    }
}

pub(super) fn remove(list: &mut Vec<Quick>, name: &str) -> bool {
    let before = list.len();
    let name = name.trim().to_lowercase();
    list.retain(|q| q.name.to_lowercase() != name);
    list.len() != before
}

/// A one-line preview of a command for lists: its first line, shortened.
pub(super) fn preview(command: &str) -> String {
    let first = command.lines().find(|l| !l.trim().is_empty()).unwrap_or_default().trim();
    let more = command.lines().filter(|l| !l.trim().is_empty()).count() > 1;
    let mut text: String = first.chars().take(80).collect();
    if first.chars().count() > 80 || more {
        text.push('…');
    }
    text
}

pub(super) struct QuickView {
    app: WeakEntity<AppWindow>,
    old: Option<String>,
    name: Entity<InputState>,
    command: Entity<TextareaState>,
    enter: bool,
    focus: dialog::Focus,
    problem: Option<Problem>,
}

impl QuickView {
    fn new(app: WeakEntity<AppWindow>, current: Option<Quick>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let old = current.as_ref().map(|q| q.name.clone());
        let (name_text, command_text, enter) = current.map_or((String::new(), String::new(), true), |q| (q.name, q.command, q.enter));
        let name = dialog::input(&name_text, &i18n::t("quick.name_placeholder"), window, cx, |this: &mut Self, ev, _, cx| {
            if matches!(ev, InputEvent::Change) {
                this.problem = None;
                cx.notify();
            }
        });
        let area = cx.new(|cx| TextareaState::new(window, cx).default_value(command_text).placeholder(i18n::t("quick.command_placeholder")).rows(4));
        area.update(cx, |s, _| s.set_editor_style(dialog::input_style()));
        cx.subscribe_in(&area, window, |this, area, ev: &InputEvent, _, cx| match ev {
            InputEvent::Change => {
                this.problem = None;
                dialog::mark_typed(area.entity_id(), !area.read(cx).value().is_empty());
                cx.notify();
            }
            InputEvent::Focus | InputEvent::Blur => cx.notify(),
            _ => {}
        })
        .detach();
        name.update(cx, |s, cx| s.focus(window, cx));
        Self { app, old, name, command: area, enter, focus: dialog::Focus::new(cx), problem: None }
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.app.update(cx, |app, cx| app.close_overlay(ID, window, cx));
    }

    fn texts(&self, cx: &App) -> (String, String) {
        (self.name.read(cx).value().trim().to_owned(), self.command.read(cx).value().to_string())
    }

    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (name, command) = self.texts(cx);
        match check(&name, &command, &load(), self.old.as_deref()) {
            Err(p) => {
                self.problem = Some(p);
                cx.notify();
            }
            Ok(()) => {
                let mut list = load();
                upsert(&mut list, self.old.as_deref(), Quick { name, command: command.trim_end().to_owned(), enter: self.enter });
                save(&list);
                self.close(window, cx);
            }
        }
    }
}

impl Render for QuickView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();
        let (name, command) = self.texts(cx);
        let ok = check(&name, &command, &load(), self.old.as_deref()).is_ok();
        let area_focused = self.command.read(cx).focus_handle(cx).is_focused(window);
        let flip = {
            let this = this.clone();
            move |_: &mut Window, cx: &mut App| this.update(cx, |v, cx| {
                v.enter = !v.enter;
                cx.notify();
            })
        };
        let body = div()
            .flex()
            .flex_col()
            .gap(z(10.0))
            .child(dialog::field(i18n::t("quick.name"), &self.name, matches!(self.problem, Some(Problem::NoName | Problem::Taken)), window, cx))
            .child(dialog::label(i18n::t("quick.command")))
            .child(dialog::field_box(area_focused, matches!(self.problem, Some(Problem::NoCommand))).h(z(104.0)).items_start().py(z(6.0)).child(div().flex_1().h_full().child(Textarea::new(&self.command))))
            .child(dialog::toggle("quick-enter", i18n::t("quick.press_enter"), self.enter, flip))
            .when_some(self.problem, |d, p| d.child(dialog::message(p.text(), true)));
        let title = i18n::t(if self.old.is_some() { "quick.edit_title" } else { "quick.new_title" });
        Dialog::new(title, &self.focus, dialog::run(&this, |v, w, cx| v.close(w, cx)))
            .width(WIDTH)
            .fields(2)
            .body(body)
            .cancel_label(i18n::t("feedback.cancel"))
            .action(Act::primary(i18n::t("quick.save"), dialog::run(&this, |v, w, cx| v.submit(w, cx))).enabled(ok))
            .render("quick")
    }
}

fn z(n: f32) -> Pixels {
    settings::z(n)
}

impl AppWindow {
    /// The new / edit dialog (`quick.edit {name?}`).
    pub(super) fn open_quick_editor(&mut self, name: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        self.close_overlay(ID, window, cx);
        let current = name.and_then(|n| find(&load(), n).cloned());
        let app = cx.weak_entity();
        let view = cx.new(|cx| QuickView::new(app, current, window, cx));
        self.push_dialog_overlay(ID.into(), WIDTH + 48.0, view.into(), window, cx);
    }

    /// Types a quick command into the focused terminal. Answers an error text when it cannot.
    pub(super) fn run_quick(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let list = load();
        let q = find(&list, name).ok_or_else(|| format!("no quick command {name:?}"))?.clone();
        let t = self.focused_terminal(window, cx).ok_or("no focused terminal to type into")?;
        t.view.update(cx, |v, cx| v.type_command(&q.command, q.enter, cx));
        Ok(())
    }

    pub(super) fn quick_command(&mut self, name: &str, a: &Value, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let text = |k: &str| a.get(k).and_then(Value::as_str);
        match name {
            "terminal.quick" => self.run_quick(text("name").unwrap_or_default(), window, cx),
            "quick.edit" => {
                self.open_quick_editor(text("name"), window, cx);
                Ok(())
            }
            "quick.set" => {
                let (n, command) = (text("name").unwrap_or_default(), text("command").unwrap_or_default());
                let mut list = load();
                check(n, command, &list, text("old").or(Some(n)).filter(|o| find(&list, o).is_some())).map_err(|p| p.text())?;
                let enter = a.get("enter").and_then(Value::as_bool).unwrap_or(true);
                upsert(&mut list, text("old").or(Some(n)), Quick { name: n.trim().to_owned(), command: command.trim_end().to_owned(), enter });
                save(&list);
                Ok(())
            }
            _ => {
                // quick.delete
                let n = text("name").unwrap_or_default();
                let mut list = load();
                if find(&list, n).is_none() {
                    return Err(format!("no quick command {n:?}"));
                }
                if a.get("confirmed").and_then(Value::as_bool) != Some(true) {
                    let ask = confirm::Ask::new(i18n::tf("quick.delete_title", &[("name", n)]), i18n::t("quick.delete_detail"), "quick.delete", json!({ "name": n, "confirmed": true }));
                    self.open_confirm(ask, window, cx);
                    return Ok(());
                }
                remove(&mut list, n);
                save(&list);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;

    fn q(name: &str, command: &str) -> Quick {
        Quick { name: name.into(), command: command.into(), enter: true }
    }

    #[test]
    fn the_kept_list_drops_entries_without_name_or_command_and_repeated_names() {
        let v = json!([
            { "name": "test", "command": "cargo test" },
            { "name": " ", "command": "x" },
            { "name": "empty", "command": "  " },
            { "name": "TEST", "command": "other" },
            { "name": "dev", "command": "bun dev", "enter": false },
            "junk",
        ]);
        assert_eq!(parse(&v), vec![q("test", "cargo test"), Quick { name: "dev".into(), command: "bun dev".into(), enter: false }]);
        assert!(parse(&Value::Null).is_empty());
    }

    #[test]
    fn the_list_round_trips_through_json() {
        let list = vec![q("a", "ls"), Quick { name: "b".into(), command: "git status".into(), enter: false }];
        assert_eq!(parse(&to_json(&list)), list);
    }

    #[test]
    fn names_are_found_ignoring_case_and_spaces_around() {
        let list = vec![q("Build", "make")];
        assert!(find(&list, " build ").is_some());
        assert!(find(&list, "test").is_none());
    }

    #[test]
    fn a_quick_command_needs_a_name_a_command_and_a_free_name() {
        let list = vec![q("test", "cargo test")];
        assert_eq!(check("", "ls", &list, None), Err(Problem::NoName));
        assert_eq!(check("x", "  ", &list, None), Err(Problem::NoCommand));
        assert_eq!(check("Test", "ls", &list, None), Err(Problem::Taken));
        assert_eq!(check("test", "cargo test -q", &list, Some("test")), Ok(()));
        assert_eq!(check("other", "ls", &list, Some("test")), Ok(()));
        assert_eq!(check("test", "ls", &[q("test", "a"), q("other", "b")], Some("other")), Err(Problem::Taken));
    }

    #[test]
    fn saving_replaces_in_place_when_editing_and_appends_when_new() {
        let mut list = vec![q("a", "1"), q("b", "2")];
        upsert(&mut list, Some("a"), q("c", "3"));
        assert_eq!(list, vec![q("c", "3"), q("b", "2")]);
        upsert(&mut list, None, q("d", "4"));
        assert_eq!(list.len(), 3);
        assert_eq!(list[2].name, "d");
    }

    #[test]
    fn removing_says_whether_something_went() {
        let mut list = vec![q("a", "1")];
        assert!(!remove(&mut list, "zzz"));
        assert!(remove(&mut list, "A"));
        assert!(list.is_empty());
    }

    #[test]
    fn the_preview_is_the_first_line_shortened() {
        assert_eq!(preview("ls -la"), "ls -la");
        assert_eq!(preview("cd x\nmake"), "cd x…");
        assert!(preview(&"a".repeat(200)).ends_with('…'));
        assert_eq!(preview("\n\n"), "");
    }
}
