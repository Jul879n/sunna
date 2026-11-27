//! The tab bar (`{"native": "tabs"}`, once the `tabs` mod): the active space's
//! tabs, `+`, one button per side panel and, where the layout draws them, the
//! window buttons. Rules that are not drawing (the tab's caption, the wheel,
//! the recent-use order, the panel remembered per space) are plain functions
//! below, tested at the end.

use super::native::{self, icon, r, z};
use super::*;
use crate::i18n;
use crate::model::Space;
use std::collections::BTreeMap;

macro_rules! tab_icon {
    ($name:literal) => {
        include_bytes!(concat!("../../assets/icons/tabs/", $name, ".svg")).as_slice()
    };
}
const PANEL: &[u8] = tab_icon!("panel");
const WIN_MIN: &[u8] = tab_icon!("win-min");
const WIN_MAX: &[u8] = tab_icon!("win-max");
const WIN_RESTORE: &[u8] = tab_icon!("win-restore");
const WIN_CLOSE: &[u8] = tab_icon!("win-close");
const CLOSE: &[u8] = include_bytes!("../../assets/icons/pane/close.svg");
const SPINNER: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/spinner.svg");

/// The panels shown in the tab bar until the person picks (Settings > Layout > Sidebar).
const DEFAULT_PANELS: [&str; 3] = ["files", "changes", "search"];
/// A panel's title and icon come from its mod.json `panel`; these show until it is read.
const PANEL_FALLBACK: [(&str, &str, &str); 0] = [];
/// The panels that are native: title key and icon. A mod folder of the same name still wins.
const NATIVE_PANELS: [(&str, &str, &[u8]); 5] = [
    ("files", "tabs.files", include_bytes!("../../assets/icons/files/file.svg")),
    ("changes", "tabs.changes", include_bytes!("../../assets/icons/changes/changes.svg")),
    ("search", "tabs.search", include_bytes!("../../assets/icons/search/search.svg")),
    ("activity", "tabs.activity", include_bytes!("../../assets/icons/activity/activity.svg")),
    ("bots", "tabs.bots", include_bytes!("../../assets/icons/bots/bot.svg")),
];

// Pure rules ----------------------------------------------------------------

/// Drops a status glyph, keeps a leading bracket or quote (as the agents list does).
pub(super) fn clean(s: &str) -> String {
    let keep = |c: char| c.is_alphanumeric() || matches!(c, '(' | '[' | '{' | '\u{2018}' | '\u{201C}' | '\u{00AB}' | '"' | '\'');
    let s = s.trim_start_matches(|c: char| !keep(c));
    let s = s.strip_prefix("[ ! ] ").or_else(|| s.strip_prefix("[ . ] ")).unwrap_or(s);
    s.trim().to_owned()
}

/// What a tab's caption needs from its lead terminal.
#[derive(Clone, Debug, Default)]
struct Lead {
    program: Option<String>,
    key: String,
    agent: bool,
    title: String,
    terminal: String,
}

/// A tab Herdr named (unnamed, or after its program) follows its agent's live
/// title, as the agents list does; a name the person gave stays.
fn tab_label(name: &str, named: bool, lead: Option<&Lead>) -> String {
    let prog = lead.and_then(|l| l.program.as_deref());
    let base = if named { name.to_owned() } else { prog.unwrap_or("Terminal").to_owned() };
    let Some(lead) = lead.filter(|l| l.agent) else { return base };
    let live = clean(&lead.title);
    // clean() would strip a path's leading ~ or /: test the title with only its status glyph gone.
    let bare = lead.title.trim_start_matches(|c: char| !(c.is_alphanumeric() || c == '~' || c == '/'));
    let drive = {
        let b: Vec<char> = lead.title.chars().take(3).collect();
        b.len() == 3 && b[0].is_ascii_alphabetic() && b[1] == ':' && b[2] == '\\'
    };
    let path = bare.starts_with(['~', '/']) || drive;
    let telling = !live.is_empty() && live != lead.terminal && Some(live.as_str()) != prog && !path && !live.contains('@') && live.to_lowercase() != lead.key;
    if (Some(name) == prog || !named) && telling { live } else { base }
}

/// What the wheel asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Prev,
    Next,
}

/// A touchpad or smooth-scrolling mouse sends many small events per gesture: a
/// short swipe or one smoothed notch (about 25 px) switches one tab, a longer
/// swipe keeps going a tab per 60 px, at most one every 100 ms so each shows.
#[derive(Default)]
struct Swipe {
    acc: f32,
    steps: u32,
    at: u64,
    last: u64,
}

impl Swipe {
    fn feed(&mut self, dy: f32, started: bool, now: u64) -> Option<Step> {
        // Phases often arrive only as moves (a macOS touchpad included): a pause or a turn starts a new gesture.
        if started || now.saturating_sub(self.at) > 250 || dy * self.acc < 0.0 {
            self.acc = 0.0;
            self.steps = 0;
        }
        self.at = now;
        self.acc += dy;
        let need = if self.steps > 0 { 60.0 } else { 20.0 };
        if self.acc.abs() < need || now.saturating_sub(self.last) < 100 {
            return None;
        }
        self.acc -= self.acc.signum() * need;
        self.steps += 1;
        self.last = now;
        Some(if dy > 0.0 { Step::Prev } else { Step::Next })
    }
}

/// Tabs walked in the order they were last used (Settings > Look > Tab order), as one gesture walks back through them.
#[derive(Default)]
struct Cycle {
    list: Vec<String>,
    i: usize,
    at: u64,
}

const CYCLE_MS: u64 = 1500;

/// The tab a step lands on when walking by recent use, or `None` (the end).
fn recent_step(cycle: &mut Option<Cycle>, recent: &[String], ids: &[String], step: Step, now: u64) -> Option<String> {
    if cycle.as_ref().is_none_or(|c| now.saturating_sub(c.at) > CYCLE_MS) {
        let mut list: Vec<String> = recent.iter().filter(|id| ids.contains(id)).cloned().collect();
        list.extend(ids.iter().filter(|id| !recent.contains(id)).cloned());
        *cycle = Some(Cycle { list, i: 0, at: now });
    }
    let c = cycle.as_mut()?;
    c.at = now;
    let next = match step {
        Step::Prev => c.i.saturating_sub(1),
        Step::Next => (c.i + 1).min(c.list.len().saturating_sub(1)),
    };
    if next == c.i {
        return None;
    }
    c.i = next;
    c.list.get(next).cloned()
}

/// The side panel is remembered per space: switching shows that space's choice
/// (closed if it never had one), and opening or closing it records it.
#[derive(Default)]
struct PanelSync {
    panels: BTreeMap<String, Option<String>>,
    /// `None` until the first space is seen.
    space: Option<String>,
    pending: Option<(Option<String>, u64)>,
}

#[derive(Debug, PartialEq)]
struct PanelOut {
    /// Show this child (or close the panel when `None`).
    restore: Option<Option<String>>,
    save: bool,
}

impl PanelSync {
    fn sync(&mut self, space: &str, now_panel: Option<&str>, enabled: &[String], now: u64) -> PanelOut {
        let now_panel = now_panel.map(str::to_owned);
        if self.space.as_deref() != Some(space) {
            let first = self.space.is_none();
            self.space = Some(space.to_owned());
            self.pending = None;
            if !first || self.panels.contains_key(space) {
                let stored = self.panels.get(space).cloned().flatten();
                let want = stored.filter(|s| enabled.contains(s));
                if want != now_panel {
                    self.pending = Some((want.clone(), now + 1000));
                    return PanelOut { restore: Some(want), save: false };
                }
            }
        }
        // Until the restore lands (or something else won), the old state isn't a choice.
        if let Some((want, until)) = &self.pending {
            if *want != now_panel && now < *until {
                return PanelOut { restore: None, save: false };
            }
            self.pending = None;
        }
        if self.panels.get(space) == Some(&now_panel) {
            return PanelOut { restore: None, save: false };
        }
        self.panels.insert(space.to_owned(), now_panel);
        PanelOut { restore: None, save: true }
    }
}

/// The saved panel map; anything else (a hand-edited or old file) starts empty.
fn parse_panels(raw: Option<&str>) -> BTreeMap<String, Option<String>> {
    let Some(Value::Object(map)) = raw.and_then(|r| serde_json::from_str(r).ok()) else { return BTreeMap::new() };
    map.into_iter().map(|(k, v)| (k, v.as_str().map(str::to_owned))).collect()
}

fn store_path() -> PathBuf {
    mods::app_dir().join("data").join("tabs").join("store.json")
}

fn load_panels() -> BTreeMap<String, Option<String>> {
    let file: Value = std::fs::read_to_string(store_path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null);
    parse_panels(file.get("panels").and_then(Value::as_str))
}

/// Same file and shape the `tabs` mod kept (`panels`: a JSON string), so what a person had carries over.
fn save_panels(panels: &BTreeMap<String, Option<String>>) {
    let path = store_path();
    let mut file = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).filter(Value::is_object).unwrap_or_else(|| json!({}));
    file["panels"] = json!(serde_json::to_string(panels).unwrap_or_default());
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, serde_json::to_string_pretty(&file).unwrap_or_default());
}

fn now_ms() -> u64 {
    model::now_ms()
}

// State between frames --------------------------------------------------------

struct State {
    pressed: Option<usize>,
    swipe: Swipe,
    cycle: Option<Cycle>,
    recent: Vec<String>,
    ids: Vec<String>,
    panels: PanelSync,
    /// `mod` -> its mod.json `panel` (read once).
    info: HashMap<String, (Option<String>, Option<String>)>,
    scroll: ScrollHandle,
    selected: Option<(String, usize)>,
    buttons: bool,
    maximized: bool,
    windows: bool,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
        pressed: None,
        swipe: Swipe::default(),
        cycle: None,
        recent: Vec::new(),
        ids: Vec::new(),
        panels: PanelSync { panels: load_panels(), ..PanelSync::default() },
        info: HashMap::new(),
        scroll: ScrollHandle::new(),
        selected: None,
        buttons: false,
        maximized: false,
        windows: cfg!(windows),
    });
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with_borrow_mut(f)
}

fn panel_info(module: &str) -> (Option<String>, Option<String>) {
    if let Some(found) = with(|s| s.info.get(module).cloned()) {
        return found;
    }
    let raw = std::fs::read_to_string(mods::app_dir().join("mods").join(module).join("mod.json")).ok();
    let panel = raw.and_then(|r| serde_json::from_str::<Value>(&r).ok()).and_then(|v| v.get("panel").cloned()).unwrap_or(Value::Null);
    let text = |k: &str| panel.get(k).and_then(Value::as_str).map(str::to_owned);
    let info = (text("title"), text("icon"));
    with(|s| s.info.insert(module.to_owned(), info.clone()));
    info
}

fn text_of(key: &str) -> String {
    let t = i18n::t(key);
    if t == key { key.to_owned() } else { t }
}

fn enabled_panels() -> Vec<String> {
    match mods::shared("workspace.panels") {
        Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect(),
        _ => DEFAULT_PANELS.iter().map(|s| (*s).to_owned()).collect(),
    }
}

/// The stack of panels (`panel-body`) and whether the `panel` node is hidden.
fn panel_state(chrome: &Node) -> Option<(bool, Option<String>, Vec<(String, String)>)> {
    let panel = chrome.find("panel")?;
    let body = chrome.find("panel-body")?;
    let (active, children) = match &body.kind {
        Kind::Stack { children, active, .. } => (active.clone(), children),
        _ => return None,
    };
    let kids = children
        .iter()
        .map(|c| {
            let module = match c.content() {
                Some(Content::Mod { name, .. }) => name.clone(),
                _ => c.id.clone(),
            };
            (c.id.clone(), module)
        })
        .collect();
    Some((panel.hidden, active, kids))
}

// The window's side -------------------------------------------------------------

impl AppWindow {
    fn has_native_tabs(&self) -> bool {
        self.chrome.visible_blocks().iter().any(|n| n.content().and_then(|c| self.native_of(c)).is_some_and(|(name, _)| name == "tabs"))
    }

    /// Once a frame: what the bar remembers (recent tabs, the window's buttons) and the panel each space keeps.
    pub(super) fn sync_tabs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_native_tabs() {
            return;
        }
        let w = self.window_json(window);
        with(|s| {
            s.buttons = w["buttons"].as_bool().unwrap_or(false);
            s.maximized = w["maximized"].as_bool().unwrap_or(false);
            s.windows = w["platform"] == "windows";
        });
        let now = now_ms();
        let selected = self.model.active_space().and_then(|p| p.active_tab()).map(|t| t.id.clone());
        let ids: Vec<String> = self.model.active_space().map(|p| p.tabs.iter().map(|t| t.id.clone()).collect()).unwrap_or_default();
        with(|s| {
            s.ids = ids;
            if let Some(id) = &selected
                && s.recent.first() != Some(id)
                && s.cycle.as_ref().is_none_or(|c| now.saturating_sub(c.at) >= CYCLE_MS)
            {
                s.recent.retain(|x| x != id);
                s.recent.insert(0, id.clone());
                s.recent.truncate(100);
            }
        });
        let (Some(space), Some((hidden, active, _))) = (self.model.active_space().map(|p| p.id.clone()), panel_state(&self.chrome)) else { return };
        let enabled = enabled_panels();
        let shown = if hidden { None } else { active.as_deref() };
        let out = with(|s| s.panels.sync(&space, shown, &enabled, now));
        if out.save {
            with(|s| save_panels(&s.panels.panels));
        }
        if let Some(want) = out.restore {
            cx.defer_in(window, move |this, window, cx| match want {
                Some(child) => {
                    this.run("layout.select", json!({ "stack": "panel-body", "child": child }), window, cx);
                    this.run("layout.show", json!({ "id": "panel" }), window, cx);
                }
                None => this.run("layout.hide", json!({ "id": "panel" }), window, cx),
            });
        }
    }

    /// One step to the previous or next tab: by position, or by recent use when asked.
    fn tab_go(&mut self, step: Step, window: &mut Window, cx: &mut Context<Self>) {
        let command = if step == Step::Prev { "tab.prev" } else { "tab.next" };
        let recent = mods::shared("tabs.order").is_some_and(|v| v == "recent");
        let target = with(|s| {
            if !recent || s.ids.is_empty() {
                return Err(());
            }
            let (ids, rec) = (s.ids.clone(), s.recent.clone());
            Ok(recent_step(&mut s.cycle, &rec, &ids, step, now_ms()))
        });
        match target {
            Err(()) => self.run(command, json!({}), window, cx),
            Ok(Some(id)) => self.run("tab.select", json!({ "id": id }), window, cx),
            Ok(None) => {}
        }
    }

    fn tab_wheel(&mut self, ev: &ScrollWheelEvent, window: &mut Window, cx: &mut Context<Self>) {
        let step = match ev.delta {
            ScrollDelta::Lines(l) => (l.y != 0.0).then_some(if l.y > 0.0 { Step::Prev } else { Step::Next }),
            ScrollDelta::Pixels(p) => {
                let dy: f32 = p.y.into();
                with(|s| s.swipe.feed(dy, ev.touch_phase == TouchPhase::Started, now_ms()))
            }
        };
        if let Some(step) = step {
            self.tab_go(step, window, cx);
        }
    }

    /// The caption inputs of a tab: its lead terminal (an agent's if any).
    fn tab_lead(&self, tab: &Tab, cx: &mut Context<Self>) -> (Option<Lead>, Value) {
        let mut first: Option<(Option<Lead>, Value)> = None;
        for pane in tab.root.panes() {
            let Some(addr) = pane.terminal() else {
                first.get_or_insert((None, Value::Null));
                continue;
            };
            let program = self.program(addr);
            let title = self.terms.get(&pane.id).map(|t| t.view.read(cx).title.clone()).or_else(|| self.live.get(addr).and_then(|r| r.get("title")).and_then(Value::as_str).map(str::to_owned)).unwrap_or_default();
            let lead = Lead {
                program: program.get("name").and_then(Value::as_str).map(str::to_owned),
                key: program.get("key").and_then(Value::as_str).unwrap_or_default().to_owned(),
                agent: program.get("agent").and_then(Value::as_bool).unwrap_or(false),
                title,
                terminal: addr.to_owned(),
            };
            if lead.agent {
                return (Some(lead), program);
            }
            first.get_or_insert((Some(lead), program));
        }
        first.unwrap_or((None, Value::Null))
    }

    fn tab_button(&self, id: &'static str, cx: &mut Context<Self>, action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static) -> Stateful<Div> {
        let ring = c("accent");
        let app = cx.entity();
        native::keyed(
            div()
                .id(id)
                .size(z(32.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(r(6.0))
                .border_2()
                .border_color(Hsla { a: 0.0, ..ring })
                .text_color(tone("dim"))
                .hover(|s| s.bg(c("hover")).text_color(c("foreground")))
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()),
            ring,
            move |window, cx| app.update(cx, |this, cx| action(this, window, cx)),
        )
    }

    fn tab_item(&self, space: &Space, tab: &Tab, i: usize, unsaved: &HashSet<String>, cx: &mut Context<Self>) -> Stateful<Div> {
        let m = &self.model;
        let (lead, program) = self.tab_lead(tab, cx);
        let name = tab_label(&Model::tab_name(space, tab), !tab.name.is_empty(), lead.as_ref());
        let name = if tab.zoomed.is_some() { format!("{name} Z") } else { name };
        let status = m.rollup(&model::addresses(&tab.root));
        let dirty = tab.root.panes().iter().any(|n| unsaved.contains(&n.id));
        let active = space.active_tab().is_some_and(|a| a.id == tab.id);
        let (id, space_id) = (tab.id.clone(), space.id.clone());
        let (down_id, up_id, menu_id, close_id) = (id.clone(), id.clone(), id.clone(), id.clone());
        let icon_path = program.get("icon").and_then(Value::as_str).map(|p| mods::app_dir().join(p));
        let tint = program.get("tint").and_then(Value::as_bool).unwrap_or(true);
        div()
            .id(SharedString::from(format!("tab-{id}")))
            .flex()
            .items_center()
            .flex_none()
            .h(z(29.0))
            .px(z(10.0))
            .gap(z(7.0))
            .rounded(r(7.0))
            .text_size(z(12.8))
            .border_1()
            .border_color(c(if active { "border" } else { "chrome" }))
            .bg(c(if active { "raised" } else { "chrome" }))
            .text_color(c(if active { "foreground" } else { "muted" }))
            .hover(|s| s.bg(c("hover")))
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, ev: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                with(|s| s.pressed = Some(i));
                if ev.click_count == 2 {
                    this.run("tab.rename", json!({ "id": down_id }), window, cx);
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(move |this, _: &MouseUpEvent, window, cx| {
                let from = with(|s| s.pressed.take());
                match from {
                    Some(from) if from != i => {
                        let moved = this.model.active_space().and_then(|p| p.tabs.get(from).map(|t| t.id.clone()));
                        if let Some(moved) = moved {
                            this.run("tab.move", json!({ "id": moved, "index": i + 1 }), window, cx);
                        }
                    }
                    _ => this.run("tab.select", json!({ "id": up_id }), window, cx),
                }
            }))
            .on_mouse_down(MouseButton::Right, cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                this.open_native_menu("tab", &json!({ "id": menu_id, "space": space_id }), window, cx);
            }))
            .when_some(icon_path, |el, path| {
                if tint {
                    el.child(svg().external_path(path.to_string_lossy().into_owned()).size(z(14.0)).flex_none().text_color(tone("muted")))
                } else {
                    el.child(img(path).size(z(14.0)).flex_none())
                }
            })
            .child(div().id("tab-name").max_w(z(170.0)).truncate().tooltip(native::tip(name.to_string())).child(name))
            // A file edited in this tab and not saved.
            .when(dirty, |el| el.child(div().size(z(7.0)).flex_none().rounded(r(4.0)).bg(c("accent"))))
            .when(status == Some("working"), |el| {
                el.child(icon(SPINNER, 12.0, c("working")).with_animation(
                    SharedString::from(format!("spin-{id}")),
                    Animation::new(Duration::from_millis(900)).repeat(),
                    |s, delta| s.with_transformation(Transformation::rotate(percentage(delta))),
                ))
            })
            .when(matches!(status, Some("blocked" | "done")), |el| el.child(div().size(z(6.0)).flex_none().rounded(r(3.0)).bg(c(status.unwrap_or("done")))))
            .when(active, |el| {
                el.child(
                    div()
                        .id(SharedString::from(format!("close-{close_id}")))
                        .w(z(32.0))
                        .h_full()
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(tone("dim"))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            cx.stop_propagation();
                            this.run("tab.close", json!({ "id": close_id }), window, cx);
                        }))
                        .child(icon(CLOSE, 12.0, graphic("dim"))),
                )
            })
    }

    fn panel_button(&self, id: String, label: String, icon_el: AnyElement, on: bool, cx: &mut Context<Self>) -> Stateful<Div> {
        let key = id.clone();
        let ring = c("accent");
        let app = cx.entity();
        native::keyed(
            div()
                .id(SharedString::from(format!("panel-{id}")))
                .flex()
                .items_center()
                .flex_none()
                .h(z(32.0))
                .px(z(8.0))
                .gap(z(5.0))
                .rounded(r(6.0))
                .border_2()
                .border_color(Hsla { a: 0.0, ..ring })
                .text_size(z(12.0))
                .bg(c(if on { "selected" } else { "chrome" }))
                .text_color(c(if on { "foreground" } else { "muted" }))
                .hover(|s| s.bg(c("hover")))
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()),
            ring,
            move |window, cx| {
                app.update(cx, |this, cx| {
                    if on {
                        this.run("layout.hide", json!({ "id": "panel" }), window, cx);
                    } else {
                        this.run("layout.select", json!({ "stack": "panel-body", "child": key.clone() }), window, cx);
                        this.run("layout.show", json!({ "id": "panel" }), window, cx);
                    }
                })
            },
        )
        .child(icon_el)
        .child(label)
    }

    pub(super) fn native_tabs(&self, cx: &mut Context<Self>) -> AnyElement {
        let space = self.model.active_space();
        let unsaved = self.dirty_panes();
        let active = space.and_then(|p| { let a = p.active_tab()?; p.tabs.iter().position(|t| t.id == a.id) });
        let selected = active.and_then(|i| space.map(|p| (p.tabs[i].id.clone(), i)));
        let scroll = with(|s| {
            if selected != s.selected {
                s.selected = selected.clone();
                if let Some((_, i)) = &selected {
                    s.scroll.scroll_to_item(*i);
                }
            }
            s.scroll.clone()
        });
        let items: Vec<Stateful<Div>> = space.map(|p| p.tabs.iter().enumerate().map(|(i, t)| self.tab_item(p, t, i, &unsaved, cx)).collect()).unwrap_or_default();

        let (hidden, body_active, kids) = panel_state(&self.chrome).map_or((true, None, Vec::new()), |p| p);
        let enabled = enabled_panels();
        let buttons: Vec<Stateful<Div>> = kids
            .into_iter()
            .filter(|(id, _)| enabled.contains(id))
            .map(|(id, module)| {
                let native = NATIVE_PANELS.iter().find(|(k, _, _)| *k == module && !self.folders.contains_key(&module));
                let (title, icon_path) = if native.is_some() { (None, None) } else { panel_info(&module) };
                let fallback = PANEL_FALLBACK.iter().find(|(k, _, _)| *k == id);
                let title = text_of(&title.or_else(|| native.map(|n| n.1.to_owned())).unwrap_or_else(|| fallback.map_or(id.clone(), |f| f.1.to_owned())));
                let on = !hidden && body_active.as_deref() == Some(id.as_str());
                let tone = c(if on { "foreground" } else { "muted" });
                let icon_rel = icon_path.or_else(|| fallback.map(|f| f.2.to_owned()));
                let icon_el = match icon_rel {
                    _ if native.is_some() => svg().data(native.map_or(PANEL, |n| n.2)).size(z(14.0)).flex_none().text_color(tone).into_any_element(),
                    Some(rel) => svg().external_path(mods::app_dir().join(rel).to_string_lossy().into_owned()).size(z(14.0)).flex_none().text_color(tone).into_any_element(),
                    None => svg().data(PANEL).size(z(14.0)).flex_none().text_color(tone).into_any_element(),
                };
                self.panel_button(id, title, icon_el, on, cx)
            })
            .collect();

        let (win_buttons, maximized, windows) = with(|s| (s.buttons, s.maximized, s.windows));
        let glyph = |text: &'static str, bytes: &'static [u8], size: f32| -> AnyElement {
            if windows { icon(bytes, size, c("dim")).into_any_element() } else { div().child(text).into_any_element() }
        };
        div()
            .size_full()
            .min_w(px(0.0))
            .flex()
            .items_center()
            .px(z(10.0))
            .gap(z(3.0))
            .bg(c("chrome"))
            .on_mouse_up(MouseButton::Left, |_, _, _| with(|s| s.pressed = None))
            .on_scroll_wheel(cx.listener(|this, ev: &ScrollWheelEvent, window, cx| this.tab_wheel(ev, window, cx)))
            .child(
                div()
                    .id("tab-scroll")
                    .flex()
                    .flex_1()
                    .min_w(px(0.0))
                    .h_full()
                    .items_center()
                    .gap(z(3.0))
                    .overflow_x_scroll()
                    .track_scroll(&scroll)
                    .children(items),
            )
            .child(self.tab_button("tab-new", cx, |this, window, cx| this.open_new_menu(window, cx)).tooltip(native::tip(i18n::t("menu.new_tab"))).child("+"))
            .children(buttons)
            .when(win_buttons, |el| {
                el.child(self.tab_button("minimize", cx, |this, window, cx| this.run("window.minimize", json!({}), window, cx)).child(glyph("\u{2212}", WIN_MIN, 16.0)))
                    .child(self.tab_button("maximize", cx, |this, window, cx| this.run("window.maximize", json!({}), window, cx)).child(glyph("\u{25A1}", if maximized { WIN_RESTORE } else { WIN_MAX }, 16.0)))
                    .child(self.tab_button("close-window", cx, |this, window, cx| this.run("window.close", json!({}), window, cx)).child(if windows { icon(WIN_CLOSE, 16.0, graphic("dim")) } else { icon(CLOSE, 13.0, graphic("dim")) }))
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{BTreeMap, CYCLE_MS, Lead, PanelOut, PanelSync, Step, Swipe, clean, parse_panels, recent_step, tab_label};

    fn claude(title: &str) -> Lead {
        Lead { program: Some("Claude Code".into()), key: "claude".into(), agent: true, title: title.into(), terminal: String::new() }
    }

    #[test]
    fn a_tab_herdr_named_after_its_agent_follows_the_agents_session_title() {
        assert_eq!(tab_label("Claude Code", true, Some(&claude("my-task"))), "my-task");
        assert_eq!(tab_label("", false, Some(&claude("my-task"))), "my-task");
        assert_eq!(tab_label("", false, Some(&claude("\u{2726} [web] login"))), "[web] login");
        assert_eq!(tab_label("Claude Code", true, Some(&claude("renamed-task"))), "renamed-task");
    }

    #[test]
    fn an_empty_title_published_as_the_terminal_address_keeps_the_program_label() {
        for terminal in ["t2", "remote/t2"] {
            let l = Lead { terminal: terminal.into(), ..claude(terminal) };
            assert_eq!(tab_label("2", false, Some(&l)), "Claude Code");
            assert_eq!(tab_label("Claude Code", true, Some(&l)), "Claude Code");
            assert_eq!(tab_label("fix login", true, Some(&l)), "fix login");
            let renamed = Lead { title: "renamed-task".into(), ..l };
            assert_eq!(tab_label("2", false, Some(&renamed)), "renamed-task");
        }
    }

    #[test]
    fn a_name_the_person_gave_stays_and_a_title_that_says_nothing_new_is_ignored() {
        assert_eq!(tab_label("fix login", true, Some(&claude("my-task"))), "fix login");
        assert_eq!(tab_label("Claude Code", true, Some(&claude("Claude Code"))), "Claude Code");
        assert_eq!(tab_label("Claude Code", true, Some(&claude("me@build-box"))), "Claude Code");
        assert_eq!(tab_label("Claude Code", true, Some(&claude("~/src/app"))), "Claude Code");
        assert_eq!(tab_label("Claude Code", true, Some(&claude("/src/app"))), "Claude Code");
        assert_eq!(tab_label("Claude Code", true, Some(&claude("C:\\src\\app"))), "Claude Code");
        assert_eq!(tab_label("Claude Code", true, Some(&claude("claude"))), "Claude Code");
    }

    #[test]
    fn plain_tabs_show_their_program_or_terminal() {
        let shell = Lead { program: Some("zsh".into()), agent: false, ..Lead::default() };
        assert_eq!(tab_label("1", false, Some(&shell)), "zsh");
        assert_eq!(tab_label("1", false, None), "Terminal");
        assert_eq!(tab_label("build", true, Some(&shell)), "build");
    }

    #[test]
    fn clean_drops_a_status_glyph_but_keeps_a_leading_bracket() {
        assert_eq!(clean("\u{2726} fix"), "fix");
        assert_eq!(clean("[web] login"), "[web] login");
        assert_eq!(clean("[ ! ] needs you"), "needs you");
    }

    fn swipe(ys: &[(f32, u64)]) -> Vec<Step> {
        let mut s = Swipe::default();
        let mut now = 1_000_000;
        ys.iter().filter_map(|(y, ms)| {
            now += ms;
            s.feed(*y, false, now)
        }).collect()
    }

    #[test]
    fn a_short_touchpad_swipe_switches_one_tab_and_a_nudge_none() {
        assert_eq!(swipe(&[(-6.0, 8); 12]), [Step::Next]);
        assert_eq!(swipe(&[(-5.0, 8); 3]), []);
    }

    #[test]
    fn each_notch_of_a_smooth_scrolling_mouse_switches_one_tab() {
        let mut events = vec![(-1.0, 10); 28];
        events.push((-1.0, 700));
        events.extend(vec![(-1.0, 10); 27]);
        assert_eq!(swipe(&events), [Step::Next, Step::Next]);
    }

    #[test]
    fn a_long_swipe_keeps_switching_a_tab_per_60_px_and_a_flick_waits_100_ms() {
        assert_eq!(swipe(&[(10.0, 40); 40]), vec![Step::Prev; 7]);
        assert_eq!(swipe(&[(-40.0, 8); 30]), [Step::Next, Step::Next, Step::Next]);
    }

    #[test]
    fn a_pause_or_a_turn_starts_a_new_gesture() {
        let mut events = vec![(-6.0, 8); 12];
        events.push((-30.0, 400));
        events.extend(vec![(-5.0, 8); 4]);
        events.extend(vec![(6.0, 20); 12]);
        assert_eq!(swipe(&events), [Step::Next, Step::Next, Step::Prev]);
    }

    #[test]
    fn walking_by_recent_use_goes_through_the_used_tabs_then_the_rest() {
        let strings = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let (recent, ids) = (strings(&["b", "a", "c"]), strings(&["a", "b", "c", "d"]));
        let mut cycle = None;
        assert_eq!(recent_step(&mut cycle, &recent, &ids, Step::Next, 1000).as_deref(), Some("a"));
        assert_eq!(recent_step(&mut cycle, &recent, &ids, Step::Next, 1100).as_deref(), Some("c"));
        assert_eq!(recent_step(&mut cycle, &recent, &ids, Step::Next, 1200).as_deref(), Some("d"));
        assert_eq!(recent_step(&mut cycle, &recent, &ids, Step::Next, 1300), None);
        assert_eq!(recent_step(&mut cycle, &recent, &ids, Step::Prev, 1400).as_deref(), Some("c"));
        // A new gesture starts from the tab in front: nothing newer.
        assert_eq!(recent_step(&mut cycle, &recent, &ids, Step::Prev, 1400 + CYCLE_MS + 1), None);
    }

    #[test]
    fn a_space_keeps_its_own_panel() {
        let on = vec!["files".to_owned(), "changes".to_owned()];
        let mut p = PanelSync::default();
        // The first space ever seen with no record records what is shown.
        assert_eq!(p.sync("s1", None, &on, 0), PanelOut { restore: None, save: true });
        assert_eq!(p.panels["s1"], None);
        // Opening a panel is recorded.
        assert_eq!(p.sync("s1", Some("files"), &on, 10), PanelOut { restore: None, save: true });
        // Another space: it never had one, so the panel closes.
        assert_eq!(p.sync("s2", Some("files"), &on, 20), PanelOut { restore: Some(None), save: false });
        // Until the close lands the old state is not a choice.
        assert_eq!(p.sync("s2", Some("files"), &on, 30), PanelOut { restore: None, save: false });
        assert_eq!(p.sync("s2", None, &on, 40), PanelOut { restore: None, save: true });
        // Back to s1: its panel returns.
        assert_eq!(p.sync("s1", None, &on, 50), PanelOut { restore: Some(Some("files".into())), save: false });
        // A panel that was turned off in settings is not restored.
        let mut q = PanelSync::default();
        q.panels.insert("s1".into(), Some("search".into()));
        q.space = Some("other".into());
        assert_eq!(q.sync("s1", Some("files"), &on, 0), PanelOut { restore: Some(None), save: false });
    }

    #[test]
    fn invalid_saved_panels_start_empty_and_valid_ones_are_kept() {
        for raw in ["{{{not json", "null", "42", "\"old shape\"", "[]"] {
            assert!(parse_panels(Some(raw)).is_empty(), "{raw}");
        }
        assert!(parse_panels(None).is_empty());
        assert_eq!(parse_panels(Some(r#"{"s1":"files","s2":null}"#)), BTreeMap::from([("s1".into(), Some("files".into())), ("s2".into(), None)]));
    }
}
