//! The small set of controls every settings page is made of: heading, row
//! (one setting per card), switch, segmented choice, stepper, button, text
//! field. Pages only compose these. Targets are at least 32 px tall (44 px
//! for the primary ones is met by the card row around them), states show on
//! hover, and the switch and choices say what they are on to screen readers
//! by their text.

use super::*;
use gpui_base::input::{Input, InputState};

/// What a control does when pressed.
pub type Act = Rc<dyn Fn(&mut Window, &mut App)>;

pub fn act(f: impl Fn(&mut Window, &mut App) + 'static) -> Act {
    Rc::new(f)
}

fn z(v: f32) -> Pixels {
    settings::z(v)
}

/// What every focusable control shows while the keyboard (not the mouse) has
/// it: a 2 px ring in the accent. Controls are tab stops; Enter and Space press them.
pub fn ring(s: StyleRefinement) -> StyleRefinement {
    s.border_2().border_color(c("accent"))
}

/// A group's small caps title.
pub fn heading(text: impl Into<SharedString>) -> Div {
    let text: SharedString = text.into();
    div().pt(z(6.0)).pb(z(8.0)).text_size(z(12.0)).font_weight(FontWeight::SEMIBOLD).text_color(c("muted")).child(text.to_uppercase())
}

/// A paragraph under a heading.
pub fn note(text: impl Into<SharedString>) -> Div {
    div().pb(z(12.0)).text_size(z(13.0)).text_color(c("muted")).child(text.into())
}

pub fn gap() -> Div {
    div().h(z(18.0))
}

/// One setting: what it is on the left, the control on the right.
pub fn row(title: impl Into<SharedString>, about: Option<String>, control: impl IntoElement) -> Div {
    div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(z(16.0))
        .px(z(16.0))
        .py(z(12.0))
        .mb(z(8.0))
        .rounded(z(10.0))
        .border_1()
        .border_color(c("border"))
        .bg(c("surface"))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(z(3.0))
                .child(div().text_size(z(14.0)).text_color(c("foreground")).child(title.into()))
                .when_some(about.filter(|a| !a.is_empty()), |d, a| d.child(div().text_size(z(12.5)).line_height(z(18.0)).text_color(c("muted")).child(a))),
        )
        .child(div().flex_none().child(control))
}

pub fn button(id: impl Into<ElementId>, label: impl Into<SharedString>, on: Act) -> Stateful<Div> {
    div()
        .id(id)
        .h(z(32.0))
        .px(z(12.0))
        .flex()
        .items_center()
        .rounded(z(7.0))
        .border_1()
        .border_color(c("border"))
        .bg(c("raised"))
        .cursor_pointer()
        .text_size(z(13.0))
        .text_color(c("foreground"))
        .hover(|s| s.bg(c("hover")))
        .active(|s| s.bg(c("selected")))
        .tab_index(0)
        .focus_visible(ring)
        .on_click(move |_, w, cx| on(w, cx))
        .child(label.into())
}

/// The call to action of a row: filled with the accent.
pub fn primary(id: impl Into<ElementId>, label: impl Into<SharedString>, on: Act) -> Stateful<Div> {
    div()
        .id(id)
        .h(z(32.0))
        .px(z(12.0))
        .flex()
        .items_center()
        .rounded(z(7.0))
        .cursor_pointer()
        .bg(c("accent"))
        .text_color(c("accent_foreground"))
        .text_size(z(13.0))
        .font_weight(FontWeight::SEMIBOLD)
        .border_1()
        .border_color(c("accent"))
        .hover(|s| s.opacity(0.9))
        .active(|s| s.opacity(0.8))
        .tab_index(0)
        .focus_visible(|s| s.border_2().border_color(c("foreground")))
        .on_click(move |_, w, cx| on(w, cx))
        .child(label.into())
}

/// An on/off switch.
pub fn switch(id: impl Into<ElementId>, on: bool, toggle: Act) -> Stateful<Div> {
    div()
        .id(id)
        // A 44 x 32 target around the 36 x 20 track.
        .w(z(44.0))
        .h(z(32.0))
        .flex()
        .items_center()
        .justify_end()
        .rounded(z(8.0))
        .cursor_pointer()
        .tab_index(0)
        .focus_visible(ring)
        .on_click(move |_, w, cx| toggle(w, cx))
        .child(
            div()
                .w(z(36.0))
                .h(z(20.0))
                .p(z(2.0))
                .rounded(z(10.0))
                .bg(if on { c("accent") } else { c("hover") })
                .flex()
                .when(on, |d| d.justify_end())
                .child(div().w(z(16.0)).h(z(16.0)).rounded(z(8.0)).bg(if on { c("accent_foreground") } else { c("muted") })),
        )
}

/// A row of choices, one picked. `choices`: (value, label).
pub fn segmented<V: PartialEq + Clone + 'static>(id: &str, choices: Vec<(V, String)>, now: &V, pick: impl Fn(V, &mut Window, &mut App) + 'static) -> Div {
    let pick = Rc::new(pick);
    div()
        .flex()
        .flex_row()
        .p(z(2.0))
        .gap(z(2.0))
        .rounded(z(8.0))
        .border_1()
        .border_color(c("border"))
        .bg(c("background"))
        .children(choices.into_iter().enumerate().map(|(i, (value, label))| {
            let on = &value == now;
            let pick = pick.clone();
            div()
                .id(SharedString::from(format!("st-{id}-{i}")))
                .h(z(32.0))
                .px(z(12.0))
                .flex()
                .items_center()
                .rounded(z(6.0))
                .text_size(z(13.0))
                .cursor_pointer()
                .border_1()
                .border_color(if on { c("border") } else { gpui::transparent_black() })
                .when(on, |d| d.bg(c("raised")).text_color(c("foreground")))
                .when(!on, |d| d.text_color(c("muted")).hover(|s| s.text_color(c("foreground")).bg(c("hover"))))
                .tab_index(0)
                .focus_visible(ring)
                .on_click(move |_, w, cx| pick(value.clone(), w, cx))
                .child(label)
        }))
}

/// − value +
pub fn stepper(id: &str, value: impl Into<SharedString>, minus: Act, plus: Act) -> Div {
    let side = |k: &str, label: &'static str, f: Act| {
        div()
            .id(SharedString::from(format!("st-{id}-{k}")))
            .w(z(32.0))
            .h(z(32.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .text_size(z(15.0))
            .text_color(c("muted"))
            .hover(|s| s.bg(c("hover")).text_color(c("foreground")))
            .active(|s| s.bg(c("selected")))
            .tab_index(0)
            .focus_visible(ring)
            .on_click(move |_, w, cx| f(w, cx))
            .child(label)
    };
    div()
        .flex()
        .flex_row()
        .items_center()
        .rounded(z(7.0))
        .border_1()
        .border_color(c("border"))
        .bg(c("raised"))
        .overflow_hidden()
        .child(side("minus", "\u{2212}", minus))
        .child(div().min_w(z(54.0)).px(z(4.0)).text_size(z(13.0)).text_color(c("foreground")).flex().justify_center().child(value.into()))
        .child(side("plus", "+", plus))
}

fn field_base(input: &Entity<InputState>, focused: bool) -> Div {
    div()
        .h(z(32.0))
        .px(z(8.0))
        .flex()
        .items_center()
        .rounded(z(7.0))
        .border_1()
        .border_color(c("border"))
        .when(focused, |d| d.border_2().border_color(c("accent")))
        .bg(c("background"))
        .text_size(z(12.5))
        .text_color(c("foreground"))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(div().size_full().child(Input::new(input)))
}

/// The same, showing the ring while it has the keyboard.
pub fn field_ring(input: &Entity<InputState>, width: f32, focused: bool) -> Div {
    field_base(input, focused).w(z(width))
}

/// A fixed-width field that can show the ring and a wrong value (a bad color).
pub fn field_state(input: &Entity<InputState>, width: f32, focused: bool, invalid: bool) -> Div {
    field_base(input, focused).w(z(width)).when(invalid, |d| d.border_color(c("blocked")))
}

/// The same, taking the width its row leaves.
pub fn field_fill(input: &Entity<InputState>, focused: bool) -> Div {
    field_base(input, focused).flex_1().min_w_0()
}

/// How the app's inputs look inside settings: the theme's colors.
pub fn input_style() -> gpui_base::input::InputEditorStyle {
    gpui_base::input::InputEditorStyle {
        foreground: c("foreground"),
        muted_foreground: c("muted"),
        background: gpui::transparent_black(),
        border: gpui::transparent_black(),
        selection: c("selection"),
        caret: c("foreground"),
        ..Default::default()
    }
}

/// A 32 x 32 button drawn as an icon (an x to remove, a pencil to edit).
pub fn icon_button(id: impl Into<ElementId>, icon: &'static [u8], on: Act) -> Stateful<Div> {
    div()
        .id(id)
        .size(z(32.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(z(7.0))
        .border_1()
        .border_color(gpui::transparent_black())
        .cursor_pointer()
        .hover(|s| s.bg(c("hover")))
        .active(|s| s.bg(c("selected")))
        .tab_index(0)
        .focus_visible(ring)
        .on_click(move |_, w, cx| on(w, cx))
        .child(svg().data(icon).size(z(14.0)).text_color(c("muted")))
}

/// One choice of a list of exclusive options: a radio mark and its text, 44 px tall.
pub fn radio(id: impl Into<ElementId>, label: impl Into<SharedString>, on: bool, pick: Act) -> Stateful<Div> {
    div()
        .id(id)
        .w_full()
        .min_h(z(44.0))
        .px(z(14.0))
        .flex()
        .flex_row()
        .items_center()
        .gap(z(12.0))
        .rounded(z(10.0))
        .border_1()
        .border_color(if on { c("accent") } else { c("border") })
        .bg(if on { c("raised") } else { c("surface") })
        .cursor_pointer()
        .hover(|s| s.bg(c("hover")))
        .active(|s| s.bg(c("selected")))
        .tab_index(0)
        .focus_visible(ring)
        .on_click(move |_, w, cx| pick(w, cx))
        .child(
            div()
                .size(z(18.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .border_2()
                .border_color(if on { c("accent") } else { c("muted") })
                .when(on, |d| d.child(div().size(z(8.0)).rounded_full().bg(c("accent")))),
        )
        .child(div().text_size(z(14.0)).text_color(c("foreground")).child(label.into()))
}

/// A group of rows in one bordered card (the shortcuts of a section).
pub fn card() -> Div {
    div().w_full().flex().flex_col().p(z(6.0)).mb(z(10.0)).rounded(z(10.0)).border_1().border_color(c("border")).bg(c("surface"))
}
