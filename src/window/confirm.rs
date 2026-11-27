//! The confirmation dialog, native: asks before something that can't be undone
//! ("Close space?", "Unsaved changes in a.rs"). Enter runs the main action,
//! Esc or Cancel closes. Closing a space, tab or pane, quitting, discarding
//! unsaved changes and reloading a file all ask through it.
//!
//! The `confirm` mod it replaced is still answered: a mod that opens an overlay
//! named `confirm` with the same props gets this dialog (`Ask::from_props`),
//! unless the person has a mod folder of that name of their own.

use super::*;
use crate::i18n;

/// What the dialog says and runs.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Ask {
    pub title: String,
    pub detail: String,
    /// The confirm button's text (default "Confirm").
    pub confirm_label: Option<String>,
    /// What the confirm button runs: a command and its args.
    pub command: (String, Value),
    /// A third, primary action (the editor's "Save and close"): its label and command.
    /// Enter then runs it, the safe choice, and the confirm button turns secondary.
    pub extra: Option<(String, (String, Value))>,
    pub width: f32,
    /// The confirm button destroys something (a tinted red button; the keyboard starts on Cancel).
    pub danger: bool,
}

impl Ask {
    pub fn new(title: impl Into<String>, detail: impl Into<String>, command: &str, args: Value) -> Self {
        Self { title: title.into(), detail: detail.into(), confirm_label: None, command: (command.to_owned(), args), extra: None, width: dialog::NARROW, danger: true }
    }

    /// The same ask the `confirm` mod took as props: { title, detail, command: { name, args },
    /// confirm_label, extra: { label, command } }; `width` comes from the overlay spec.
    pub fn from_props(props: &Value, width: Option<f32>) -> Option<Self> {
        let command = |v: &Value| Some((v.get("name")?.as_str()?.to_owned(), v.get("args").filter(|a| a.is_object()).cloned().unwrap_or_else(|| json!({}))));
        let extra = props.get("extra").and_then(|e| Some((e.get("label")?.as_str()?.to_owned(), command(e.get("command")?)?)));
        Some(Self {
            title: props.get("title").and_then(Value::as_str).map_or_else(|| i18n::t("rs.confirm.are_you_sure"), str::to_owned),
            detail: props.get("detail").and_then(Value::as_str).unwrap_or_default().to_owned(),
            confirm_label: props.get("confirm_label").and_then(Value::as_str).map(str::to_owned),
            command: command(props.get("command")?)?,
            extra,
            width: width.unwrap_or(dialog::NARROW),
            danger: props.get("danger").and_then(Value::as_bool).unwrap_or(true),
        })
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Choice {
    Confirm,
    Extra,
}

pub(super) struct ConfirmView {
    app: WeakEntity<AppWindow>,
    id: String,
    ask: Ask,
    focus: dialog::Focus,
}

impl ConfirmView {
    fn new(app: WeakEntity<AppWindow>, id: String, ask: Ask, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = dialog::Focus::new(cx);
        let tones = Self::tones(&ask);
        focus.put(dialog::initial_stop(0, &tones), tones.len(), window, cx);
        Self { app, id, ask, focus }
    }

    /// The actions after Cancel: the confirm button (red when it destroys), then the extra primary one.
    fn tones(ask: &Ask) -> Vec<dialog::Tone> {
        let confirm = match (ask.danger, ask.extra.is_some()) {
            (true, _) => dialog::Tone::Danger,
            (false, true) => dialog::Tone::Neutral,
            (false, false) => dialog::Tone::Primary,
        };
        std::iter::once(confirm).chain(ask.extra.as_ref().map(|_| dialog::Tone::Primary)).collect()
    }

    fn close(&self, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.id.clone();
        let _ = self.app.update(cx, |app, cx| app.close_overlay(&id, window, cx));
    }

    /// Closes the dialog and runs what was chosen.
    fn choose(&self, which: Choice, window: &mut Window, cx: &mut Context<Self>) {
        let (name, args) = match (which, &self.ask.extra) {
            (Choice::Extra, Some((_, command))) => command.clone(),
            _ => self.ask.command.clone(),
        };
        let id = self.id.clone();
        let _ = self.app.update(cx, |app, cx| {
            app.close_overlay(&id, window, cx);
            app.run(&name, args, window, cx);
        });
    }
}

impl Render for ConfirmView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();
        let act = |which: Option<Choice>| {
            let this = this.clone();
            move |w: &mut Window, cx: &mut App| this.update(cx, |v, cx| match which {
                Some(which) => v.choose(which, w, cx),
                None => v.close(w, cx),
            })
        };
        let tones = Self::tones(&self.ask);
        let confirm_label = self.ask.confirm_label.clone().unwrap_or_else(|| i18n::t("rs.confirm.confirm"));
        let mut d = dialog::Dialog::new(self.ask.title.clone(), &self.focus, act(None))
            .description(self.ask.detail.clone())
            .action(dialog::Act::new(confirm_label, tones[0], act(Some(Choice::Confirm))));
        if let Some((label, _)) = &self.ask.extra {
            d = d.action(dialog::Act::new(label.clone(), dialog::Tone::Primary, act(Some(Choice::Extra))));
        }
        d.render("confirm")
    }
}

impl AppWindow {
    /// Opens the confirmation dialog.
    pub(super) fn open_confirm(&mut self, ask: Ask, window: &mut Window, cx: &mut Context<Self>) {
        let id = mods::new_overlay_id();
        let app = cx.weak_entity();
        let width = ask.width + 48.0;
        let view = cx.new(|cx| ConfirmView::new(app, id.clone(), ask, window, cx));
        self.push_dialog_overlay(id, width, view.into(), window, cx);
    }

    /// A native dialog (see `dialog.rs`): the card draws its own frame, so the overlay is bare and
    /// `width` already includes the room for the shadow.
    pub(super) fn push_dialog_overlay(&mut self, id: String, width: f32, view: AnyView, window: &Window, cx: &mut Context<Self>) {
        let focus = cx.focus_handle();
        self.push_view_overlay(id.clone(), OverlayKind::Dialog, size(width, 0.92), None, view, focus, window, cx);
        if let Some(o) = self.overlays.iter_mut().find(|o| o.id == id) {
            o.bare = true;
        }
    }

    /// An overlay that is a native view instead of a mod.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn push_view_overlay(&mut self, id: String, kind: OverlayKind, size: Size<f32>, top: Option<f32>, view: AnyView, focus: FocusHandle, window: &Window, cx: &mut Context<Self>) {
        dialog::forget_typed();
        // Where the view put the keyboard while it was built (its first field or button): where the keyboard goes back
        // to when another overlay (the menu this was opened from) closes.
        let initial = window.focused(cx).filter(|f| *f != focus);
        self.overlays.push(Overlay {
            slot: format!("overlay:{id}"),
            id,
            kind,
            at: self.pointer,
            size,
            expires: None,
            ttl: None,
            focus,
            layout: None,
            top,
            fit: true,
            modal: false,
            bare: false,
            place: None,
            key: None,
            frame: false,
            initial,
            view: Some(view),
        });
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::Ask;
    use crate::i18n;
    use serde_json::json;

    #[test]
    fn props_of_the_old_mod_become_the_same_ask() {
        let props = json!({
            "title": "Unsaved changes in a.rs", "detail": "a.rs has changes.", "confirm_label": "Discard",
            "command": { "name": "tab.close", "args": { "id": "t1", "confirmed": true } },
            "extra": { "label": "Save and close", "command": { "name": "editor.save_close", "args": { "panes": ["p1"] } } },
        });
        let ask = Ask::from_props(&props, Some(420.0)).unwrap();
        assert_eq!(ask.title, "Unsaved changes in a.rs");
        assert_eq!(ask.confirm_label.as_deref(), Some("Discard"));
        assert_eq!(ask.command, ("tab.close".to_owned(), json!({ "id": "t1", "confirmed": true })));
        assert_eq!(ask.extra, Some(("Save and close".to_owned(), ("editor.save_close".to_owned(), json!({ "panes": ["p1"] })))));
        assert_eq!(ask.width, 420.0);
    }

    #[test]
    fn a_plain_ask_has_a_default_title_no_extra_and_the_default_width() {
        let ask = Ask::from_props(&json!({ "command": { "name": "space.close" } }), None).unwrap();
        assert_eq!(ask.title, i18n::t("rs.confirm.are_you_sure"));
        assert_eq!(ask.detail, "");
        assert_eq!(ask.command, ("space.close".to_owned(), json!({})));
        assert!(ask.extra.is_none() && ask.confirm_label.is_none());
        assert_eq!(ask.width, 420.0);
    }

    #[test]
    fn an_ask_without_a_command_is_refused() {
        assert!(Ask::from_props(&json!({ "title": "x" }), None).is_none());
        assert!(Ask::from_props(&json!({ "command": {} }), None).is_none());
    }
}
