//! The context menu, native: one component for every right-click and «⋯» menu of the
//! sidebar (a space, a project, a machine). It opens as a popup at the pointer, keeps its
//! submenus in the same panel (a row with a chevron opens them, the first line goes back),
//! and every entry is a command: choosing one closes the menu and runs `app.run(name, args)`,
//! so what a menu can do, `sunna do` can too.
//!
//! Keys: ↑ ↓ Home End move, → or Enter opens a submenu, ← or Backspace goes back, Enter or
//! Space runs, Esc closes. The keyboard cursor wears the accent ring (`native::ring`); the
//! pointer only tints the row under it. Rows are 32 px; the divider is `native::fade`.
//! What is drawn is separate from what is chosen: `Item`s and the cursor rules are plain
//! values, tested at the end.

use super::native::{fade, icon, r, ring, tone, z};
use super::*;

const CHEVRON_RIGHT: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/chevron-right.svg");
const CHEVRON_LEFT: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/chevron-left.svg");
const CHECK: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/check.svg");

/// How wide the panel is (designed px; drawn at the current zoom).
pub(super) const WIDTH: f32 = 244.0;
const ROW: f32 = 32.0;

/// A line of the menu that runs a command.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Entry {
    pub label: String,
    pub icon: Option<&'static [u8]>,
    pub command: (String, Value),
    /// A check mark: the choice that is on now.
    pub on: bool,
    /// Destructive (red text).
    pub danger: bool,
    pub disabled: bool,
    /// An agent's program (as `programs::startable` lists it): its icon leads the row instead of `icon`.
    pub lead: Option<Value>,
    /// The key (or note) shown dimmed at the right edge.
    pub keys: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Item {
    Entry(Entry),
    Sub { label: String, icon: Option<&'static [u8]>, items: Vec<Item> },
    /// A dimmed heading over the lines that follow (the cursor skips it).
    Head(String),
    Sep,
}

impl Item {
    pub fn run(label: impl Into<String>, icon: Option<&'static [u8]>, name: &str, args: Value) -> Self {
        Item::Entry(Entry { label: label.into(), icon, command: (name.to_owned(), args), on: false, danger: false, disabled: false, lead: None, keys: None })
    }
    pub fn sub(label: impl Into<String>, icon: Option<&'static [u8]>, items: Vec<Item>) -> Self {
        Item::Sub { label: label.into(), icon, items }
    }
    /// A dimmed heading line (not a choice: the cursor skips it).
    pub fn heading(label: impl Into<String>) -> Self {
        Item::Head(label.into())
    }
    /// The agent's icon leads the row.
    pub fn lead(mut self, program: Value) -> Self {
        if let Item::Entry(e) = &mut self {
            e.lead = Some(program);
        }
        self
    }
    /// A key or short note at the right edge.
    pub fn keys(mut self, text: impl Into<String>) -> Self {
        if let Item::Entry(e) = &mut self {
            e.keys = Some(text.into()).filter(|t| !t.is_empty());
        }
        self
    }
    pub fn disabled(mut self, off: bool) -> Self {
        if let Item::Entry(e) = &mut self {
            e.disabled = off;
        }
        self
    }
    pub fn danger(mut self) -> Self {
        if let Item::Entry(e) = &mut self {
            e.danger = true;
        }
        self
    }
    pub fn checked(mut self, on: bool) -> Self {
        if let Item::Entry(e) = &mut self {
            e.on = on;
        }
        self
    }
    /// Can the cursor stop here?
    fn stops(&self) -> bool {
        match self {
            Item::Entry(e) => !e.disabled,
            Item::Sub { .. } => true,
            Item::Sep | Item::Head(_) => false,
        }
    }
}

/// The items without a divider at either end or two in a row.
pub(super) fn tidy(items: Vec<Item>) -> Vec<Item> {
    let mut out: Vec<Item> = Vec::new();
    for item in items {
        match item {
            Item::Sep if matches!(out.last(), None | Some(Item::Sep)) => {}
            _ => out.push(item),
        }
    }
    while matches!(out.last(), Some(Item::Sep)) {
        out.pop();
    }
    out
}

/// Where the cursor goes from `from` by one step (`1` down, `-1` up), skipping dividers, headings and disabled
/// lines, and wrapping; `None` when nothing can be chosen.
pub(super) fn step(items: &[Item], from: usize, by: i32) -> Option<usize> {
    let n = items.len();
    (1..=n).map(|k| (from as i64 + by as i64 * k as i64).rem_euclid(n as i64) as usize).find(|&i| items[i].stops())
}

pub(super) fn first(items: &[Item]) -> Option<usize> {
    items.iter().position(Item::stops)
}

pub(super) fn last(items: &[Item]) -> Option<usize> {
    items.iter().rposition(Item::stops)
}

struct Level {
    title: Option<String>,
    items: Vec<Item>,
    cursor: usize,
}

pub(super) struct MenuView {
    app: WeakEntity<AppWindow>,
    id: String,
    levels: Vec<Level>,
    /// The keyboard has the cursor (shows the ring); the pointer moving gives it back.
    keys: bool,
    /// The pointer has moved over the menu (before that no row is tinted).
    moved: bool,
    focus: FocusHandle,
}

impl MenuView {
    fn new(app: WeakEntity<AppWindow>, id: String, items: Vec<Item>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let cursor = first(&items).unwrap_or(0);
        // Opened from the sidebar list's keys: the ring starts on the first line, and the list gets the keys back when it closes.
        let from_list = AppWindow::opened_from_list(&id);
        Self { app, id, levels: vec![Level { title: None, items, cursor }], keys: from_list, moved: false, focus }
    }

    fn level(&self) -> &Level {
        self.levels.last().expect("a menu always has its first level")
    }

    fn close(&self, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.id.clone();
        let _ = self.app.update(cx, |app, cx| app.close_overlay(&id, window, cx));
    }

    /// Opens the submenu at `at`, or runs the entry there.
    fn choose(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        match self.level().items.get(at).cloned() {
            Some(Item::Sub { label, items, .. }) => {
                let cursor = first(&items).unwrap_or(0);
                self.levels.push(Level { title: Some(label), items, cursor });
                cx.notify();
            }
            Some(Item::Entry(e)) if !e.disabled => {
                let (name, args) = e.command;
                let id = self.id.clone();
                let _ = self.app.update(cx, |app, cx| {
                    app.close_overlay(&id, window, cx);
                    app.run(&name, args, window, cx);
                });
            }
            _ => {}
        }
    }

    fn back(&mut self, cx: &mut Context<Self>) -> bool {
        if self.levels.len() < 2 {
            return false;
        }
        self.levels.pop();
        cx.notify();
        true
    }

    fn key(&mut self, e: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = e.keystroke.key.as_str();
        let (cursor, len) = {
            let l = self.level();
            (l.cursor, l.items.len())
        };
        let to = match key {
            "down" => step(&self.level().items, cursor, 1),
            "up" => step(&self.level().items, cursor, -1),
            "home" => first(&self.level().items),
            "end" => last(&self.level().items),
            "enter" | "space" => {
                self.keys = true;
                self.choose(cursor, window, cx);
                cx.stop_propagation();
                return;
            }
            "right" => {
                if matches!(self.level().items.get(cursor), Some(Item::Sub { .. })) {
                    self.keys = true;
                    self.choose(cursor, window, cx);
                }
                cx.stop_propagation();
                return;
            }
            "left" | "backspace" => {
                self.back(cx);
                cx.stop_propagation();
                return;
            }
            // Esc closes the menu (the window's own Esc binding may get there first); so does Tab, as in any menu: the keys never wander out of an open one.
            "escape" | "tab" => {
                self.close(window, cx);
                cx.stop_propagation();
                return;
            }
            // A line that shows a digit at its right edge (the new-tab menu's agents) answers to that digit.
            k if k.len() == 1 && k.as_bytes()[0].is_ascii_digit() && !e.keystroke.modifiers.control && !e.keystroke.modifiers.platform && !e.keystroke.modifiers.alt => {
                if let Some(at) = self.level().items.iter().position(|i| matches!(i, Item::Entry(en) if en.keys.as_deref() == Some(k) && !en.disabled)) {
                    self.choose(at, window, cx);
                    cx.stop_propagation();
                }
                return;
            }
            _ => return,
        };
        if let Some(to) = to.filter(|_| len > 0)
            && let Some(l) = self.levels.last_mut() {
                l.cursor = to;
            }
        self.keys = true;
        cx.stop_propagation();
        cx.notify();
    }

    fn row(&self, i: usize, item: &Item, cx: &mut Context<Self>) -> AnyElement {
        let level = self.level();
        let here = level.cursor == i;
        let (label, glyph, command_danger, disabled, on, sub, lead, keys) = match item {
            Item::Entry(e) => (e.label.clone(), e.icon, e.danger, e.disabled, e.on, false, e.lead.clone(), e.keys.clone()),
            Item::Sub { label, icon, .. } => (label.clone(), *icon, false, false, false, true, None, None),
            Item::Sep => return fade(4.0).my(z(4.0)).into_any_element(),
            Item::Head(text) => return div().px(z(8.0)).pt(z(8.0)).pb(z(4.0)).flex_none().text_size(z(11.0)).text_color(tone("muted")).child(text.clone()).into_any_element(),
        };
        let fg = if disabled { tone("dim") } else if command_danger { tone("blocked") } else { c("foreground") };
        let glyph_color = if command_danger { tone("blocked") } else { tone("muted") };
        ring(
            div()
                .id(SharedString::from(format!("ctx-{}-{i}", self.levels.len())))
                .h(z(ROW))
                .flex_none()
                .flex()
                .items_center()
                .px(z(8.0))
                .gap(z(9.0))
                .rounded(r(7.0))
                .text_size(z(12.5))
                .text_color(fg)
                .when(!disabled, |d| d.cursor_pointer())
                .when(here && !self.keys && self.moved, |d| d.bg(c("hover")))
                .when(!here, |d| d.hover(|s| s.bg(c("hover"))))
                .on_mouse_move(cx.listener(move |this, _: &MouseMoveEvent, _, cx| {
                    let moved = this.levels.last_mut().is_some_and(|l| std::mem::replace(&mut l.cursor, i) != i);
                    if moved || this.keys || !this.moved {
                        this.moved = true;
                        this.keys = false;
                        cx.notify();
                    }
                }))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.choose(i, window, cx))),
            here && self.keys,
        )
        .child(div().size(z(14.0)).flex_none().flex().items_center().justify_center().when_some(glyph.filter(|_| lead.is_none()), |d, g| d.child(icon(g, 14.0, glyph_color))).when_some(lead, |d, p| d.child(super::agents::program_glyph(&p, 14.0))))
        .child(
            div().flex_1().min_w(px(0.0)).child(div().truncate().child(label)),
        )
        .when_some(keys, |d, k| d.child(div().flex_none().text_size(z(11.0)).text_color(tone("dim")).child(k)))
        .when(on, |d| d.child(icon(CHECK, 14.0, c("accent"))))
        .when(sub, |d| d.child(icon(CHEVRON_RIGHT, 14.0, tone("dim"))))
        .into_any_element()
    }
}

impl Render for MenuView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let level = self.level();
        let rows: Vec<AnyElement> = level.items.clone().iter().enumerate().map(|(i, item)| self.row(i, item, cx)).collect();
        let head = level.title.clone().map(|title| {
            div()
                .id("ctx-back")
                .h(z(ROW))
                .flex_none()
                .flex()
                .items_center()
                .px(z(8.0))
                .gap(z(9.0))
                .rounded(r(7.0))
                .cursor_pointer()
                .text_size(z(12.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tone("muted"))
                .hover(|s| s.bg(c("hover")).text_color(c("foreground")))
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.back(cx);
                }))
                .child(icon(CHEVRON_LEFT, 14.0, tone("muted")))
                .child(div().flex_1().min_w(px(0.0)).truncate().child(title))
                .into_any_element()
        });
        div()
            .id("ctx-menu")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| this.key(e, window, cx)))
            .w_full()
            .min_h_0()
            .flex()
            .flex_col()
            .p(z(5.0))
            .text_color(c("foreground"))
            .when_some(head, |d, h| d.child(h).child(fade(4.0).my(z(4.0))))
            .child(div().id("ctx-list").flex().flex_col().max_h(z(520.0)).overflow_y_scroll().children(rows))
    }
}

impl AppWindow {
    /// Opens a context menu at the pointer: `items` as built for the thing that was clicked.
    pub(super) fn open_context_menu(&mut self, items: Vec<Item>, window: &mut Window, cx: &mut Context<Self>) {
        let items = tidy(items);
        if items.is_empty() {
            return;
        }
        // One at a time: a second right-click replaces the first.
        let stale: Vec<String> = self.overlays.iter().filter(|o| o.slot.starts_with("overlay:ctx")).map(|o| o.id.clone()).collect();
        for id in stale {
            self.close_overlay(&id, window, cx);
        }
        let id = format!("ctx{}", mods::new_overlay_id());
        let app = cx.weak_entity();
        let view = cx.new(|cx| MenuView::new(app, id.clone(), items, window, cx));
        let focus = cx.focus_handle();
        self.push_view_overlay(id, OverlayKind::Popup, size(WIDTH, 0.92), None, view.into(), focus, window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::{Item, first, last, step, tidy};
    use serde_json::json;

    fn go(label: &str) -> Item {
        Item::run(label, None, "x.y", json!({}))
    }
    fn labels(items: &[Item]) -> Vec<&str> {
        items.iter().map(|i| match i {
            Item::Entry(e) => e.label.as_str(),
            Item::Sub { label, .. } => label.as_str(),
            Item::Head(t) => t.as_str(),
            Item::Sep => "-",
        }).collect()
    }

    #[test]
    fn dividers_never_lead_trail_or_double_up() {
        let out = tidy(vec![Item::Sep, go("a"), Item::Sep, Item::Sep, go("b"), Item::Sep]);
        assert_eq!(labels(&out), ["a", "-", "b"]);
        assert!(tidy(vec![Item::Sep, Item::Sep]).is_empty());
    }

    #[test]
    fn the_cursor_skips_what_cannot_be_chosen_and_wraps() {
        let mut off = go("off");
        if let Item::Entry(e) = &mut off {
            e.disabled = true;
        }
        let items = vec![Item::Sep, go("a"), Item::Sep, off, Item::sub("s", None, vec![go("in")]), Item::Sep];
        assert_eq!(first(&items), Some(1));
        assert_eq!(last(&items), Some(4));
        assert_eq!(step(&items, 1, 1), Some(4));
        assert_eq!(step(&items, 4, 1), Some(1));
        assert_eq!(step(&items, 1, -1), Some(4));
        assert_eq!(step(&[Item::Sep], 0, 1), None);
        assert_eq!(step(&[], 0, 1), None);
    }

    #[test]
    fn a_checked_danger_entry_keeps_its_command() {
        let Item::Entry(e) = go("a").danger().checked(true) else { unreachable!() };
        assert!(e.danger && e.on);
        assert_eq!(e.command.0, "x.y");
    }
}
