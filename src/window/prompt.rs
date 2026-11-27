//! A text prompt, native: asks for a name and runs a command with it (`rename`, Settings > machines).
//! Enter saves, Esc cancels. A mod that opens an overlay named `prompt` with the same props gets this
//! dialog, unless the person keeps a mod folder of that name (see `open_overlay`).

use super::dialog::{self, Act, Dialog};
use super::native::z;
use super::*;
use crate::i18n;
use gpui_base::input::{InputEvent, InputState};

/// What the prompt asks and runs: `props` of the old `prompt` mod.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Ask {
    pub title: String,
    pub value: String,
    pub hint: String,
    /// The command it runs with `name: <text>` added to its args.
    pub command: (String, Value),
}

impl Ask {
    pub fn from_props(props: &Value) -> Option<Self> {
        let command = props.get("command")?;
        let text = |k: &str| props.get(k).and_then(Value::as_str).unwrap_or_default().to_owned();
        Some(Self {
            title: capitalize(&text("title")),
            value: text("value"),
            hint: text("hint"),
            command: (command.get("name")?.as_str()?.to_owned(), command.get("args").filter(|a| a.is_object()).cloned().unwrap_or_else(|| json!({}))),
        })
    }

    /// The command and args that run on save.
    pub fn run_with(&self, text: &str) -> (String, Value) {
        let mut args = self.command.1.clone();
        args["name"] = json!(text);
        (self.command.0.clone(), args)
    }
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |f| f.to_uppercase().chain(chars).collect())
}

pub(super) struct PromptView {
    app: WeakEntity<AppWindow>,
    id: String,
    ask: Ask,
    input: Entity<InputState>,
    focus: dialog::Focus,
}

impl PromptView {
    fn new(app: WeakEntity<AppWindow>, id: String, ask: Ask, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = dialog::input(&ask.value, "", window, cx, |this, ev, window, cx| {
            if matches!(ev, InputEvent::PressEnter { .. }) {
                this.save(window, cx);
            }
        });
        // The text it starts with is selected: typing replaces it, an arrow key keeps it.
        let all = 0..ask.value.len();
        input.update(cx, |s, cx| {
            s.focus(window, cx);
            s.set_selected_range(all, cx);
        });
        Self { app, id, ask, input, focus: dialog::Focus::new(cx) }
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.id.clone();
        let _ = self.app.update(cx, |app, cx| app.close_overlay(&id, window, cx));
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (name, args) = self.ask.run_with(&self.input.read(cx).value());
        let id = self.id.clone();
        let _ = self.app.update(cx, |app, cx| {
            app.close_overlay(&id, window, cx);
            app.run(&name, args, window, cx);
        });
    }
}

impl Render for PromptView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();
        let on = |save: bool| {
            let this = this.clone();
            move |w: &mut Window, cx: &mut App| this.update(cx, |v, cx| if save { v.save(w, cx) } else { v.close(w, cx) })
        };
        let body = div()
            .flex()
            .flex_col()
            .gap(z(8.0))
            .child(dialog::field(i18n::t("prompt.name"), &self.input, false, window, cx))
            .when(!self.ask.hint.is_empty(), |d| d.child(dialog::message(self.ask.hint.clone(), false)));
        Dialog::new(self.ask.title.clone(), &self.focus, on(false))
            .fields(1)
            .body(body)
            .cancel_label(i18n::t("prompt.cancel"))
            .action(Act::primary(i18n::t("prompt.save"), on(true)))
            .render("prompt")
    }
}

impl AppWindow {
    /// Opens the prompt under overlay `id` (a second open of the same id is the caller's to close first).
    pub(super) fn open_prompt(&mut self, id: String, ask: Ask, window: &mut Window, cx: &mut Context<Self>) {
        let app = cx.weak_entity();
        let view = cx.new(|cx| PromptView::new(app, id.clone(), ask, window, cx));
        self.push_dialog_overlay(id, dialog::NARROW + 48.0, view.into(), window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `super::*` brings gpui's own `test` macro: the plain one is the std attribute.
    use core::prelude::v1::test;

    #[test]
    fn props_of_the_old_mod_become_the_same_ask() {
        let ask = Ask::from_props(&json!({ "title": "rename tab", "value": "api", "hint": "Empty: its number", "command": { "name": "tab.rename", "args": { "id": "t1" } } })).unwrap();
        assert_eq!(ask.title, "Rename tab");
        assert_eq!(ask.run_with("web"), ("tab.rename".to_owned(), json!({ "id": "t1", "name": "web" })));
    }

    #[test]
    fn a_prompt_without_a_command_is_refused() {
        assert!(Ask::from_props(&json!({ "title": "x" })).is_none());
        assert!(Ask::from_props(&json!({ "command": {} })).is_none());
    }

    #[test]
    fn the_command_may_come_without_args() {
        let ask = Ask::from_props(&json!({ "command": { "name": "space.rename" } })).unwrap();
        assert_eq!(ask.run_with("x"), ("space.rename".to_owned(), json!({ "name": "x" })));
    }
}
