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
}

impl Ask {
    pub fn new(title: impl Into<String>, detail: impl Into<String>, command: &str, args: Value) -> Self {
        Self { title: title.into(), detail: detail.into(), confirm_label: None, command: (command.to_owned(), args), extra: None, width: 380.0 }
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
            width: width.unwrap_or(380.0),
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
    focus: FocusHandle,
}

impl ConfirmView {
    fn new(app: WeakEntity<AppWindow>, id: String, ask: Ask, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        Self { app, id, ask, focus }
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

    fn button(id: &'static str, label: String, bg: &str, fg: &str, border: &str, on: impl Fn(&mut Window, &mut App) + 'static) -> Stateful<Div> {
        let z = settings::z;
        div().id(id).h(z(32.0)).px(z(14.0)).flex().items_center().rounded(z(8.0)).border_1().border_color(c(border)).bg(c(bg)).text_color(c(fg))
            .cursor_pointer().hover(|s| s.opacity(0.88)).on_click(move |_, w, cx| on(w, cx)).child(label)
    }
}

impl Render for ConfirmView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let z = settings::z;
        let this = cx.entity();
        let act = |which: Option<Choice>| {
            let this = this.clone();
            move |w: &mut Window, cx: &mut App| this.update(cx, |v, cx| match which {
                Some(which) => v.choose(which, w, cx),
                None => v.close(w, cx),
            })
        };
        let extra = self.ask.extra.as_ref();
        let confirm_label = self.ask.confirm_label.clone().unwrap_or_else(|| i18n::t("rs.confirm.confirm"));
        div()
            .id("confirm")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, w, cx| {
                match e.keystroke.key.as_str() {
                    "enter" => this.choose(if this.ask.extra.is_some() { Choice::Extra } else { Choice::Confirm }, w, cx),
                    "escape" => this.close(w, cx),
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .w_full()
            .min_h_0()
            .p(z(18.0))
            .flex()
            .flex_col()
            .gap(z(12.0))
            .text_size(z(13.0))
            .child(
                div().flex().flex_row().items_center()
                    .child(div().flex_1().text_size(z(15.0)).font_weight(FontWeight::SEMIBOLD).text_color(c("foreground")).child(self.ask.title.clone()))
                    .child(div().text_size(z(11.0)).text_color(c("dim")).child("Esc")),
            )
            .child(div().text_color(c("muted")).line_height(z(19.0)).child(self.ask.detail.clone()))
            .child(
                div().flex().flex_row().justify_end().gap(z(8.0)).text_size(z(12.0))
                    .child(Self::button("no", i18n::t("rs.confirm.cancel"), "raised", "foreground", "input", act(None)))
                    .child(if extra.is_some() {
                        Self::button("yes", confirm_label, "raised", "foreground", "input", act(Some(Choice::Confirm)))
                    } else {
                        Self::button("yes", confirm_label, "solid", "solid_foreground", "solid", act(Some(Choice::Confirm)))
                    })
                    .when_some(extra, |d, (label, _)| d.child(Self::button("more", label.clone(), "solid", "solid_foreground", "solid", act(Some(Choice::Extra))))),
            )
    }
}

impl AppWindow {
    /// Opens the confirmation dialog.
    pub(super) fn open_confirm(&mut self, ask: Ask, window: &mut Window, cx: &mut Context<Self>) {
        let id = mods::new_overlay_id();
        let app = cx.weak_entity();
        let width = ask.width;
        let view = cx.new(|cx| ConfirmView::new(app, id.clone(), ask, window, cx));
        let focus = cx.focus_handle();
        self.push_view_overlay(id, OverlayKind::Dialog, size(width, 0.85), None, view.into(), focus, cx);
    }

    /// An overlay that is a native view instead of a mod.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn push_view_overlay(&mut self, id: String, kind: OverlayKind, size: Size<f32>, top: Option<f32>, view: AnyView, focus: FocusHandle, cx: &mut Context<Self>) {
        self.overlays.push(Overlay {
            slot: format!("overlay:{id}"),
            id,
            kind,
            at: self.pointer,
            size,
            expires: None,
            focus,
            layout: None,
            top,
            fit: true,
            modal: false,
            bare: false,
            place: None,
            key: None,
            frame: false,
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
        assert_eq!(ask.width, 380.0);
    }

    #[test]
    fn an_ask_without_a_command_is_refused() {
        assert!(Ask::from_props(&json!({ "title": "x" }), None).is_none());
        assert!(Ask::from_props(&json!({ "command": {} }), None).is_none());
    }
}
