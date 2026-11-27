//! The one dialog every native dialog is built from: a centered card (420 px, 480 for forms) over a dimmed
//! window, a title with an optional description, a body the caller draws, and a footer with Cancel and the
//! actions on the right (a destructive one is a tinted red button, border included). A visible 32 px close
//! button sits at the top right for people without a keyboard; Esc does the same.
//!
//! Keyboard: Tab and Shift+Tab walk the fields, then the buttons, then the close button, and wrap inside the
//! card. Enter runs the primary action (a form wires its fields' Enter itself). The first stop is the first
//! field, else the primary button; a destructive dialog starts on Cancel so Enter cannot destroy by reflex.
//!
//! The rules (order of the stops, the default action, the colors and their contrast) are plain functions at
//! the top and tested at the bottom; the drawing below only applies them.

use crate::i18n;
use gpui_base::input::{Input, InputEvent, InputState};
use std::cell::RefCell;
use std::collections::HashMap;
use gpui_shell::action::ShellAction;
use super::native::{Rgb, icon, opaque, pull_readable, r, tip, z};
use super::*;

const CLOSE_ICON: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/x.svg");
/// Card widths: a question, and a form.
pub(super) const NARROW: f32 = 420.0;
pub(super) const WIDE: f32 = 480.0;
/// Room around the card so its shadow is not cut by the overlay's frame.
const MARGIN: f32 = 24.0;
/// The tallest the actions can be (a third one is the editor's "Save and close").
const SLOTS: usize = 3;

// Rules ------------------------------------------------------------------------------

/// How a button looks and what it means.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Tone {
    Neutral,
    Primary,
    Danger,
}

/// Where the keyboard can stand inside a dialog (the order is checked by the tests).
#[derive(Clone, Copy, PartialEq, Debug)]
#[cfg_attr(not(test), allow(dead_code))]
pub(super) enum Stop {
    Field(usize),
    Cancel,
    Action(usize),
    Close,
}

/// The stops in the order Tab walks them: the fields, Cancel, the actions left to right, then the close button.
#[cfg(test)]
pub(super) fn tab_order(fields: usize, actions: usize) -> Vec<Stop> {
    (0..fields).map(Stop::Field).chain([Stop::Cancel]).chain((0..actions).map(Stop::Action)).chain([Stop::Close]).collect()
}

/// The action Enter runs: the last primary one (the safe choice), else the last destructive one.
pub(super) fn default_action(tones: &[Tone]) -> Option<usize> {
    tones.iter().rposition(|t| *t == Tone::Primary).or_else(|| tones.iter().rposition(|t| *t == Tone::Danger))
}

/// Where the keyboard starts: the first field, else the primary button, else Cancel (a destructive dialog).
pub(super) fn initial_stop(fields: usize, tones: &[Tone]) -> Stop {
    if fields > 0 {
        return Stop::Field(0);
    }
    match tones.iter().rposition(|t| *t == Tone::Primary) {
        Some(i) => Stop::Action(i),
        None => Stop::Cancel,
    }
}

/// The focus handle of action `i` among `n`: counted from the right, so the last (primary) button keeps its
/// handle when another button appears or goes (the welcome's Back), and the keyboard stays on it.
pub(super) fn slot(i: usize, n: usize) -> usize {
    SLOTS - n.clamp(1, SLOTS) + i.min(n.saturating_sub(1)).min(SLOTS - 1)
}

/// The next stop after `at` (None: the keyboard is outside the card), wrapping both ways.
#[cfg(test)]
pub(super) fn step(order: &[Stop], at: Option<Stop>, forward: bool) -> Stop {
    let n = order.len();
    let i = at.and_then(|s| order.iter().position(|o| *o == s));
    order[match (i, forward) {
        (Some(i), true) => (i + 1) % n,
        (Some(i), false) => (i + n - 1) % n,
        (None, true) => 0,
        (None, false) => n - 1,
    }]
}

/// The theme colors a dialog uses.
#[derive(Clone, Copy)]
pub(super) struct Palette {
    pub popover: Rgb,
    pub foreground: Rgb,
    pub muted: Rgb,
    pub accent: Rgb,
    pub accent_foreground: Rgb,
    pub blocked: Rgb,
    pub raised: Rgb,
    pub input: Rgb,
    /// The theme's selection color.
    pub selection: Rgb,
    /// The window's own color (what a field is filled with, when its text reads on it).
    pub background: Rgb,
}

impl Palette {
    /// The dialog colors of a resolved theme. A translucent one (the theme's `input` line) is laid over the card.
    pub fn of(theme: &crate::settings::Theme) -> Self {
        let get = |name: &str, under: Rgb| {
            let color = theme.get(name).and_then(|v| crate::settings::parse_hex(v)).unwrap_or(hsla(0.83, 1.0, 0.5, 1.0));
            opaque(color, under)
        };
        let background = get("background", [0.0; 3]);
        let popover = get("popover", background);
        Self {
            popover,
            foreground: get("foreground", popover),
            muted: get("muted", popover),
            accent: get("accent", popover),
            accent_foreground: get("accent_foreground", popover),
            blocked: get("blocked", popover),
            raised: get("raised", popover),
            input: get("input", popover),
            selection: get("selection", popover),
            background,
        }
    }
}

/// A button's colors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Paint {
    pub fill: Rgb,
    pub border: Rgb,
    pub text: Rgb,
}

fn mix(a: Rgb, over: Rgb, alpha: f32) -> Rgb {
    [0, 1, 2].map(|i| a[i] * alpha + over[i] * (1.0 - alpha))
}

/// The colors of a button of `tone`: text always reads at 4.5:1 on its fill. A destructive one is the theme's
/// red tinted into the card (fill and full border), its text a readable red.
pub(super) fn paint(p: &Palette, tone: Tone) -> Paint {
    const MIN: f32 = 4.5;
    match tone {
        Tone::Neutral => Paint { fill: p.raised, border: p.input, text: pull_readable(p.foreground, &[p.raised], p.foreground, MIN) },
        Tone::Primary => Paint { fill: p.accent, border: p.accent, text: pull_readable(p.accent_foreground, &[p.accent], p.foreground, MIN) },
        Tone::Danger => {
            let fill = mix(p.blocked, p.popover, 0.16);
            Paint { fill, border: mix(p.blocked, p.popover, 0.55), text: pull_readable(p.blocked, &[fill], p.foreground, MIN) }
        }
    }
}

/// The description's grey: the theme's muted text, nearer the foreground when it would not read on the card.
pub(super) fn description(p: &Palette) -> Rgb {
    pull_readable(p.muted, &[p.popover], p.foreground, 4.5)
}

/// The accent as a focus ring: the theme's own, nearer the text when it would not be seen against the card (3:1).
pub(super) fn accent_ring(p: &Palette) -> Rgb {
    pull_readable(p.accent, &[p.popover], p.foreground, 3.0)
}

/// The ring a focused button wears: the accent ring, except on the primary button (its own fill), which takes the text color.
pub(super) fn ring_color(p: &Palette, tone: Tone) -> Rgb {
    if tone == Tone::Primary { p.foreground } else { pull_readable(p.accent, &[p.popover, paint(p, tone).fill], p.foreground, 3.0) }
}

/// How dark the veil behind a dialog is: stronger over a dark window (where a black veil changes little), lighter
/// over a light one. `background` is the window's color.
pub(super) fn scrim_alpha(background: Rgb) -> f32 {
    if super::native::contrast(background, [0.0; 3]) < 4.0 { 0.62 } else { 0.42 }
}

/// The veil drawn behind every dialog.
pub(super) fn scrim() -> Hsla {
    hsla(0.0, 0.0, 0.0, scrim_alpha(opaque(c("background"), [0.0; 3])))
}

/// A click outside the card closes the dialog only while nothing has been written in it: typed work is lost
/// only by the ✕, Esc or Cancel.
pub(super) fn closes_on_outside_click(typed: bool) -> bool {
    !typed
}

thread_local! {
    /// The text fields of the open dialogs that have text (by input): the dialog has data typed in it.
    static TYPED: RefCell<HashMap<EntityId, bool>> = RefCell::new(HashMap::new());
}

/// Records whether `input` has text (a form's field; the search lines of lists do not count).
pub(super) fn mark_typed(input: EntityId, has_text: bool) {
    TYPED.with_borrow_mut(|m| {
        m.insert(input, has_text);
    });
}

/// Does an open dialog have data typed in it?
pub(super) fn typed() -> bool {
    TYPED.with_borrow(|m| m.values().any(|v| *v))
}

/// A click outside a native dialog: the same `overlay.close` Esc sends, a frame later (the caller is mid-update). It
/// goes to whatever holds the keyboard, the dialog's card first, so the dialog's own close runs (the welcome records
/// that it was skipped) and only an orphaned one is taken away by the window.
pub(super) fn dismiss(window: &mut Window, cx: &mut App) {
    window.defer(cx, |window, cx| window.dispatch_action(ShellAction::new("overlay.close").boxed_clone(), cx));
}

/// A dialog opened or closed: what was typed in the last one no longer counts.
pub(super) fn forget_typed() {
    TYPED.with_borrow_mut(HashMap::clear);
}

fn palette() -> Palette {
    Palette::of(&crate::settings::theme())
}

/// What a text field is filled with: the window's color, else (a theme whose background is the text's own color, like
/// win98's black) the first of the raised and card colors the text reads on.
pub(super) fn field_fill(p: &Palette) -> Rgb {
    [p.background, p.raised, p.popover].into_iter().find(|f| super::native::contrast(p.foreground, *f) >= 4.5).unwrap_or(p.background)
}

/// A field's typed text, readable on its fill.
pub(super) fn field_text(p: &Palette) -> Rgb {
    pull_readable(p.foreground, &[field_fill(p)], p.foreground, 4.5)
}

/// The grey of a field's placeholder: readable on its fill (4.5:1).
pub(super) fn placeholder(p: &Palette) -> Rgb {
    pull_readable(p.muted, &[field_fill(p)], p.foreground, 4.5)
}

/// A field's outline: the theme's line, nearer the text when it would not be seen against the card (3:1, WCAG 1.4.11:
/// an empty field is only its outline).
pub(super) fn field_border(p: &Palette) -> Rgb {
    pull_readable(p.input, &[p.popover], p.foreground, 3.0)
}

/// The outline of a field in error: the theme's red, readable against the card as a graphic (3:1).
pub(super) fn error_border(p: &Palette) -> Rgb {
    pull_readable(p.blocked, &[p.popover], p.foreground, 3.0)
}

/// The fill behind selected text in a field: the theme's own when the typed text reads on it (4.5:1), else the accent
/// tinted into the field (35 % down to nothing) until it does. The field's fill itself always reads, so this ends.
pub(super) fn field_selection(p: &Palette) -> Rgb {
    let (fill, text) = (field_fill(p), field_text(p));
    let reads = |c: Rgb| super::native::contrast(text, c) >= 4.5;
    if reads(p.selection) {
        return p.selection;
    }
    (0..=7).map(|k| mix(p.accent, fill, 0.35 - 0.05 * k as f32)).find(|c| reads(*c)).unwrap_or(fill)
}

/// The editor colors of a dialog's text input.
pub(super) fn input_style() -> gpui_base::input::InputEditorStyle {
    let p = palette();
    gpui_base::input::InputEditorStyle {
        foreground: hs(field_text(&p)),
        muted_foreground: hs(placeholder(&p)),
        selection: hs(field_selection(&p)),
        ..settings_ui::controls::input_style()
    }
}

/// The fill of a field or control well drawn on the window's card: `field_fill` as a color.
pub(super) fn field_bg() -> Hsla {
    hs(field_fill(&palette()))
}

fn hs(rgb: Rgb) -> Hsla {
    Rgba { r: rgb[0], g: rgb[1], b: rgb[2], a: 1.0 }.into()
}

// Drawing ----------------------------------------------------------------------------

/// What a button or Esc runs.
pub(super) type Run = Rc<dyn Fn(&mut Window, &mut App)>;

/// The keyboard stops of one dialog: made once by its view, drawn every frame by `Dialog`.
#[derive(Clone)]
pub(super) struct Focus {
    root: FocusHandle,
    cancel: FocusHandle,
    close: FocusHandle,
    actions: [FocusHandle; SLOTS],
}

impl Focus {
    pub fn new(cx: &mut App) -> Self {
        let stop = |cx: &mut App| cx.focus_handle().tab_stop(true);
        Self { root: cx.focus_handle(), cancel: stop(cx), close: stop(cx), actions: [stop(cx), stop(cx), stop(cx)] }
    }

    /// Puts the keyboard on `stop`; a field is the view's to focus (its input owns the handle).
    /// `actions` is how many buttons the dialog has.
    pub fn put(&self, stop: Stop, actions: usize, window: &mut Window, cx: &mut App) {
        match stop {
            Stop::Field(_) => self.root.focus(window, cx),
            Stop::Cancel => self.cancel.focus(window, cx),
            Stop::Action(i) => self.actions[slot(i, actions)].focus(window, cx),
            Stop::Close => self.close.focus(window, cx),
        }
    }

    /// Tab (or Shift+Tab) inside the card: the next tab stop, wrapping, never out of it. A text field would
    /// otherwise keep the key and the keyboard could not leave it.
    fn cycle(&self, forward: bool, window: &mut Window, cx: &mut App) {
        for _ in 0..50 {
            if forward { window.focus_next(cx) } else { window.focus_prev(cx) }
            if self.root.contains_focused(window, cx) {
                return;
            }
        }
    }
}

/// One button of the footer.
pub(super) struct Act {
    label: String,
    tone: Tone,
    run: Run,
    enabled: bool,
}

impl Act {
    pub fn new(label: impl Into<String>, tone: Tone, run: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        Self { label: label.into(), tone, run: Rc::new(run), enabled: true }
    }
    pub fn primary(label: impl Into<String>, run: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        Self::new(label, Tone::Primary, run)
    }
    pub fn danger(label: impl Into<String>, run: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        Self::new(label, Tone::Danger, run)
    }
    /// Shown dimmed and inert (a form that cannot be sent yet).
    pub fn enabled(mut self, on: bool) -> Self {
        self.enabled = on;
        self
    }
}

pub(super) struct Dialog {
    title: String,
    description: Option<String>,
    body: Option<AnyElement>,
    width: f32,
    focus: Focus,
    close: Run,
    cancel: Option<String>,
    actions: Vec<Act>,
    enter: bool,
    fields: usize,
}

impl Dialog {
    /// `close` runs for Esc, the ✕ and Cancel.
    pub fn new(title: impl Into<String>, focus: &Focus, close: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        Self {
            title: title.into(),
            description: None,
            body: None,
            width: NARROW,
            focus: focus.clone(),
            close: Rc::new(close),
            cancel: Some(i18n::t("rs.confirm.cancel")),
            actions: Vec::new(),
            enter: true,
            fields: 0,
        }
    }
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into()).filter(|t: &String| !t.is_empty());
        self
    }
    pub fn body(mut self, body: impl IntoElement) -> Self {
        self.body = Some(body.into_any_element());
        self
    }
    /// Another width (a list with room for paths).
    pub fn width(mut self, px: f32) -> Self {
        self.width = px;
        self
    }
    /// A form: 480 px wide.
    pub fn wide(mut self) -> Self {
        self.width = WIDE;
        self
    }
    pub fn action(mut self, act: Act) -> Self {
        self.actions.push(act);
        self
    }
    /// A dialog with only information: no Cancel, the ✕ and Esc close it.
    pub fn no_cancel(mut self) -> Self {
        self.cancel = None;
        self
    }
    /// The Cancel button's text.
    pub fn cancel_label(mut self, text: impl Into<String>) -> Self {
        self.cancel = Some(text.into());
        self
    }
    /// The dialog has `n` text fields: Enter is theirs (they send the form), not the card's.
    pub fn fields(mut self, n: usize) -> Self {
        self.fields = n;
        self.enter = n == 0;
        self
    }

    pub fn render(self, id: &'static str) -> AnyElement {
        let p = palette();
        let tones: Vec<Tone> = self.actions.iter().map(|a| a.tone).collect();
        let default = default_action(&tones);
        let (focus, close, enter) = (self.focus.clone(), self.close.clone(), self.enter);
        let close_action = self.close.clone();
        let default_run = default.and_then(|i| self.actions.get(i)).filter(|a| a.enabled).map(|a| a.run.clone());

        let button = |bid: String, label: String, tone: Tone, handle: &FocusHandle, enabled: bool, run: Run| {
            let look = paint(&p, tone);
            let ring = hs(ring_color(&p, tone));
            let key_run = run.clone();
            div()
                .id(SharedString::from(bid))
                .track_focus(handle)
                .h(z(34.0))
                .px(z(14.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(r(8.0))
                .border_1()
                .border_color(hs(look.border))
                .bg(hs(look.fill))
                .text_color(hs(look.text))
                .text_size(z(13.0))
                .when(tone != Tone::Neutral, |d| d.font_weight(FontWeight::SEMIBOLD))
                .when(enabled, |d| d.cursor_pointer().hover(|s| s.opacity(0.88)))
                .when(!enabled, |d| d.opacity(0.5))
                .focus_visible(move |s| s.border_2().border_color(ring).px(z(13.0)))
                .on_key_down(move |e: &KeyDownEvent, window, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                        if enabled {
                            key_run(window, cx);
                        }
                        cx.stop_propagation();
                    }
                })
                // A mouse click only: Enter and Space already ran through `on_key_down`, and gpui would also click
                // on the key's release (after the dialog redrew, on whatever button then holds the focus).
                .on_click(move |e, window, cx| {
                    if enabled && !e.is_keyboard() {
                        run(window, cx)
                    }
                })
                .child(label)
        };

        let has_footer = self.cancel.is_some() || !self.actions.is_empty();
        let mut footer = div().flex().flex_row().flex_wrap().justify_end().gap(z(8.0));
        if let Some(label) = self.cancel.clone() {
            footer = footer.child(button(format!("{id}-cancel"), label, Tone::Neutral, &self.focus.cancel, true, self.close.clone()));
        }
        let n = self.actions.len();
        for (i, a) in self.actions.into_iter().enumerate() {
            footer = footer.child(button(format!("{id}-act-{i}"), a.label, a.tone, &self.focus.actions[slot(i, n)], a.enabled, a.run));
        }

        let muted = hs(description(&p));
        let close_run = self.close.clone();
        let close_key = self.close.clone();
        let ring = hs(accent_ring(&p));
        let x = div()
            .id(SharedString::from(format!("{id}-close")))
            .track_focus(&self.focus.close)
            .absolute()
            .top(z(12.0))
            .right(z(12.0))
            .size(z(32.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(r(8.0))
            .border_1()
            .border_color(Hsla { a: 0.0, ..ring })
            .cursor_pointer()
            .hover(|s| s.bg(c("hover")))
            .focus_visible(move |s| s.border_2().border_color(ring))
            .tooltip(tip(format!("{} (Esc)", i18n::t("menu.close"))))
            .on_key_down(move |e: &KeyDownEvent, window, cx| {
                if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                    close_key(window, cx);
                    cx.stop_propagation();
                }
            })
            .on_click(move |e, window, cx| if !e.is_keyboard() { close_run(window, cx) })
            .child(icon(CLOSE_ICON, 14.0, muted));

        let card = div()
            .id(id)
            .track_focus(&self.focus.root)
            .relative()
            .w(z(self.width))
            .flex()
            .flex_col()
            .gap(z(14.0))
            .p(z(20.0))
            .rounded(r(14.0))
            .border_1()
            .border_color(c("border"))
            .bg(hs(p.popover))
            .text_size(z(13.0))
            .text_color(hs(p.foreground))
            .when(!crate::settings::get().reduce_effects, |d| d.shadow(vec![BoxShadow { color: hsla(0.0, 0.0, 0.0, 0.45), offset: point(px(0.0), z(18.0)), blur_radius: z(48.0), spread_radius: px(0.0), inset: false }]))
            // Esc is the keymap's `overlay.close` (see keys.rs): it would close the overlay and skip what this dialog
            // does when closed (the welcome records that it was skipped), so the dialog takes the action first.
            .on_action(move |a: &ShellAction, window, cx| {
                if a.id() == "overlay.close" {
                    close_action(window, cx);
                } else {
                    cx.propagate();
                }
            })
            .on_key_down(move |e: &KeyDownEvent, window, cx| {
                match e.keystroke.key.as_str() {
                    "escape" => close(window, cx),
                    "tab" => focus.cycle(!e.keystroke.modifiers.shift, window, cx),
                    "enter" if enter => match &default_run {
                        Some(run) => run(window, cx),
                        None => return,
                    },
                    _ => return,
                }
                cx.stop_propagation();
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(z(4.0))
                    .pr(z(36.0))
                    .child(div().text_size(z(15.0)).font_weight(FontWeight::SEMIBOLD).line_height(z(20.0)).child(self.title))
                    .when_some(self.description, |d, text| d.child(div().text_color(muted).line_height(z(19.0)).child(text))),
            )
            .when_some(self.body, |d, body| d.child(body))
            .when(has_footer, |d| d.child(footer))
            .child(x);
        div().p(z(MARGIN)).child(card).into_any_element()
    }
}

/// A single-line input of a form, styled like the settings' fields; `on` hears its events (Enter, Change); focus
/// changes repaint the dialog so the 2 px ring follows the keyboard.
pub(super) fn input<V: 'static>(
    value: &str,
    placeholder: &str,
    window: &mut Window,
    cx: &mut Context<V>,
    on: impl Fn(&mut V, &InputEvent, &mut Window, &mut Context<V>) + 'static,
) -> Entity<InputState> {
    let input = cx.new(|cx| InputState::new(window, cx).default_value(value.to_owned()).placeholder(placeholder.to_owned()));
    input.update(cx, |s, _| s.set_editor_style(input_style()));
    cx.subscribe_in(&input, window, move |this, state, ev: &InputEvent, window, cx| {
        if matches!(ev, InputEvent::Focus | InputEvent::Blur) {
            cx.notify();
        }
        if matches!(ev, InputEvent::Change) {
            mark_typed(state.entity_id(), !state.read(cx).value().is_empty());
        }
        on(this, ev, window, cx);
    })
    .detach();
    input
}

/// A labelled field around an `input`; `error` paints its border red.
pub(super) fn field(text: impl Into<SharedString>, input: &Entity<InputState>, error: bool, window: &Window, cx: &App) -> Div {
    let focused = input.read(cx).focus_handle(cx).is_focused(window);
    div().flex().flex_col().gap(z(6.0)).child(label(text)).child(field_box(focused, error).child(div().flex_1().h_full().child(Input::new(input))))
}

/// A line of feedback under a form (an error in red, readable on the card).
pub(super) fn message(text: impl Into<SharedString>, error: bool) -> Div {
    let p = palette();
    let color = if error { paint(&p, Tone::Danger).text } else { description(&p) };
    div().text_size(z(12.0)).line_height(z(17.0)).text_color(hs(color)).child(text.into())
}

/// A press handler for a button or Esc that calls `f` on the view `this`.
pub(super) fn run<V: 'static>(this: &Entity<V>, f: fn(&mut V, &mut Window, &mut Context<V>)) -> impl Fn(&mut Window, &mut App) + 'static {
    let this = this.clone();
    move |window, cx| this.update(cx, |v, cx| f(v, window, cx))
}

/// A small neutral button inside a form (next to a field): 34 px tall, a tab stop, Enter and Space press it.
pub(super) fn side_button(id: impl Into<SharedString>, text: impl Into<SharedString>, press: impl Fn(&mut Window, &mut App) + 'static) -> Stateful<Div> {
    side_button_enabled(id, text, true, press)
}

/// A `side_button` that can be off (a form that cannot be sent yet): dimmed, inert and skipped by Tab.
pub(super) fn side_button_enabled(id: impl Into<SharedString>, text: impl Into<SharedString>, enabled: bool, press: impl Fn(&mut Window, &mut App) + 'static) -> Stateful<Div> {
    let p = palette();
    let look = paint(&p, Tone::Neutral);
    let press = Rc::new(press);
    let key = press.clone();
    let ring = hs(accent_ring(&p));
    div()
        .id(id.into())
        .focusable()
        .tab_stop(enabled)
        .h(z(34.0))
        .px(z(14.0))
        .flex()
        .flex_none()
        .items_center()
        .rounded(r(8.0))
        .border_1()
        .border_color(hs(look.border))
        .bg(hs(look.fill))
        .text_color(hs(look.text))
        .text_size(z(13.0))
        .when(enabled, |d| d.cursor_pointer().hover(|s| s.opacity(0.88)))
        .when(!enabled, |d| d.opacity(0.5))
        .focus_visible(move |s| s.border_2().border_color(ring).px(z(13.0)))
        .on_key_down(move |e: &KeyDownEvent, window, cx| {
            if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                if enabled {
                    key(window, cx);
                }
                cx.stop_propagation();
            }
        })
        // A mouse click only: Enter and Space ran through `on_key_down` (gpui would also click on the key's release).
        .on_click(move |e, window, cx| {
            if enabled && !e.is_keyboard() {
                press(window, cx)
            }
        })
        .child(text.into())
}

/// A switch in a form: a 16 px box (checked: accent fill and a check) and its text, 34 px tall, a tab stop; a click,
/// Enter or Space flips it by calling `press`. The text reads at the description's grey, the box at the field's border.
pub(super) fn toggle(id: impl Into<SharedString>, text: impl Into<SharedString>, on: bool, press: impl Fn(&mut Window, &mut App) + 'static) -> Stateful<Div> {
    let p = palette();
    let press = Rc::new(press);
    let key = press.clone();
    let ring = hs(accent_ring(&p));
    let (border, accent, check) = (hs(field_border(&p)), hs(p.accent), hs(p.accent_foreground));
    div()
        .id(id.into())
        .focusable()
        .tab_stop(true)
        .h(z(34.0))
        .px(z(7.0))
        .flex()
        .items_center()
        .gap(z(9.0))
        .rounded(r(8.0))
        .border_1()
        .border_color(Hsla { a: 0.0, ..border })
        .cursor_pointer()
        .text_color(hs(field_text(&p)))
        .text_size(z(13.0))
        .focus_visible(move |s| s.border_2().border_color(ring).px(z(6.0)))
        .on_key_down(move |e: &KeyDownEvent, window, cx| {
            if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                key(window, cx);
                cx.stop_propagation();
            }
        })
        .on_click(move |e, window, cx| {
            if !e.is_keyboard() {
                press(window, cx)
            }
        })
        .child(
            div()
                .size(z(16.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(r(4.0))
                .border_1()
                .border_color(if on { accent } else { border })
                .when(on, |d| d.bg(accent))
                .when(on, |d| d.child(super::native::icon(include_bytes!("../../defaults/mods/workspace-chrome/check.svg"), 12.0, check))),
        )
        .child(text.into())
}

/// A field's label: 12 px, semibold, the description's grey.
pub(super) fn label(text: impl Into<SharedString>) -> Div {
    div().text_size(z(12.0)).font_weight(FontWeight::SEMIBOLD).text_color(hs(description(&palette()))).child(text.into())
}

/// A bordered box for a field's input: 34 px tall, radius 8, full border; while `focused` the border is a 2 px
/// accent ring (the padding gives that pixel back); `error` paints it red.
pub(super) fn field_box(focused: bool, error: bool) -> Div {
    let p = palette();
    let ring = hs(if error { error_border(&p) } else if focused { accent_ring(&p) } else { field_border(&p) });
    div()
        .h(z(34.0))
        .px(z(if focused { 9.0 } else { 10.0 }))
        .flex()
        .items_center()
        .rounded(r(8.0))
        .when(focused, |d| d.border_2())
        .when(!focused, |d| d.border_1())
        .border_color(ring)
        .bg(hs(field_fill(&p)))
        .text_size(z(13.0))
        .text_color(hs(field_text(&p)))
}

#[cfg(test)]
mod tests {
    use super::*;
    // `super::*` brings gpui's own `test` macro: the plain one is the std attribute.
    use core::prelude::v1::test;
    use crate::window::native::contrast;

    const DARK: Palette = Palette {
        popover: [0.090, 0.094, 0.110],
        foreground: [0.910, 0.914, 0.925],
        muted: [0.604, 0.616, 0.651],
        accent: [0.478, 0.635, 0.969],
        accent_foreground: [0.043, 0.063, 0.125],
        blocked: [1.0, 0.482, 0.525],
        raised: [0.114, 0.122, 0.145],
        input: [0.184, 0.196, 0.227],
        selection: [0.20, 0.27, 0.45],
        background: [0.043, 0.047, 0.063],
    };
    const LIGHT: Palette = Palette {
        popover: [1.0, 1.0, 1.0],
        foreground: [0.10, 0.11, 0.13],
        muted: [0.55, 0.57, 0.60],
        accent: [0.23, 0.44, 0.88],
        accent_foreground: [1.0, 1.0, 1.0],
        blocked: [0.82, 0.18, 0.23],
        raised: [0.94, 0.95, 0.96],
        input: [0.80, 0.82, 0.85],
        selection: [0.70, 0.80, 0.98],
        background: [1.0, 1.0, 1.0],
    };

    #[test]
    fn tab_walks_fields_then_buttons_then_close_and_wraps() {
        let order = tab_order(2, 2);
        assert_eq!(order, [Stop::Field(0), Stop::Field(1), Stop::Cancel, Stop::Action(0), Stop::Action(1), Stop::Close]);
        assert_eq!(step(&order, Some(Stop::Field(0)), true), Stop::Field(1));
        assert_eq!(step(&order, Some(Stop::Close), true), Stop::Field(0));
        assert_eq!(step(&order, Some(Stop::Field(0)), false), Stop::Close);
        assert_eq!(step(&order, None, true), Stop::Field(0));
        assert_eq!(step(&order, None, false), Stop::Close);
        // A question has no fields: Cancel first.
        assert_eq!(tab_order(0, 1), [Stop::Cancel, Stop::Action(0), Stop::Close]);
    }

    #[test]
    fn the_keyboard_starts_on_the_first_field_else_the_primary_button_else_cancel() {
        assert_eq!(initial_stop(2, &[Tone::Primary]), Stop::Field(0));
        assert_eq!(initial_stop(0, &[Tone::Primary]), Stop::Action(0));
        // Destructive: Cancel, so Enter cannot destroy by reflex.
        assert_eq!(initial_stop(0, &[Tone::Danger]), Stop::Cancel);
        // Discard (red) and Save and close (primary): the safe one.
        assert_eq!(initial_stop(0, &[Tone::Danger, Tone::Primary]), Stop::Action(1));
    }

    /// The primary button (the last) keeps its focus handle when the welcome adds or drops Back.
    #[test]
    fn the_last_action_keeps_its_focus_handle_whatever_the_count() {
        assert_eq!(slot(0, 1), slot(1, 2));
        assert_eq!(slot(1, 2), slot(2, 3));
        assert_eq!((slot(0, 1), slot(0, 2), slot(1, 2), slot(0, 3), slot(1, 3), slot(2, 3)), (2, 1, 2, 0, 1, 2));
        // Distinct buttons never share a handle.
        for n in 1..=SLOTS {
            let all: std::collections::HashSet<_> = (0..n).map(|i| slot(i, n)).collect();
            assert_eq!(all.len(), n);
        }
        assert_eq!(slot(0, 0), SLOTS - 1);
    }

    #[test]
    fn enter_runs_the_primary_action_else_the_destructive_one() {
        assert_eq!(default_action(&[Tone::Danger, Tone::Primary]), Some(1));
        assert_eq!(default_action(&[Tone::Danger]), Some(0));
        assert_eq!(default_action(&[Tone::Neutral]), None);
        assert_eq!(default_action(&[]), None);
    }

    /// Every text a dialog draws reads at 4.5:1 on what is behind it, in a dark and a light theme.
    #[test]
    fn dialog_text_reads_at_four_and_a_half_to_one() {
        for (name, p) in [("dark", DARK), ("light", LIGHT)] {
            let ratio = |fg: Rgb, bg: Rgb| contrast(fg, bg);
            assert!(ratio(p.foreground, p.popover) >= 4.5, "{name}: title");
            assert!(ratio(description(&p), p.popover) >= 4.5, "{name}: description and labels");
            for tone in [Tone::Neutral, Tone::Primary, Tone::Danger] {
                let look = paint(&p, tone);
                assert!(ratio(look.text, look.fill) >= 4.5, "{name}: {tone:?} button text {}", ratio(look.text, look.fill));
                // The focus ring must be seen against the card and the button (3:1 for graphics).
                assert!(ratio(ring_color(&p, tone), p.popover) >= 3.0, "{name}: {tone:?} ring on the card");
                // On the primary button the ring is the text color, set against the card (the button's own fill is the accent).
                if tone != Tone::Primary {
                    assert!(ratio(ring_color(&p, tone), look.fill) >= 3.0, "{name}: {tone:?} ring on the button");
                }
            }
        }
    }

    /// Every theme the app ships (the presets, the light ones, win98...): what a dialog draws reads at WCAG AA, the
    /// description, labels, typed text, placeholder and buttons at 4.5:1, outlines and rings at 3:1.
    #[test]
    fn every_shipped_theme_reads_in_a_dialog() {
        let themes = crate::settings::shipped_themes_resolved();
        // At least a light, a dark and win98 are among them.
        for need in ["paper", "graphite", "win98"] {
            assert!(themes.iter().any(|(n, _)| n == need), "{need} is checked");
        }
        let mut failures = Vec::new();
        for (name, theme) in &themes {
            let p = Palette::of(theme);
            let mut check = |what: &str, fg: Rgb, bg: Rgb, min: f32| {
                let got = contrast(fg, bg);
                if got < min {
                    failures.push(format!("{name}: {what} {got:.2} < {min}"));
                }
            };
            check("title", p.foreground, p.popover, 4.5);
            check("description and labels", description(&p), p.popover, 4.5);
            check("error message", paint(&p, Tone::Danger).text, p.popover, 4.5);
            check("typed text", field_text(&p), field_fill(&p), 4.5);
            check("placeholder", placeholder(&p), field_fill(&p), 4.5);
            check("field outline", field_border(&p), p.popover, 3.0);
            check("field outline in error", error_border(&p), p.popover, 3.0);
            for tone in [Tone::Neutral, Tone::Primary, Tone::Danger] {
                let look = paint(&p, tone);
                check(&format!("{tone:?} button text"), look.text, look.fill, 4.5);
                check(&format!("{tone:?} focus ring on the card"), ring_color(&p, tone), p.popover, 3.0);
                if tone != Tone::Primary {
                    check(&format!("{tone:?} focus ring on its button"), ring_color(&p, tone), look.fill, 3.0);
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// win98 paints the window black and its text black: a field filled with the window color would hide what is typed.
    #[test]
    fn a_field_never_hides_its_text_even_in_win98() {
        let themes = crate::settings::shipped_themes_resolved();
        let (_, theme) = themes.iter().find(|(n, _)| n == "win98").unwrap();
        let p = Palette::of(theme);
        assert!(contrast(p.foreground, p.background) < 4.5, "the case this guards: text on the window color does not read");
        assert!(contrast(field_text(&p), field_fill(&p)) >= 4.5);
    }

    #[test]
    fn a_destructive_button_is_a_tinted_fill_with_a_full_border() {
        let look = paint(&DARK, Tone::Danger);
        assert_ne!(look.fill, DARK.popover);
        assert_ne!(look.border, look.fill);
        // Tinted, not solid: much nearer the card than the red itself.
        assert!(contrast(look.fill, DARK.popover) < 1.6);
    }

    #[test]
    fn the_veil_is_stronger_over_a_dark_window() {
        assert!(scrim_alpha(DARK.popover) > scrim_alpha(LIGHT.popover));
        assert!(scrim_alpha(LIGHT.popover) >= 0.4);
    }

    #[test]
    fn a_click_outside_closes_only_an_empty_dialog() {
        assert!(closes_on_outside_click(false));
        assert!(!closes_on_outside_click(true));
        let id = EntityId::from(1u64);
        mark_typed(id, true);
        assert!(typed());
        mark_typed(id, false);
        assert!(!typed());
        mark_typed(id, true);
        forget_typed();
        assert!(!typed());
    }

    #[test]
    fn a_dialog_is_420_wide_and_a_form_480() {
        assert_eq!((NARROW, WIDE), (420.0, 480.0));
    }
}
