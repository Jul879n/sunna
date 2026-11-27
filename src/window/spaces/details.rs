//! «Edit details…» of a space: its name (empty: called after its folder again) and a note
//! shown as the tooltip of its row. Saving runs `space.details {id, name, note}`; the note is
//! kept in `shared()` (`space.notes`).

use super::super::dialog::{self, Act, Dialog};
use super::super::native::z;
use super::*;
use gpui_base::input::{Input, InputEvent, InputState};

pub(super) struct DetailsView {
    app: WeakEntity<AppWindow>,
    overlay: String,
    space: String,
    name: Entity<InputState>,
    note: Entity<InputState>,
    focus: dialog::Focus,
}

impl DetailsView {
    fn new(app: WeakEntity<AppWindow>, overlay: String, space: String, name: String, note: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Through `dialog::input`: it repaints the ring with the focus and records that something was typed (a click
        // outside then keeps the dialog).
        let field = |value: String, hint: String, window: &mut Window, cx: &mut Context<Self>| {
            dialog::input(&value, &hint, window, cx, |this: &mut Self, ev, window, cx| {
                if matches!(ev, InputEvent::PressEnter { .. }) {
                    this.save(window, cx);
                }
            })
        };
        let name = field(name, i18n::t("ctx.name_hint"), window, cx);
        let note = field(note, i18n::t("ctx.note_hint"), window, cx);
        name.update(cx, |s, cx| s.focus(window, cx));
        Self { app, overlay, space, name, note, focus: dialog::Focus::new(cx) }
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.overlay.clone();
        let _ = self.app.update(cx, |app, cx| app.close_overlay(&id, window, cx));
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let args = json!({ "id": self.space, "name": self.name.read(cx).value().to_string(), "note": self.note.read(cx).value().to_string() });
        let id = self.overlay.clone();
        let _ = self.app.update(cx, |app, cx| {
            app.close_overlay(&id, window, cx);
            app.run("space.details", args, window, cx);
        });
    }

    /// A labelled text field; while it has the keyboard its border is a 2 px accent ring.
    fn field(&self, label: String, input: &Entity<InputState>, window: &Window, cx: &App) -> Div {
        let focused = input.read(cx).focus_handle(cx).is_focused(window);
        div().flex().flex_col().gap(z(6.0)).child(dialog::label(label)).child(dialog::field_box(focused, false).child(div().flex_1().h_full().child(Input::new(input))))
    }
}

impl Render for DetailsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();
        let on = |save: bool| {
            let this = this.clone();
            move |w: &mut Window, cx: &mut App| this.update(cx, |v, cx| if save { v.save(w, cx) } else { v.close(w, cx) })
        };
        let body = div().flex().flex_col().gap(z(12.0)).child(self.field(i18n::t("ctx.name"), &self.name, window, cx)).child(self.field(i18n::t("ctx.note"), &self.note, window, cx));
        Dialog::new(i18n::t("ctx.details_title"), &self.focus, on(false))
            .wide()
            .fields(2)
            .body(body)
            .action(Act::primary(i18n::t("ctx.save"), on(true)))
            .render("space-details")
    }
}

impl AppWindow {
    /// Opens the details dialog of a space.
    pub(super) fn open_details(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.model.space(id).map(|p| p.name.clone()).unwrap_or_default();
        let note = mods::shared("space.notes").and_then(|v| v.get(id).and_then(Value::as_str).map(str::to_owned)).unwrap_or_default();
        let (overlay, app, space) = (mods::new_overlay_id(), cx.weak_entity(), id.to_owned());
        let view = cx.new(|cx| DetailsView::new(app, overlay.clone(), space, name, note, window, cx));
        self.push_dialog_overlay(overlay, dialog::WIDE + 48.0, view.into(), window, cx);
    }
}
