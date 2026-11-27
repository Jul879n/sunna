//! Native layout blocks: the parts of the app that used to be shipped mods and
//! are now drawn in Rust. A block says `{"native": "<name>"}` (optionally with
//! `props`); an unknown name shows an empty block. Layouts written before this
//! (`{"mod": "workspace-status"}`, ...) get the same block through
//! `layout::native_alias`, unless the person keeps a mod folder of that name.
//!
//! Names: `status` (bottom bar), `brand` (top bar's left part), `sidebar-footer`
//! (Settings and Add machine under the sidebar), `welcome` (no space open), `tabs`
//! (the tab bar, in `tabs.rs`), `spaces` (the sidebar list), `files` and `changes` (the side
//! panels, in `files.rs` and `changes.rs`).
//! The next phases add theirs in `render_native`.

use super::*;
use crate::i18n;
use crate::layout::native_alias;
use std::sync::OnceLock;

const SIDEBAR: &str = "sidebar";
const BRAND: &str = "brand";
/// Below this width a sidebar is the retired rail.
const RAIL_BELOW: f32 = 160.0;
const SIDEBAR_PX: f32 = 268.0;

macro_rules! icon_file {
    ($name:literal) => {
        include_bytes!(concat!("../../defaults/mods/workspace-chrome/", $name, ".svg")).as_slice()
    };
}
const SERVER: &[u8] = icon_file!("server");
const MONITOR: &[u8] = icon_file!("monitor");
const BRANCH: &[u8] = icon_file!("branch");
const SEARCH: &[u8] = icon_file!("search");
const SIDEBAR_ICON: &[u8] = icon_file!("sidebar");
const SETTINGS: &[u8] = icon_file!("settings");
const PLUS: &[u8] = icon_file!("plus");
const HELP: &[u8] = include_bytes!("../../assets/icons/sidebar/help.svg").as_slice();
const COFFEE: &[u8] = include_bytes!("../../assets/icons/status/coffee.svg").as_slice();
const LOGO: &[u8] = include_bytes!("../../assets/sunna.png");

fn logo() -> Arc<Image> {
    static IMAGE: OnceLock<Arc<Image>> = OnceLock::new();
    IMAGE.get_or_init(|| Arc::new(Image::from_bytes(ImageFormat::Png, LOGO.to_vec()))).clone()
}

/// The theme's `style.radius` scale (6 is the original sizes), at the current zoom.
pub(super) fn r(n: f32) -> Pixels {
    settings::z((n * settings::style().radius.unwrap_or(6.0) / 6.0).round())
}

pub(super) fn z(n: f32) -> Pixels {
    settings::z(n)
}

/// An icon in `color` (an svg takes its own color, not its parent's).
pub(super) fn icon(bytes: &'static [u8], size: f32, color: Hsla) -> Svg {
    svg().data(bytes).size(z(size)).flex_none().text_color(color)
}

/// An icon that takes `over` while the element of `group` is hovered.
pub(super) fn icon_hover(bytes: &'static [u8], size: f32, color: Hsla, over: Hsla, group: &'static str) -> Svg {
    icon(bytes, size, color).group_hover(group, move |s| s.text_color(over))
}

/// A plain text tooltip.
struct Tip(SharedString);

impl Render for Tip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().px(z(8.0)).py(z(4.0)).rounded(r(6.0)).bg(c("raised")).border_1().border_color(c("border")).text_size(z(12.0)).text_color(c("foreground")).child(self.0.clone())
    }
}

pub(super) fn tip(text: String) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    move |_, cx| cx.new(|_| Tip(text.clone().into())).into()
}

// Pure rules (tested below) -------------------------------------------------

/// The native block a layout block stands for: `{"native": ..}` itself, or a
/// shipped mod name that went native, unless `has_mod` says the person keeps a
/// mod folder of that name.
pub(super) fn native_for(content: &Content, has_mod: impl Fn(&str) -> bool) -> Option<(String, Value)> {
    match content {
        Content::Native { native, props } => Some((native.clone(), props.clone())),
        Content::Mod { name, props } if !has_mod(name) => native_alias(name, props).map(|n| (n.to_owned(), props.clone())),
        _ => None,
    }
}

/// The top bar's brand has the sidebar's width, never less than what its content needs (the window
/// buttons on macOS, the logo, the name "Sunna", search and the toggle).
fn brand_width(macos: bool, wide: f32) -> f32 {
    wide.max(if macos { 236.0 } else { 168.0 })
}

/// Status bar items (Settings > Look): all on unless `status.items` says otherwise.
#[derive(Debug, PartialEq)]
struct StatusItems {
    machine: bool,
    branch: bool,
    agents: bool,
    awake: bool,
    hint: bool,
}

fn status_items(shared: Option<&Value>) -> StatusItems {
    let on = |k: &str| shared.and_then(|v| v.get(k)).and_then(Value::as_bool).unwrap_or(true);
    StatusItems { machine: on("machine"), branch: on("branch"), agents: on("agents"), awake: on("awake"), hint: on("hint") }
}

/// How a key chord reads in the status bar: `Ctrl+P`, `⌘K`.
fn palette_hint(bindings: &[(String, String, String)], mac: bool) -> Option<String> {
    let (_, keys, _) = bindings.iter().find(|(section, _, action)| section == "global" && action == "app.palette")?;
    let word = |p: &str| match (p.to_lowercase().as_str(), mac) {
        ("cmd", true) => "⌘".to_owned(),
        ("cmd", false) => "Ctrl+".to_owned(),
        ("ctrl", true) => "⌃".to_owned(),
        ("ctrl", false) => "Ctrl+".to_owned(),
        ("shift", true) => "⇧".to_owned(),
        ("shift", false) => "Shift+".to_owned(),
        ("alt" | "option", true) => "⌥".to_owned(),
        ("alt" | "option", false) => "Alt+".to_owned(),
        _ => p.to_uppercase(),
    };
    let parts: Vec<&str> = match keys.strip_suffix("--") {
        Some(head) => head.split('-').chain(["-"]).collect(),
        None => keys.split('-').collect(),
    };
    Some(parts.into_iter().map(word).collect())
}

/// What the welcome screen lists: the shortcuts read from the keys, never fixed.
const SHORTCUTS: [(&str, &str); 3] = [
    ("space.new", "welcome.empty_key_new"),
    ("space.prev", "welcome.empty_key_prev"),
    ("space.next", "welcome.empty_key_next"),
];

/// `cmd-shift-down` -> `["⌘", "⇧", "↓"]`; a plain letter is shown capitalized.
pub(super) fn key_parts(keys: &str) -> Vec<String> {
    keys.split('-')
        .filter(|k| !k.is_empty())
        .map(|k| match k {
            "cmd" => "⌘".into(),
            "shift" => "⇧".into(),
            "alt" => "⌥".into(),
            "ctrl" => "⌃".into(),
            "up" => "↑".into(),
            "down" => "↓".into(),
            "left" => "←".into(),
            "right" => "→".into(),
            "enter" => "↵".into(),
            "escape" => "Esc".into(),
            k if k.chars().count() == 1 => k.to_uppercase(),
            k => k.to_owned(),
        })
        .collect()
}

/// The first key bound to each action; unbound ones are left out.
fn shortcuts(bindings: &[(String, String, String)], label: impl Fn(&str) -> String) -> Vec<(String, Vec<String>)> {
    SHORTCUTS
        .iter()
        .filter_map(|(action, text)| {
            let (_, keys, _) = bindings.iter().find(|(_, _, a)| a == action)?;
            Some((label(text), key_parts(keys)))
        })
        .collect()
}

// Readable text ---------------------------------------------------------------------

pub(super) type Rgb = [f32; 3];

fn lum(c: Rgb) -> f32 {
    let l = |x: f32| if x <= 0.03928 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4) };
    0.2126 * l(c[0]) + 0.7152 * l(c[1]) + 0.0722 * l(c[2])
}

/// WCAG contrast ratio of two opaque colors.
pub(super) fn contrast(a: Rgb, b: Rgb) -> f32 {
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// `fg` moved toward `foreground` until it reads at `min`:1 on every fill; when even the foreground does
/// not (a theme whose text and background are close), toward black or white, whichever reads better.
pub(super) fn pull_readable(fg: Rgb, fills: &[Rgb], foreground: Rgb, min: f32) -> Rgb {
    let worst = |c: Rgb| fills.iter().map(|f| contrast(c, *f)).fold(f32::MAX, f32::min);
    if worst(fg) >= min {
        return fg;
    }
    let toward = if worst(foreground) >= min {
        foreground
    } else if worst([1.0; 3]) >= worst([0.0; 3]) {
        // Black or white, whichever reads better (a mid-tone fill that reads on neither the text nor the foreground).
        [1.0; 3]
    } else {
        [0.0; 3]
    };
    let mix = |t: f32| [0, 1, 2].map(|i| fg[i] + (toward[i] - fg[i]) * t);
    match (1..=20).map(|k| mix(k as f32 / 20.0)).find(|c| worst(*c) >= min) {
        Some(c) => c,
        // No single color reads on every fill (a theme whose selection is a dark bar on a light page): the page wins.
        None if fills.len() > 1 => pull_readable(fg, &fills[..1], foreground, min),
        None => toward,
    }
}

pub(super) fn opaque(c: Hsla, under: Rgb) -> Rgb {
    let r = Rgba::from(c);
    [r.r * r.a + under[0] * (1.0 - r.a), r.g * r.a + under[1] * (1.0 - r.a), r.b * r.a + under[2] * (1.0 - r.a)]
}

/// Every ground the app's text can sit on: the page, the bars, menus and tooltips, and the cards.
const GROUNDS: [&str; 4] = ["surface", "chrome", "popover", "raised"];

/// A theme color for text (a secondary grey, a status color) that reads at 4.5:1 on every ground the app
/// draws text on (the page, the bars, menus, cards) and on their hover and selected fills: the theme's own
/// color when it does, else a nearer-the-foreground one.
pub(super) fn tone(name: &str) -> Hsla {
    tone_over(name, &GROUNDS, 4.5)
}

/// `tone` for what the sidebar draws, which sits on `chrome` (a theme may make that very different from the page).
pub(super) fn side_tone(name: &str) -> Hsla {
    tone_over(name, &["chrome"], 4.5)
}

/// A color for lines and shapes on the sidebar (the lineage guides): 3:1 on its surface and row fills (WCAG 1.4.11).
pub(super) fn side_graphic(name: &str) -> Hsla {
    tone_over(name, &["chrome"], 3.0)
}

/// A color for a shape that carries meaning (an icon, an unchecked box, a switch knob): 3:1 on every ground (WCAG 1.4.11).
pub(super) fn graphic(name: &str) -> Hsla {
    tone_over(name, &GROUNDS, 3.0)
}

/// The text on an accent fill (a primary button, a badge, a checked mark): the theme's `accent_foreground`, nearer
/// the foreground (or black or white) when it would not read at 4.5:1 on the accent.
pub(super) fn on_accent() -> Hsla {
    let accent = opaque(c("accent"), [0.0; 3]);
    let out = pull_readable(opaque(c("accent_foreground"), accent), &[accent], opaque(c("foreground"), accent), 4.5);
    Rgba { r: out[0], g: out[1], b: out[2], a: 1.0 }.into()
}

/// The outline of a control that is only its outline (a text field, a switch track): the theme's `input` line,
/// nearer the text when it would not be seen (3:1 on every ground).
pub(super) fn outline() -> Hsla {
    tone_over("input", &GROUNDS, 3.0)
}

/// `name` readable at `min`:1 on each of `grounds` and their hover and selected fills; the first ground wins
/// when no single color can read on all.
fn tone_over(name: &str, grounds: &[&str], min: f32) -> Hsla {
    let fg = c(name);
    let base = opaque(c(grounds[0]), [0.0; 3]);
    let mut fills = Vec::new();
    for g in grounds {
        let bg = opaque(c(g), [0.0; 3]);
        fills.extend([bg, opaque(c("hover"), bg), opaque(c("selected"), bg)]);
    }
    let out = pull_readable(opaque(fg, base), &fills, opaque(c("foreground"), base), min);
    Rgba { r: out[0], g: out[1], b: out[2], a: 1.0 }.into()
}

/// Where a list's keyboard cursor goes on `key` (Up, Down, Home, End, PageUp, PageDown) in a list of `len` rows;
/// `None` for any other key or an empty list.
pub(super) fn step_cursor(cursor: usize, key: &str, len: usize) -> Option<usize> {
    let last = len.checked_sub(1)?;
    Some(match key {
        "down" => (cursor + 1).min(last),
        "up" => cursor.saturating_sub(1),
        "home" => 0,
        "end" => last,
        "pagedown" => (cursor + 8).min(last),
        "pageup" => cursor.saturating_sub(8),
        _ => return None,
    })
}

/// A button the keyboard reaches: a tab stop with the accent ring while focused; Enter or Space and a mouse click
/// press it (a keyboard click is skipped: the key already did).
pub(super) fn keyed(el: Stateful<Div>, ring: Hsla, press: impl Fn(&mut Window, &mut App) + 'static) -> Stateful<Div> {
    let press = Rc::new(press);
    let key = press.clone();
    el.focusable()
        .tab_stop(true)
        .focus_visible(move |s| s.border_2().border_color(ring))
        .on_key_down(move |e: &KeyDownEvent, window, cx| {
            if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                key(window, cx);
                cx.stop_propagation();
            }
        })
        .on_click(move |e, window, cx| {
            if !e.is_keyboard() {
                cx.stop_propagation();
                press(window, cx)
            }
        })
}

/// The ring a row with the keyboard cursor wears while its list has focus: a full 2 px accent border.
pub(super) fn ring(row: Stateful<Div>, on: bool) -> Stateful<Div> {
    row.border_2().border_color(if on { graphic("accent") } else { Hsla { a: 0.0, ..c("accent") } })
}

/// The alpha a divider peaks at. A faint theme border still needs a visible divider.
fn fade_alpha(border: f32, k: f32) -> f32 {
    let peak = if border < 1.0 { border.max(0x33 as f32 / 255.0) } else { 1.0 };
    (peak * k * 255.0).round() / 255.0
}

/// A 1 px line that fades in from nothing to the border's color at the middle and out again.
pub(super) fn fade(mx: f32) -> Div {
    let border = c("border");
    fade_line(mx, 1.0, border, fade_alpha(border.a, 1.0))
}

/// The same fade in `color` (peaking at `peak` alpha), `height` px thick.
fn fade_line(mx: f32, height: f32, color: Hsla, peak: f32) -> Div {
    let none = Hsla { a: 0.0, ..color };
    let peak = Hsla { a: peak, ..color };
    div()
        .h(px(height))
        .flex_none()
        .flex()
        .mx(z(mx))
        .my(z(6.0))
        .child(div().flex_1().h(px(height)).bg(linear_gradient(90.0, linear_color_stop(none, 0.0), linear_color_stop(peak, 1.0))))
        .child(div().flex_1().h(px(height)).bg(linear_gradient(90.0, linear_color_stop(peak, 0.0), linear_color_stop(none, 1.0))))
}

/// Where a dragged row will land: the accent fade (2 px) between two dots, laid over the row's top (`top`) or bottom edge.
pub(super) fn drop_line(top: bool) -> Div {
    let accent = c("accent");
    let dot = || div().size(px(6.0)).flex_none().rounded_full().bg(accent);
    div()
        .absolute()
        .left(px(0.0))
        .right(px(0.0))
        .h(px(6.0))
        .flex()
        .items_center()
        .when(top, |d| d.top(px(-3.0)))
        .when(!top, |d| d.bottom(px(-3.0)))
        .child(dot())
        .child(fade_line(0.0, 2.0, accent, 1.0).my(px(0.0)).flex_1())
        .child(dot())
}

/// The sidebar's one divider: the same line, 8 px from the sidebar's edges, wherever it sits (the blocks that hold it pad 8 px).
pub(super) fn sidebar_separator() -> Div {
    fade(0.0)
}

/// The key bindings, read again at most once a second (render must not touch the disk each frame).
fn bindings() -> Vec<(String, String, String)> {
    thread_local! {
        static CACHE: RefCell<Option<(Instant, Vec<(String, String, String)>)>> = const { RefCell::new(None) };
    }
    CACHE.with_borrow_mut(|cache| match cache {
        Some((at, list)) if at.elapsed() < Duration::from_secs(1) => list.clone(),
        _ => {
            let list = crate::keys::listing();
            *cache = Some((Instant::now(), list.clone()));
            list
        }
    })
}

/// The side panels' views: made once a layout block names them, kept while the window lives.
#[derive(Default)]
pub(super) struct Side {
    pub files: Option<Entity<super::files::FilesView>>,
    pub changes: Option<Entity<super::changes::ChangesView>>,
    pub search: Option<Entity<super::search::SearchView>>,
    pub activity: Option<Entity<super::activity::ActivityView>>,
    pub bots: Option<Entity<super::bots::BotsView>>,
}

// The window's side ---------------------------------------------------------

impl AppWindow {
    /// The native block a layout block stands for (see `native_for`).
    pub(super) fn native_of(&self, content: &Content) -> Option<(String, Value)> {
        native_for(content, |name| self.folders.contains_key(name))
    }

    /// Makes the Files and Changes views when the layout has a block for them, and tells them where the
    /// focused pane is (its folder and machine, and the space).
    pub(super) fn sync_panels(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let wants = |name: &str| self.chrome.walk().iter().any(|n| n.content().and_then(|c| self.native_of(c)).is_some_and(|(x, _)| x == name));
        let (files, changes, search) = (wants("files"), wants("changes"), wants("search"));
        let (activity, bots) = (wants("activity"), wants("bots"));
        if activity && self.side.activity.is_none() {
            let app = cx.weak_entity();
            self.side.activity = Some(cx.new(|cx| super::activity::ActivityView::new(app, window, cx)));
        }
        if bots && self.side.bots.is_none() {
            let app = cx.weak_entity();
            self.side.bots = Some(cx.new(|cx| super::bots::BotsView::new(app, window, cx)));
        }
        if !files && !changes && !search {
            return;
        }
        let (cwd, on) = self.goto_place();
        let place = super::files::Place { cwd, on, space: self.model.active_space().map(|s| s.id.clone()) };
        if files && self.side.files.is_none() {
            let app = cx.weak_entity();
            self.side.files = Some(cx.new(|cx| super::files::FilesView::new(app, cx)));
        }
        if changes && self.side.changes.is_none() {
            self.side.changes = Some(cx.new(|cx| super::changes::ChangesView::new(window, cx)));
        }
        if search && self.side.search.is_none() {
            let app = cx.weak_entity();
            self.side.search = Some(cx.new(|cx| super::search::SearchView::new(app, window, cx)));
        }
        if let Some(view) = &self.side.search {
            let place = place.clone();
            view.update(cx, |v, cx| v.set_place(place, cx));
        }
        if let Some(view) = &self.side.files {
            let place = place.clone();
            view.update(cx, |v, cx| v.set_place(place, cx));
        }
        if let Some(view) = &self.side.changes {
            view.update(cx, |v, cx| v.set_place(place, cx));
        }
    }

    pub(super) fn is_native(&self, content: &Content) -> bool {
        self.native_of(content).is_some()
    }

    pub(super) fn render_native(&self, name: &str, props: &Value, cx: &mut Context<Self>) -> AnyElement {
        match name {
            "status" => self.native_status(cx),
            "brand" => self.native_brand(cx),
            "sidebar-footer" => self.native_footer(cx),
            "welcome" => self.native_welcome(cx),
            "tabs" => self.native_tabs(cx),
            "spaces" => self.native_spaces(cx),
            "files" => self.side.files.clone().map_or_else(|| div().size_full().into_any_element(), |v| v.into_any_element()),
            "agents" => self.native_agents(cx),
            "activity" => self.side.activity.clone().map_or_else(|| div().size_full().into_any_element(), |v| v.into_any_element()),
            "peek" => self.native_peek(props, cx),
            "float-bar" => self.native_float_bar(cx),
            "bots" => self.side.bots.clone().map_or_else(|| div().size_full().into_any_element(), |v| v.into_any_element()),
            "search" => self.side.search.clone().map_or_else(|| div().size_full().into_any_element(), |v| v.into_any_element()),
            "changes" => self.side.changes.clone().map_or_else(|| div().size_full().into_any_element(), |v| v.into_any_element()),
            _ => div().size_full().into_any_element(),
        }
    }

    fn sidebar_px(&self) -> Option<f32> {
        match self.chrome.find(SIDEBAR)?.size {
            Some(Span::Px { px }) => Some(px),
            _ => None,
        }
    }

    /// Dragged (or saved by an older layout) narrower than this, the sidebar is a leftover of the retired rail.
    fn sidebar_too_narrow(&self) -> bool {
        self.sidebar_px().is_some_and(|px| px < RAIL_BELOW)
    }

    /// Shows or hides the whole sidebar (Orca's way): the work area takes the full width.
    fn toggle_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.run("layout.toggle", json!({ "id": SIDEBAR }), window, cx);
    }

    /// The width the sidebar has (or had, while hidden): what the brand keeps.
    fn sidebar_width(&self) -> f32 {
        match self.sidebar_px() {
            Some(px) if px >= RAIL_BELOW => px,
            _ => mods::shared("sidebar.width").and_then(|v| v.as_f64()).map_or(SIDEBAR_PX, |w| (w as f32).max(RAIL_BELOW)),
        }
    }

    /// The retired 44 px rail: a layout that still has the sidebar that narrow (or a saved
    /// `sidebar.collapsed`) gets its sidebar hidden, at the width it had. Keeps the width for the brand.
    pub(super) fn sync_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(sidebar) = self.chrome.find(SIDEBAR) else { return };
        let (hidden, too_narrow) = (sidebar.hidden, self.sidebar_too_narrow());
        let saved = mods::shared("sidebar.width").and_then(|v| v.as_f64()).map(|w| w as f32);
        if !hidden && !too_narrow && let Some(px) = self.sidebar_px() && saved != Some(px) {
            mods::set_shared("sidebar.width".into(), json!(px));
        }
        let old_flag = mods::shared("sidebar.collapsed").is_some_and(|v| v == true);
        if old_flag {
            mods::set_shared("sidebar.collapsed".into(), json!(false));
        }
        if !(too_narrow || old_flag) {
            return;
        }
        let width = saved.map_or(SIDEBAR_PX, |w| w.max(RAIL_BELOW));
        cx.defer_in(window, move |this, window, cx| {
            if this.sidebar_too_narrow() {
                this.run("layout.resize", json!({ "id": SIDEBAR, "px": width }), window, cx);
            }
            if this.chrome.find(SIDEBAR).is_some_and(|n| !n.hidden) {
                this.run("layout.hide", json!({ "id": SIDEBAR }), window, cx);
            }
        });
    }

    /// The top bar's brand keeps one width, the sidebar's, shown or hidden: the tabs never move.
    /// Only a layout whose `brand` block is the native one (the others set their own widths).
    pub(super) fn sync_brand(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync_sidebar(window, cx);
        let Some(brand) = self.chrome.find(BRAND) else { return };
        if brand.content().and_then(|c| self.native_of(c)).is_none_or(|(n, _)| n != BRAND) {
            return;
        }
        let px = brand_width(cfg!(target_os = "macos"), self.sidebar_width());
        if brand.size != Some(Span::Px { px }) {
            cx.defer_in(window, move |this, window, cx| this.run("layout.resize", json!({ "id": BRAND, "px": px }), window, cx));
        }
    }

    /// Agents shown (with a status), working, and blocked, as the agents list counts them.
    fn agent_counts(&self) -> (usize, usize, usize) {
        let m = &self.model;
        let mut counts = (0, 0, 0);
        for space in &m.spaces {
            for tab in &space.tabs {
                for pane in tab.root.panes() {
                    let Some(status) = pane.terminal().and_then(|a| m.status(a)) else { continue };
                    counts.0 += 1;
                    counts.1 += usize::from(status == "working");
                    counts.2 += usize::from(status == "blocked");
                }
            }
        }
        counts
    }

    pub(super) fn open_popup(&mut self, spec: Value, window: &mut Window, cx: &mut Context<Self>) {
        self.open_overlay(mods::new_overlay_id(), spec, window, cx);
    }

    // The bottom bar ---------------------------------------------------------

    fn native_status(&self, cx: &mut Context<Self>) -> AnyElement {
        let items = status_items(mods::shared("status.items").as_ref());
        let working_format = mods::shared("status.format").is_some_and(|v| v == "working");
        let (total, working, blocked) = self.agent_counts();
        let space = self.model.active_space();
        let machine = space.and_then(|p| p.machine.clone());
        let branch = space
            .and_then(|p| p.active_tab()?.focused_pane())
            .and_then(|pane| self.model.address_of(&pane))
            .and_then(|a| self.model.folder_info.get(&a)?.branch.clone());
        let label = |icon_bytes: &'static [u8], text: String| div().flex().items_center().gap(z(6.0)).child(icon(icon_bytes, 13.0, c("muted"))).child(text);
        let count = if working_format { working } else { total };
        let n = count.to_string();
        let agents_text = if working_format { i18n::tf("workspace-status.n_working", &[("n", &n)]) } else { i18n::tn("workspace-status.n_agents", count, &[("n", &n)]) };
        let awake_mode = settings::get().keep_awake;
        let awake_on = awake_mode == crate::awake::Mode::On || (awake_mode == crate::awake::Mode::Auto && working > 0);
        let hint = palette_hint(&bindings(), cfg!(target_os = "macos")).map_or_else(|| i18n::t("workspace-status.commands"), |key| i18n::tf("workspace-status.key_commands", &[("key", &key)]));
        div()
            .size_full()
            .flex()
            .items_center()
            .px(z(14.0))
            .gap(z(16.0))
            .bg(c("chrome"))
            .text_size(z(11.8))
            .text_color(c("muted"))
            .when(items.machine, |el| {
                let is_remote = machine.is_some();
                el.child(label(if is_remote { SERVER } else { MONITOR }, machine.clone().unwrap_or_else(|| i18n::t("workspace-status.this_computer"))))
            })
            .when(items.branch && branch.is_some(), |el| el.child(label(BRANCH, branch.clone().unwrap_or_default())))
            .when(items.agents, |el| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(z(6.0))
                        .child(div().size(z(6.0)).rounded(r(3.0)).bg(c("dim")))
                        .child(agents_text.clone())
                        .when(blocked > 0, |el| el.child(div().text_color(c("blocked")).child(i18n::tf("workspace-status.n_needs_you", &[("n", &blocked.to_string())])))),
                )
            })
            .child(div().flex_1())
            .when(items.awake && !cfg!(windows), |el| {
                el.child(
                    div()
                        .id("awake")
                        .flex()
                        .items_center()
                        .h(z(20.0))
                        .px(z(6.0))
                        .gap(z(5.0))
                        .rounded(r(5.0))
                        .bg(c(if awake_mode.is_off() { "chrome" } else { "selected" }))
                        .text_color(c(if awake_mode.is_off() { "dim" } else { "foreground" }))
                        .hover(|s| s.bg(c("hover")))
                        .tooltip(tip(i18n::t("awake.tooltip")))
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                            this.open_awake_menu(window, cx);
                        }))
                        .child(icon(COFFEE, 13.0, c(if awake_mode.is_off() { "dim" } else { "foreground" })))
                        .when(!awake_mode.is_off(), |el| {
                            el.child(i18n::t(if awake_mode == crate::awake::Mode::On { "awake.on" } else { "awake.auto" }))
                                .child(div().size(z(6.0)).rounded(r(3.0)).bg(c(if awake_on { "working" } else { "dim" })))
                        }),
                )
            })
            .when(items.hint, |el| {
                el.child(
                    div()
                        .id("palette")
                        .text_color(c("dim"))
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.run("app.palette", json!({}), window, cx)))
                        .child(hint.clone()),
                )
            })
            .into_any_element()
    }

    // The top bar's brand ----------------------------------------------------

    fn bar_button(&self, id: &'static str, bytes: &'static [u8], cx: &mut Context<Self>, action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static) -> Stateful<Div> {
        div()
            .id(id)
            .group(id)
            .size(z(26.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(r(6.0))
            .text_color(c("muted"))
            .hover(|s| s.bg(c("hover")).text_color(c("foreground")))
            .cursor_pointer()
            // The block drags the window: a button doesn't.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| action(this, window, cx)))
            .child(icon_hover(bytes, 15.0, c("muted"), c("foreground"), id))
    }

    fn native_brand(&self, cx: &mut Context<Self>) -> AnyElement {
        let mac = cfg!(target_os = "macos");
        let mark = || img(logo()).size(z(20.0)).rounded(r(5.0)).flex_none();
        // The same content folded or not, from the left: the buttons keep their place when the sidebar toggles.
        div()
            .size_full()
            .flex()
            .items_center()
            .bg(c("chrome"))
            .pl(z(14.0))
            .pr(z(10.0))
            .gap(z(4.0))
            .text_size(z(13.5))
            .when(mac && !self.fullscreen, |el| el.pl(z(84.0)))
            .child(div().flex().flex_none().items_center().gap(z(8.0)).mr(z(4.0)).child(mark()).child(div().flex_none().whitespace_nowrap().font_weight(FontWeight::SEMIBOLD).text_color(c("foreground")).child("Sunna")))
            .child(self.bar_button("search", SEARCH, cx, |this, window, cx| this.run("mod.open", json!({ "name": "picker" }), window, cx)))
            .child(self.bar_button("sidebar-toggle", SIDEBAR_ICON, cx, |this, window, cx| this.toggle_sidebar(window, cx)))
            .into_any_element()
    }

    // Under the sidebar ------------------------------------------------------

    fn native_footer(&self, cx: &mut Context<Self>) -> AnyElement {
        let row = |id: &'static str, bytes: &'static [u8], text: String, dim: bool, cx: &mut Context<Self>, action: Box<dyn Fn(&mut Self, &mut Window, &mut Context<Self>)>| {
            div()
                .id(id)
                .group(id)
                .flex()
                .items_center()
                .h(z(28.0))
                .px(z(8.0))
                .gap(z(9.0))
                .rounded(r(8.0))
                                .text_color(c(if dim { "dim" } else { "muted" }))
                .hover(|s| s.bg(c("hover")).text_color(c("foreground")))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| action(this, window, cx)))
                .child(icon_hover(bytes, 14.0, c(if dim { "dim" } else { "muted" }), c("foreground"), id))
                .child(text.clone())
        };
        let ready = div().size(z(6.0)).rounded_full().bg(c("accent"));
        let settings_row = row("settings", SETTINGS, i18n::t("workspace-chrome.settings"), false, cx, Box::new(|this, window, cx| this.run("app.settings", json!({}), window, cx)))
            .when(crate::update::ready().is_some(), |el| el.child(ready));
        let help_row = row("help", HELP, i18n::t("workspace-chrome.help"), false, cx, Box::new(|this, window, cx| this.run("app.help", json!({}), window, cx)));
        let machine_row = row("machine", PLUS, i18n::t("workspace-chrome.add_machine"), true, cx, Box::new(|this, window, cx| this.run("mod.open", json!({ "name": "machine-add" }), window, cx)));
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(c("chrome"))
            .px(z(8.0))
            .pt(z(4.0))
            .pb(z(8.0))
            .text_size(z(13.0))
            .child(sidebar_separator())
            .child(settings_row)
            .child(help_row)
            .child(machine_row)
            .into_any_element()
    }

    // No space open ----------------------------------------------------------

    fn welcome_button(&self, id: &'static str, text: String, primary: bool, cx: &mut Context<Self>, action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static) -> Stateful<Div> {
        div()
            .id(id)
            .flex()
            .items_center()
            .h(z(34.0))
            .px(z(16.0))
            .rounded(r(8.0))
            .border_1()
            .border_color(if primary { c("solid") } else { outline() })
            .bg(c(if primary { "solid" } else { "raised" }))
            .text_color(c(if primary { "solid_foreground" } else { "foreground" }))
            .text_size(z(12.8))
            .font_weight(FontWeight::MEDIUM)
            .cursor_pointer()
            .hover(|s| s.opacity(0.88))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| action(this, window, cx)))
            .child(text)
    }

    fn native_welcome(&self, cx: &mut Context<Self>) -> AnyElement {
        let list = shortcuts(&bindings(), i18n::t);
        let key_cap = |part: String| {
            div().min_w(z(22.0)).h(z(22.0)).px(z(6.0)).flex().items_center().justify_center().rounded(r(5.0)).border_1().border_color(c("border")).bg(c("raised")).text_size(z(11.5)).text_color(c("muted")).child(part)
        };
        let rows = list.into_iter().map(|(label, parts)| {
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(z(16.0))
                .child(div().flex_1().min_w(px(0.0)).text_size(z(12.5)).text_color(c("muted")).truncate().child(label))
                .child(div().flex().flex_none().gap(z(4.0)).children(parts.into_iter().map(key_cap)))
        });
        let rows: Vec<_> = rows.collect();
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(c("surface"))
            .rounded(r(10.0))
            .border_1()
            .border_color(c("border"))
            .text_color(c("foreground"))
            .child(
                div()
                    .w_full()
                    .max_w(z(380.0))
                    .p(z(24.0))
                    .flex()
                    .flex_col()
                    .gap(z(24.0))
                    .items_center()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(z(10.0))
                            .items_center()
                            .child(img(logo()).size(z(88.0)).rounded(r(20.0)))
                            .child(div().text_size(z(30.0)).font_weight(FontWeight::SEMIBOLD).child("Sunna"))
                            .child(div().text_size(z(13.0)).text_color(c("muted")).text_center().child(i18n::t("welcome.empty_subtitle"))),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(z(10.0))
                            .justify_center()
                            .child(self.welcome_button("wl-add", i18n::t("welcome.empty_add_project"), false, cx, |this, window, cx| {
                                this.open_add_project_menu(window, cx);
                            }))
                            .child(self.welcome_button("wl-new", i18n::t("welcome.empty_new_space"), true, cx, |this, window, cx| this.run("space.new", json!({}), window, cx))),
                    )
                    .when(!rows.is_empty(), |el| el.child(div().w_full().flex().flex_col().gap(z(10.0)).pt(z(4.0)).children(rows))),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{Content, Node, SHORTCUTS, StatusItems, Value, brand_width, fade_alpha, json, native_for, palette_hint, shortcuts, status_items};

    fn list(items: &[(&str, &str, &str)]) -> Vec<(String, String, String)> {
        items.iter().map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string())).collect()
    }

    fn en(key: &str) -> String {
        match key {
            "welcome.empty_key_new" => "Create workspace".into(),
            other => other.into(),
        }
    }

    #[test]
    fn a_native_block_parses_keeps_props_and_unknown_names_are_kept() {
        let c: Content = serde_json::from_str(r#"{"native":"status"}"#).unwrap();
        assert_eq!(c, Content::Native { native: "status".into(), props: Value::Null });
        let c: Content = serde_json::from_str(r#"{"native":"future-block","props":{"a":1}}"#).unwrap();
        assert_eq!(native_for(&c, |_| false), Some(("future-block".into(), serde_json::from_str::<Value>(r#"{"a":1}"#).unwrap())));
        // And it writes back as it came.
        assert_eq!(serde_json::to_value(&c).unwrap(), serde_json::from_str::<Value>(r#"{"native":"future-block","props":{"a":1}}"#).unwrap());
        // Old kinds still parse as they did.
        assert!(matches!(serde_json::from_str::<Content>(r#"{"mod":"tabs"}"#).unwrap(), Content::Mod { .. }));
        assert!(matches!(serde_json::from_str::<Content>(r#"{"terminal":"t"}"#).unwrap(), Content::Terminal { .. }));
    }

    #[test]
    fn the_keyboard_cursor_stays_inside_the_list() {
        use super::step_cursor;
        assert_eq!(step_cursor(0, "down", 3), Some(1));
        assert_eq!(step_cursor(2, "down", 3), Some(2));
        assert_eq!(step_cursor(0, "up", 3), Some(0));
        assert_eq!(step_cursor(1, "end", 3), Some(2));
        assert_eq!(step_cursor(2, "home", 3), Some(0));
        assert_eq!(step_cursor(0, "pagedown", 20), Some(8));
        assert_eq!(step_cursor(5, "pageup", 20), Some(0));
        assert_eq!(step_cursor(0, "down", 0), None);
        assert_eq!(step_cursor(0, "x", 3), None);
    }

    #[test]
    fn shipped_mod_names_that_went_native_still_work_in_old_layouts() {
        let mod_of = |text: &str| serde_json::from_str::<Content>(text).unwrap();
        let nobody = |_: &str| false;
        assert_eq!(native_for(&mod_of(r#"{"mod":"workspace-status"}"#), nobody).unwrap().0, "status");
        assert_eq!(native_for(&mod_of(r#"{"mod":"welcome"}"#), nobody).unwrap().0, "welcome");
        assert_eq!(native_for(&mod_of(r#"{"mod":"workspace-chrome"}"#), nobody).unwrap().0, "brand");
        assert_eq!(native_for(&mod_of(r#"{"mod":"workspace-chrome","props":{"footer":true}}"#), nobody).unwrap().0, "sidebar-footer");
        // Other mods stay mods; so does one the person keeps a folder for.
        assert_eq!(native_for(&mod_of(r#"{"mod":"tabs"}"#), nobody).unwrap().0, "tabs");
        assert_eq!(native_for(&mod_of(r#"{"mod":"tabs"}"#), |n| n == "tabs"), None);
        for name in ["files", "changes", "search", "agents", "activity", "bots"] {
            assert_eq!(native_for(&mod_of(&format!(r#"{{"mod":"{name}"}}"#)), nobody).unwrap().0, name);
            assert_eq!(native_for(&mod_of(&format!(r#"{{"mod":"{name}"}}"#)), |n| n == name), None);
        }
        assert_eq!(native_for(&mod_of(r#"{"mod":"worktree"}"#), nobody), None);
        assert_eq!(native_for(&mod_of(r#"{"mod":"workspace-status"}"#), |n| n == "workspace-status"), None);
        // A native block is never a mod folder's.
        assert!(native_for(&mod_of(r#"{"native":"status"}"#), |_| true).is_some());
    }

    #[test]
    fn the_shipped_layouts_use_native_blocks_where_the_mods_were() {
        let main: Node = serde_json::from_str(include_str!("../../defaults/layouts/main.json")).unwrap();
        for (id, name) in [("brand", "brand"), ("tabs", "tabs"), ("sidebar-footer", "sidebar-footer"), ("status", "status")] {
            let content = main.find(id).and_then(|n| n.content()).unwrap_or_else(|| panic!("{id} missing"));
            assert_eq!(native_for(content, |_| false).map(|n| n.0).as_deref(), Some(name), "{id}");
        }
    }

    #[test]
    fn the_brand_follows_the_sidebar_and_leaves_room_for_the_window_buttons() {
        assert_eq!(brand_width(true, 320.0), 320.0);
        assert_eq!(brand_width(true, 200.0), 236.0);
        assert_eq!(brand_width(false, 100.0), 168.0);
    }

    #[test]
    fn the_divider_fades_from_the_borders_own_alpha_and_never_vanishes_when_faint() {
        // A faint border is lifted to a visible floor (0x33); an opaque one peaks at 1.
        assert!((fade_alpha(0x24 as f32 / 255.0, 1.0) - 0x33 as f32 / 255.0).abs() < 0.003);
        assert!((fade_alpha(0x60 as f32 / 255.0, 1.0) - 0x60 as f32 / 255.0).abs() < 0.003);
        assert_eq!(fade_alpha(0.14, 0.0), 0.0);
        assert!((fade_alpha(1.0, 0.5) - 0.5).abs() < 0.003);
    }

    #[test]
    fn status_items_are_all_on_unless_the_setting_turns_them_off() {
        assert_eq!(status_items(None), StatusItems { machine: true, branch: true, agents: true, awake: true, hint: true });
        let off = json!({ "agents": false, "hint": false, "awake": false, "branch": false, "machine": false });
        assert_eq!(status_items(Some(&off)), StatusItems { machine: false, branch: false, agents: false, awake: false, hint: false });
        assert!(status_items(Some(&json!({ "branch": false }))).agents);
    }

    #[test]
    fn the_status_bar_shows_the_real_palette_key_or_none() {
        let bound = list(&[("global", "ctrl-p", "app.palette")]);
        assert_eq!(palette_hint(&bound, false).as_deref(), Some("Ctrl+P"));
        assert_eq!(palette_hint(&list(&[("global", "cmd-k", "app.palette")]), true).as_deref(), Some("⌘K"));
        assert_eq!(palette_hint(&list(&[("global", "ctrl-shift--", "app.palette")]), false).as_deref(), Some("Ctrl+Shift+-"));
        assert_eq!(palette_hint(&list(&[("global", "ctrl-k", "app.help")]), false), None);
        assert_eq!(palette_hint(&list(&[("space", "ctrl-p", "app.palette")]), false), None);
    }

    #[test]
    fn welcome_shortcuts_come_from_the_keys_in_order_with_readable_key_caps() {
        let bound = list(&[
            ("global", "cmd-n", "space.new"),
            ("global", "cmd-shift-up", "space.prev"),
            ("global", "cmd-shift-down", "space.next"),
            ("global", "cmd-t", "tab.new"),
        ]);
        let shown = shortcuts(&bound, en);
        assert_eq!(shown.len(), 3);
        assert_eq!(shown[0], ("Create workspace".to_owned(), vec!["⌘".to_owned(), "N".to_owned()]));
        assert_eq!(shown[2].1, ["⌘", "⇧", "↓"]);
    }

    #[test]
    fn changed_keys_are_shown_as_changed_and_unbound_actions_are_left_out() {
        let shown = shortcuts(&list(&[("global", "ctrl-alt-n", "space.new")]), en);
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].1, ["⌃", "⌥", "N"]);
        assert!(shortcuts(&[], en).is_empty());
    }

    #[test]
    fn every_text_the_native_blocks_show_exists_in_both_locales() {
        let keys = ["welcome.empty_subtitle", "welcome.empty_add_project", "welcome.empty_new_space", "welcome.empty_key_new", "welcome.empty_key_prev", "welcome.empty_key_next",
            "workspace-status.commands", "workspace-status.key_commands", "workspace-status.n_agents.one", "workspace-status.n_agents.other", "workspace-status.n_working", "workspace-status.n_needs_you",
            "workspace-status.this_computer", "workspace-chrome.settings", "workspace-chrome.help", "workspace-chrome.add_machine", "awake.tooltip", "awake.on", "awake.auto"];
        for lang in ["en", "es"] {
            let file = format!("{}/defaults/locales/{lang}.json", env!("CARGO_MANIFEST_DIR"));
            let table: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
            for k in keys.iter().chain(SHORTCUTS.iter().map(|(_, t)| t)) {
                assert!(table[k].as_str().is_some_and(|s| !s.is_empty()), "{lang}:{k}");
            }
        }
    }

    #[test]
    fn the_empty_state_wording_is_the_approved_spanish() {
        let file = format!("{}/defaults/locales/es.json", env!("CARGO_MANIFEST_DIR"));
        let es: Value = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
        assert_eq!(es["welcome.empty_subtitle"], "Seleccione un espacio de trabajo en la barra lateral para comenzar.");
        assert_eq!(es["welcome.empty_add_project"], "Agregar proyecto");
        assert_eq!(es["welcome.empty_new_space"], "Crear espacio de trabajo");
        assert_eq!(es["welcome.empty_changes"], "Seleccione un espacio de trabajo para ver los cambios");
    }

    #[test]
    fn what_the_sidebar_draws_reads_on_its_own_surface_in_light_dark_and_win98() {
        use super::{contrast, opaque, side_graphic, side_tone};
        use crate::window::c;
        for name in ["paper", "graphite", "win98"] {
            crate::settings::show_theme_for_test(name);
            let chrome = opaque(c("chrome"), [0.0; 3]);
            let fills = [chrome, opaque(c("hover"), chrome), opaque(c("selected"), chrome)];
            let worst = |col: gpui::Hsla| fills.iter().map(|f| contrast(opaque(col, *f), *f)).fold(f32::MAX, f32::min);
            // Text (counts, hints, headers) 4.5:1; the lineage guides, the drop line and its target ring 3:1.
            for tone in ["dim", "muted"] {
                assert!(worst(side_tone(tone)) >= 4.49, "{name}: text {tone} {}", worst(side_tone(tone)));
            }
            assert!(worst(side_graphic("dim")) >= 2.99, "{name}: guides {}", worst(side_graphic("dim")));
            assert!(worst(c("accent")) >= 3.0, "{name}: drop indicator {}", worst(c("accent")));
        }
    }

    /// The contrast audit: for every theme the app ships, the pairs the surfaces really draw. Text 4.5:1 (WCAG 1.4.3),
    /// shapes that carry meaning (rings, field outlines, guides, icons) 3:1 (1.4.11). One failure line per pair.
    #[test]
    fn every_shipped_theme_passes_the_ui_contrast_audit() {
        use super::{contrast, graphic, opaque, outline, side_graphic, side_tone, tone, Rgb};
        use crate::window::{c, dialog};
        let mut failures = Vec::new();
        for (name, theme) in crate::settings::shipped_themes_resolved() {
            crate::settings::show_theme_for_test(&name);
            let rgb = |n: &str| opaque(c(n), [0.0; 3]);
            let on = |col: gpui::Hsla, bg: Rgb| contrast(opaque(col, bg), bg);
            // What text can sit on: the page, the bars, the menus and tooltips (raised), each plain, hovered and selected.
            let mut grounds: Vec<(String, Rgb)> = Vec::new();
            for base in ["surface", "chrome", "popover", "raised"] {
                let b = rgb(base);
                grounds.push((base.to_owned(), b));
                grounds.push((format!("{base}+hover"), opaque(c("hover"), b)));
                grounds.push((format!("{base}+selected"), opaque(c("selected"), b)));
            }
            let mut check = |what: String, got: f32, min: f32| {
                if got < min - 0.005 {
                    failures.push(format!("{name}: {what} {got:.2} < {min}"));
                }
            };
            for (g, bg) in &grounds {
                check(format!("foreground on {g}"), on(c("foreground"), *bg), 4.5);
                // Text: the secondary greys and the status colors, as every surface draws them.
                for t in ["muted", "dim", "blocked", "done", "working", "accent"] {
                    check(format!("tone({t}) on {g}"), on(tone(t), *bg), 4.5);
                }
                // Shapes that carry meaning: icons, unchecked boxes, switch knobs, field and track outlines.
                for t in ["muted", "dim"] {
                    check(format!("graphic({t}) on {g}"), on(graphic(t), *bg), 3.0);
                }
                check(format!("outline on {g}"), on(outline(), *bg), 3.0);
                if g.starts_with("chrome") {
                    for t in ["muted", "dim", "blocked", "done", "working", "accent"] {
                        check(format!("side_tone({t}) on {g}"), on(side_tone(t), *bg), 4.5);
                    }
                    check(format!("side_graphic(dim) on {g}"), on(side_graphic("dim"), *bg), 3.0);
                }
                // The 2 px accent ring of a focused row or button, and the drop line.
                if g.starts_with("surface") || g.starts_with("chrome") {
                    check(format!("accent ring on {g}"), on(graphic("accent"), *bg), 3.0);
                }
            }
            // Primary controls: the accent fill with its own text.
            check("on_accent text on accent".into(), on(super::on_accent(), rgb("accent")), 4.5);
            // The dialog palette (buttons, fields, rings, outlines).
            let p = dialog::Palette::of(&theme);
            check("dialog title".into(), contrast(p.foreground, p.popover), 4.5);
            check("dialog description".into(), contrast(dialog::description(&p), p.popover), 4.5);
            check("dialog typed text".into(), contrast(dialog::field_text(&p), dialog::field_fill(&p)), 4.5);
            check("dialog selected text".into(), contrast(dialog::field_text(&p), dialog::field_selection(&p)), 4.5);
            // The selection must also show against the field itself (a block that vanishes into the fill is no selection).
            check("dialog selection against field".into(), contrast(dialog::field_selection(&p), dialog::field_fill(&p)), 1.25);
            check("dialog placeholder".into(), contrast(dialog::placeholder(&p), dialog::field_fill(&p)), 4.5);
            check("dialog field outline".into(), contrast(dialog::field_border(&p), p.popover), 3.0);
            check("dialog error outline".into(), contrast(dialog::error_border(&p), p.popover), 3.0);
            for tone_kind in [dialog::Tone::Neutral, dialog::Tone::Primary, dialog::Tone::Danger] {
                let look = dialog::paint(&p, tone_kind);
                check(format!("{tone_kind:?} button text"), contrast(look.text, look.fill), 4.5);
                check(format!("{tone_kind:?} focus ring"), contrast(dialog::ring_color(&p, tone_kind), p.popover), 3.0);
            }
        }
        assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
    }
}
