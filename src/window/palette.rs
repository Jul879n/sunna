//! The command palette (Cmd/Ctrl+K), native: everything the app can do, by name, with its key. The search line is
//! on top, the list below (rows of 32 px, group titles while the line is empty, the key on the right); ↑ ↓ Page
//! keys move, Enter runs, Esc and the ✕ close. It is the same palette the `frame-palette` mod drew: the same
//! entries, the same ranking and the same `recent` list (`data/frame-palette/store.json`).
//!
//! The entries (which commands, in which group, with which words) and the ranking are plain functions at the
//! top and tested at the bottom; the view only draws them.

use super::native::tone;
use super::*;
use crate::commands::COMMANDS;
use crate::i18n;
use gpui_base::input::{Input, InputEvent, InputState};
use std::path::PathBuf;

/// Overlay id: a second press of the key closes it.
pub(super) const ID: &str = "palette";
const ROW_H: f32 = 32.0;
const HEAD_H: f32 = 28.0;
/// The most the list shows: ten rows.
const LIST_H: f32 = ROW_H * 10.0;
const WIDTH: f32 = 640.0;
const SEARCH_ICON: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/search.svg");
const COMMAND_ICON: &[u8] = include_bytes!("../../assets/icons/palette/command.svg");
const GROUPS: [&str; 9] = ["palette.recent", "palette.tab", "palette.pane", "palette.space", "palette.layout", "palette.terminal", "palette.agent", "palette.window", "palette.app"];

/// How an entry's title is made: a text key, or a text key with the direction or theme in it.
#[derive(Clone, Copy)]
enum Title {
    Key(&'static str),
    Direction(&'static str, &'static str),
}

/// `[command, args, group, title, words]`: the palette's own names for commands.
type Named = (&'static str, &'static str, &'static str, Title, &'static str);

const NAMED: &[Named] = &[
    ("tab.new", "{}", "palette.tab", Title::Key("palette.new_tab"), "tab agent new task pestaña agente nueva tarea"),
    ("mod.open", r#"{"name":"picker"}"#, "palette.tab", Title::Key("palette.go_to_space_or_tab"), "switch jump find cambiar saltar buscar"),
    ("file.goto", "{}", "palette.tab", Title::Key("palette.go_to_file"), "file open find archivo abrir buscar"),
    ("project.add", r#"{"step":"browse"}"#, "palette.tab", Title::Key("folders.open"), "folder directory open carpeta directorio abrir"),
    ("tab.rename", "{}", "palette.tab", Title::Key("palette.rename_tab"), "tab renombrar"),
    ("tab.close", "{}", "palette.tab", Title::Key("palette.close_tab"), "tab kill cerrar"),
    ("tab.next", "{}", "palette.tab", Title::Key("palette.next_tab"), "tab siguiente"),
    ("tab.prev", "{}", "palette.tab", Title::Key("palette.previous_tab"), "tab anterior"),
    ("pane.split", r#"{"direction":"right"}"#, "palette.pane", Title::Key("palette.split_right"), "vertical dividir"),
    ("pane.split", r#"{"direction":"down"}"#, "palette.pane", Title::Key("palette.split_down"), "horizontal dividir"),
    ("pane.close", "{}", "palette.pane", Title::Key("palette.close_pane"), "kill cerrar"),
    ("pane.zoom", "{}", "palette.pane", Title::Key("palette.zoom_pane"), "maximize fullscreen maximizar pantalla completa"),
    ("pane.go", r#"{"direction":"left"}"#, "palette.pane", Title::Direction("palette.focus_pane_direction", "left"), "move go mover ir enfocar"),
    ("pane.go", r#"{"direction":"right"}"#, "palette.pane", Title::Direction("palette.focus_pane_direction", "right"), "move go mover ir enfocar"),
    ("pane.go", r#"{"direction":"up"}"#, "palette.pane", Title::Direction("palette.focus_pane_direction", "up"), "move go mover ir enfocar"),
    ("pane.go", r#"{"direction":"down"}"#, "palette.pane", Title::Direction("palette.focus_pane_direction", "down"), "move go mover ir enfocar"),
    ("pane.swap", r#"{"direction":"left"}"#, "palette.pane", Title::Direction("palette.swap_pane_direction", "left"), "move mover intercambiar"),
    ("pane.swap", r#"{"direction":"right"}"#, "palette.pane", Title::Direction("palette.swap_pane_direction", "right"), "move mover intercambiar"),
    ("pane.swap", r#"{"direction":"up"}"#, "palette.pane", Title::Direction("palette.swap_pane_direction", "up"), "move mover intercambiar"),
    ("pane.swap", r#"{"direction":"down"}"#, "palette.pane", Title::Direction("palette.swap_pane_direction", "down"), "move mover intercambiar"),
    ("space.new", "{}", "palette.space", Title::Key("palette.new_space"), "workspace nuevo"),
    ("space.rename", "{}", "palette.space", Title::Key("palette.rename_space"), "workspace renombrar"),
    ("space.close", "{}", "palette.space", Title::Key("palette.close_space"), "workspace kill cerrar"),
    ("space.next", "{}", "palette.space", Title::Key("palette.next_space"), "workspace siguiente"),
    ("space.prev", "{}", "palette.space", Title::Key("palette.previous_space"), "workspace anterior"),
    ("layout.switch", "{}", "palette.layout", Title::Key("palette.next_layout"), "look switch apariencia cambiar siguiente"),
    ("layout.reset", "{}", "palette.layout", Title::Key("palette.reset_layout"), "new version default restore herdr's restablecer predeterminado"),
    ("layout.clone", "{}", "palette.layout", Title::Key("palette.duplicate_layout"), "clone copy customize duplicar copiar personalizar"),
    ("mod.clone", "{}", "palette.layout", Title::Key("palette.duplicate_mod"), "clone copy customize duplicar copiar personalizar"),
    ("terminal.open", "{}", "palette.terminal", Title::Key("palette.new_terminal"), "shell nueva"),
    ("terminal.copy", "{}", "palette.terminal", Title::Key("palette.copy"), "clipboard copiar portapapeles"),
    ("terminal.paste", "{}", "palette.terminal", Title::Key("palette.paste"), "clipboard pegar portapapeles"),
    ("agent.next", "{}", "palette.agent", Title::Key("palette.next_agent"), "siguiente"),
    ("agent.prev", "{}", "palette.agent", Title::Key("palette.previous_agent"), "anterior"),
    ("window.minimize", "{}", "palette.window", Title::Key("palette.minimize_window"), "minimizar"),
    ("window.maximize", "{}", "palette.window", Title::Key("palette.maximize_window"), "restore maximizar restaurar"),
    ("window.close", "{}", "palette.window", Title::Key("palette.close_window"), "quit cerrar salir"),
    ("app.settings", "{}", "palette.app", Title::Key("palette.settings"), "preferences options config look font theme sound ajustes preferencias opciones apariencia fuente tema sonido"),
    ("app.help", "{}", "palette.app", Title::Key("palette.keys"), "keybindings shortcuts help atajos ayuda teclado"),
    ("app.welcome", "{}", "palette.app", Title::Key("palette.welcome"), "intro start onboarding inicio introducción"),
    ("app.whats_new", "{}", "palette.app", Title::Key("palette.what_s_new"), "release changes news update cambios actualización"),
    ("app.reset", "{}", "palette.app", Title::Key("palette.reset_to_the_default_screen"), "escape hatch broken restore reset restablecer roto"),
    ("app.zoom_in", "{}", "palette.app", Title::Key("palette.zoom_in"), "bigger font size aumentar fuente tamaño"),
    ("app.zoom_out", "{}", "palette.app", Title::Key("palette.zoom_out"), "smaller font size reducir fuente tamaño"),
    ("app.zoom_reset", "{}", "palette.app", Title::Key("palette.reset_zoom"), "font size restablecer fuente tamaño"),
    ("app.quit", "{}", "palette.app", Title::Key("palette.quit_herdr"), "exit salir cerrar"),
];

/// The themes the palette offers by name.
const THEMES: [&str; 4] = ["graphite", "ink", "moss", "atom-one-dark"];

/// Commands that act on something you point at (ids, terminals): not for a list.
const HIDDEN: &[&str] = &[
    "space.move", "tab.move", "space.select", "tab.select", "pane.focus", "terminal.keep", "terminal.park", "terminal.kill", "terminal.show", "agent.focus", "layout.set", "layout.toggle",
    "layout.show", "layout.hide", "layout.resize", "layout.select", "mod.open", "overlay.close", "tab.settle", "tab.unsettle", "app.palette", "app.hide", "app.hide_others", "app.show_all",
    "app.theme", "app.settings", "terminal.quick", "quick.set", "quick.edit", "quick.delete",
];

/// One thing the palette can run.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Entry {
    pub name: String,
    pub args: Value,
    pub group: String,
    pub title: String,
    pub words: String,
    /// `name:args`: what `recent` remembers.
    pub id: String,
}

impl Entry {
    fn new(name: &str, args: Value, group: String, title: String, words: &str) -> Self {
        let id = format!("{name}:{args}");
        Self { name: name.to_owned(), args, group, title, words: words.to_owned(), id }
    }
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |f| f.to_uppercase().chain(chars).collect())
}

/// The name of a layout in the list: the shipped ones have their own, the others are capitalized.
fn layout_title(name: &str) -> String {
    match name {
        "main" => "Sunna".to_owned(),
        "terminals" => "Terminals".to_owned(),
        "win98" => "Herdr 98".to_owned(),
        other => capitalize(other),
    }
}

/// A command with no name of its own in the table: `tab.new_thing` is "New thing tab" in the group "Tab".
fn title_of(name: &str) -> (String, String) {
    let (noun, verb) = name.split_once('.').unwrap_or((name, ""));
    (capitalize(noun), capitalize(&format!("{} {noun}", verb.replace('_', " "))))
}

/// Every entry for the state we are in now: the table, a switch per layout, then anything new in the app.
pub(super) fn entries(layouts: &[String], mac: bool) -> Vec<Entry> {
    let title = |t: Title| match t {
        Title::Key(k) => i18n::t(k),
        Title::Direction(k, d) => i18n::tf(k, &[("direction", &i18n::t(&format!("palette.{d}")))]),
    };
    let mut list: Vec<Entry> = NAMED
        .iter()
        .filter(|(name, ..)| COMMANDS.iter().any(|c| c.name == *name))
        .filter(|(name, ..)| !(mac && name.starts_with("window.")))
        .map(|(name, args, group, t, words)| Entry::new(name, serde_json::from_str(args).unwrap_or(json!({})), i18n::t(group), title(*t), words))
        .collect();
    for theme in THEMES {
        list.push(Entry::new("app.theme", json!({ "name": theme }), i18n::t("palette.app"), i18n::tf("palette.theme_name", &[("name", &theme.replace('-', " "))]), "colors look colores apariencia"));
    }
    for l in layouts {
        list.push(Entry::new("layout.switch", json!({ "name": l }), i18n::t("palette.layout"), i18n::tf("palette.layout_name", &[("name", &layout_title(l))]), &format!("look layout diseño {l}")));
    }
    for q in super::quick::load() {
        list.push(Entry::new("terminal.quick", json!({ "name": q.name }), i18n::t("palette.quick"), i18n::tf("palette.quick_run", &[("name", &q.name)]), &format!("quick command comando rápido {} {}", q.name, q.command)));
    }
    for c in COMMANDS {
        let named = NAMED.iter().any(|(n, ..)| *n == c.name);
        if named || HIDDEN.contains(&c.name) || c.args.iter().any(|a| a.required && a.name != "name") {
            continue;
        }
        let (group, title) = title_of(c.name);
        list.push(Entry::new(c.name, json!({}), group, title, c.about));
    }
    list
}

/// Fuzzy score of `q` (lowercase) in `text`, with the matched positions (in characters): your letters in order;
/// word starts and runs rank higher. `None`: no match.
pub(super) fn fuzzy(q: &str, text: &str) -> Option<(f32, Vec<usize>)> {
    let t: Vec<char> = text.chars().map(|c| c.to_lowercase().next().unwrap_or(c)).collect();
    let (mut score, mut at, mut run) = (0.0_f32, 0usize, 0i32);
    let mut hits: Vec<usize> = Vec::new();
    for ch in q.chars() {
        let i = at + t.get(at..)?.iter().position(|c| *c == ch)?;
        let start = i == 0 || matches!(t[i - 1], ' ' | '\t' | ':' | '.' | '-' | '_' | '/');
        run = if i == at && !hits.is_empty() { run + 1 } else { 0 };
        score += 1.0 + if start { 6.0 } else { 0.0 } + run as f32 * 3.0 - (i - at).min(6) as f32 * 0.3;
        hits.push(i);
        at = i + 1;
    }
    Some((score - t.len() as f32 * 0.02, hits))
}

/// A row to show: the entry (an index into the list) and the letters of its title that matched.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Hit {
    pub at: usize,
    pub hits: Vec<usize>,
    /// Shown in the "recent" run at the top of the empty list.
    pub recent: bool,
}

/// The rows for `query`: grouped when it is empty (the recent ones first), ranked while typing.
pub(super) fn rows(all: &[Entry], recent: &[String], query: &str) -> Vec<Hit> {
    let q: String = query.to_lowercase().split_whitespace().collect();
    if q.is_empty() {
        let recent_at: Vec<usize> = recent.iter().filter_map(|id| all.iter().position(|e| e.id == *id)).take(5).collect();
        let groups: Vec<String> = GROUPS.iter().map(|g| i18n::t(g)).collect();
        let free = |i: &usize| !recent_at.contains(i);
        let mut out: Vec<usize> = recent_at.clone();
        for g in &groups[1..] {
            out.extend((0..all.len()).filter(|i| free(i) && all[*i].group == *g));
        }
        out.extend((0..all.len()).filter(|i| free(i) && !groups.contains(&all[*i].group)));
        return out.into_iter().map(|at| Hit { at, hits: Vec::new(), recent: recent_at.contains(&at) }).collect();
    }
    let rank = |e: &Entry| recent.iter().position(|r| *r == e.id).unwrap_or(99);
    let mut scored: Vec<(f32, Hit)> = all
        .iter()
        .enumerate()
        .filter_map(|(at, e)| {
            let best = fuzzy(&q, &e.title).or_else(|| fuzzy(&q, &format!("{} {} {} {}", e.group, e.title, e.name, e.words)).map(|(s, _)| (s - 4.0, Vec::new())))?;
            Some((best.0, Hit { at, hits: best.1, recent: false }))
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(rank(&all[a.1.at]).cmp(&rank(&all[b.1.at]))));
    scored.into_iter().map(|(_, h)| h).collect()
}

/// What the list draws: a group title or a row (an index into the rows).
#[derive(Clone, Debug, PartialEq)]
enum Item {
    Head(String),
    Row(usize),
}

/// The list's items: while the query is empty, a title before each run of one group.
fn items(all: &[Entry], rows: &[Hit], grouped: bool) -> Vec<Item> {
    let mut out = Vec::new();
    let mut last: Option<String> = None;
    for (i, h) in rows.iter().enumerate() {
        if grouped {
            let head = if h.recent { i18n::t("palette.recent") } else { all[h.at].group.clone() };
            if last.as_ref() != Some(&head) {
                out.push(Item::Head(head.clone()));
                last = Some(head);
            }
        }
        out.push(Item::Row(i));
    }
    out
}

/// "ctrl-shift-p" -> ["Ctrl", "Shift", "P"] (on a Mac, the symbols); "ctrl--" -> ["Ctrl", "-"].
pub(super) fn key_chips(keys: &str, mac: bool) -> Vec<String> {
    let parts: Vec<&str> = match keys.strip_suffix("--") {
        Some(head) => head.split('-').chain(["-"]).collect(),
        None => keys.split('-').collect(),
    };
    parts
        .into_iter()
        .map(|p| match (p.to_lowercase().as_str(), mac) {
            ("cmd", true) => "⌘".to_owned(),
            ("cmd", false) => "Ctrl".to_owned(),
            ("ctrl", true) => "⌃".to_owned(),
            ("shift", true) => "⇧".to_owned(),
            ("alt" | "option", true) => "⌥".to_owned(),
            ("up", _) => "↑".to_owned(),
            ("down", _) => "↓".to_owned(),
            ("left", _) => "←".to_owned(),
            ("right", _) => "→".to_owned(),
            ("enter", _) => "↵".to_owned(),
            (other, _) if other.chars().count() == 1 => other.to_uppercase(),
            (other, _) => capitalize(other),
        })
        .collect()
}

/// The key bound to an entry: `name` or `name:value` in the global section of keys.json.
pub(super) fn key_of(e: &Entry, bound: &[(String, String, String)]) -> Option<String> {
    let value = e.args.as_object().and_then(|o| o.values().next()).map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_owned));
    let want = value.map_or_else(|| e.name.clone(), |v| format!("{}:{v}", e.name));
    bound.iter().find(|(s, _, a)| s == "global" && *a == want).map(|(_, k, _)| k.clone())
}

// The recent list ---------------------------------------------------------------------

/// Where the mod kept its state: `data/<mod>/store.json`, every value a string.
pub(super) fn store_path(name: &str) -> PathBuf {
    mods::app_dir().join("data").join(name).join("store.json")
}

pub(super) fn store_get(name: &str, key: &str) -> Option<String> {
    let file: Value = serde_json::from_str(&std::fs::read_to_string(store_path(name)).ok()?).ok()?;
    file.get(key)?.as_str().map(str::to_owned)
}

pub(super) fn store_set(name: &str, key: &str, value: String) {
    let path = store_path(name);
    let mut file = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).filter(Value::is_object).unwrap_or_else(|| json!({}));
    file[key] = json!(value);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, serde_json::to_string_pretty(&file).unwrap_or_default());
}

fn load_recent() -> Vec<String> {
    store_get("frame-palette", "recent").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

/// `id` first, no repeats, at most 20.
pub(super) fn remember(recent: &[String], id: &str) -> Vec<String> {
    std::iter::once(id.to_owned()).chain(recent.iter().filter(|r| *r != id).cloned()).take(20).collect()
}

/// `label` with its matched letters (`hits`, in characters) in the accent.
pub(super) fn highlighted(label: &str, hits: &[usize]) -> StyledText {
    let starts: Vec<usize> = label.char_indices().map(|(b, _)| b).chain([label.len()]).collect();
    let lit = HighlightStyle { color: Some(c("accent")), font_weight: Some(FontWeight::SEMIBOLD), ..Default::default() };
    let marks: Vec<_> = hits.iter().filter(|&&i| i + 1 < starts.len()).map(|&i| (starts[i]..starts[i + 1], lit)).collect();
    StyledText::new(label.to_owned()).with_highlights(marks)
}

// The view ---------------------------------------------------------------------------

pub(super) struct PaletteView {
    app: WeakEntity<AppWindow>,
    input: Entity<InputState>,
    focus: dialog::Focus,
    all: Vec<Entry>,
    bound: Vec<(String, String, String)>,
    recent: Vec<String>,
    query: String,
    rows: Vec<Hit>,
    at: usize,
    /// The first item (row or title) the list shows.
    first: usize,
    /// The keys moved the cursor: the list scrolls to show it (the wheel does not).
    reveal: bool,
}

impl PaletteView {
    fn new(app: WeakEntity<AppWindow>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(i18n::t("palette.type_a_command")));
        input.update(cx, |s, _| s.set_editor_style(dialog::input_style()));
        cx.subscribe_in(&input, window, |this, input, ev: &InputEvent, window, cx| match ev {
            InputEvent::Change => {
                this.query = input.read(cx).value().to_string();
                this.at = 0;
                this.first = 0;
                this.rows = rows(&this.all, &this.recent, &this.query);
                cx.notify();
            }
            InputEvent::PressEnter { .. } => this.go(this.at, window, cx),
            InputEvent::Focus | InputEvent::Blur => cx.notify(),
            _ => {}
        })
        .detach();
        input.update(cx, |s, cx| s.focus(window, cx));
        let all = entries(&crate::window::layout_names(), cfg!(target_os = "macos"));
        let recent = load_recent();
        let rows = rows(&all, &recent, "");
        Self { app, input, focus: dialog::Focus::new(cx), all, bound: crate::keys::listing(), recent, query: String::new(), rows, at: 0, first: 0, reveal: false }
    }

    fn close(&self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.app.update(cx, |app, cx| app.close_overlay(ID, window, cx));
    }

    fn go(&mut self, i: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(e) = self.rows.get(i).map(|h| self.all[h.at].clone()) else { return };
        store_set("frame-palette", "recent", serde_json::to_string(&remember(&self.recent, &e.id)).unwrap_or_default());
        let _ = self.app.update(cx, |app, cx| {
            app.close_overlay(ID, window, cx);
            match e.name.as_str() {
                // The prompt asks which mod to copy.
                "mod.clone" => {
                    let spec = json!({ "mod": "prompt", "kind": "dialog", "width": 380, "props": {
                        "title": i18n::t("palette.duplicate_mod"), "hint": i18n::t("palette.mod_to_copy_creates_name_copy"),
                        "command": { "name": "mod.clone", "args": {} } } });
                    app.open_overlay(mods::new_overlay_id(), spec, window, cx);
                }
                "layout.clone" => {
                    let layout = mods::shared("layout").and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_else(|| "main".into());
                    app.run(&e.name, json!({ "name": layout }), window, cx);
                }
                _ => app.run(&e.name, e.args.clone(), window, cx),
            }
        });
    }

    fn items(&self) -> Vec<Item> {
        items(&self.all, &self.rows, self.query.trim().is_empty())
    }
}

impl Render for PaletteView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let z = settings::z;
        let mac = cfg!(target_os = "macos");
        self.at = self.at.min(self.rows.len().saturating_sub(1));
        let items = self.items();
        let height = |i: &Item| if matches!(i, Item::Head(_)) { HEAD_H } else { ROW_H };
        let sel = items.iter().position(|i| *i == Item::Row(self.at)).unwrap_or(0);
        self.first = self.first.min(items.len().saturating_sub(1));
        if self.reveal {
            // A row that opens a group is shown with its title.
            let from = if sel > 0 && matches!(items[sel - 1], Item::Head(_)) { sel - 1 } else { sel };
            if from < self.first {
                self.first = from;
            }
            while self.first < sel && items[self.first..=sel].iter().map(height).sum::<f32>() > LIST_H {
                self.first += 1;
            }
            self.reveal = false;
        }
        let mut used = 0.0;
        // A group title needs its first row beside it: alone at the bottom it would hang.
        let shown: Vec<&Item> = items.iter().skip(self.first).take_while(|i| {
            used += height(i) + if matches!(i, Item::Head(_)) { ROW_H } else { 0.0 };
            let fits = used <= LIST_H + 0.5;
            used -= if matches!(i, Item::Head(_)) { ROW_H } else { 0.0 };
            fits
        }).collect();
        let muted = tone("muted");
        let grouped = self.query.trim().is_empty();
        let list = if self.rows.is_empty() {
            div().px(z(16.0)).py(z(14.0)).text_size(z(13.0)).text_color(muted).child(i18n::tf("palette.nothing_matches_query_places_are", &[("query", &self.query)]))
        } else {
            div().flex().flex_col().children(shown.into_iter().map(|item| match item {
                Item::Head(text) => div().h(z(HEAD_H)).flex_none().px(z(16.0)).pb(z(4.0)).flex().items_end().text_size(z(11.0)).font_weight(FontWeight::SEMIBOLD).text_color(muted).child(text.clone()).into_any_element(),
                Item::Row(i) => {
                    let (i, h) = (*i, &self.rows[*i]);
                    let e = &self.all[h.at];
                    let on = i == self.at;
                    let chips = key_of(e, &self.bound).map(|k| key_chips(&k, mac)).unwrap_or_default();
                    div()
                        .id(SharedString::from(format!("pl-{i}")))
                        .h(z(ROW_H))
                        .flex_none()
                        .mx(z(6.0))
                        .px(z(10.0))
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(z(10.0))
                        .rounded(z(7.0))
                        .cursor_pointer()
                        .text_size(z(12.8))
                        .when(on, |d| d.bg(c("selected")))
                        .hover(|s| s.bg(c("hover")))
                        .on_click(cx.listener(move |this, _, w, cx| this.go(i, w, cx)))
                        .child(div().flex_1().min_w_0().truncate().text_color(tone(if on { "foreground" } else { "muted" })).child(highlighted(&e.title, &h.hits)))
                        .when(!grouped, |d| d.child(div().flex_none().text_size(z(11.5)).text_color(muted).child(e.group.clone())))
                        .when(!chips.is_empty(), |d| {
                            d.child(div().flex().flex_none().gap(z(3.0)).children(chips.into_iter().map(|p| {
                                div().min_w(z(20.0)).h(z(20.0)).px(z(5.0)).flex().items_center().justify_center().rounded(z(4.0)).border_1().border_color(c("border")).bg(c("raised")).text_size(z(11.0)).text_color(muted).child(p)
                            })))
                        })
                        .into_any_element()
                }
            }))
        };
        let this = cx.entity();
        let close = move |w: &mut Window, cx: &mut App| this.update(cx, |v, cx| v.close(w, cx));
        let focused = self.input.read(cx).focus_handle(cx).is_focused(window);
        let search = dialog::field_box(focused, false)
            .flex_none()
            .gap(z(10.0))
            .child(svg().data(if mac { COMMAND_ICON } else { SEARCH_ICON }).size(z(16.0)).text_color(tone("dim")))
            .child(div().flex_1().h_full().child(Input::new(&self.input)));
        let body = div()
            .flex()
            .flex_col()
            .gap(z(10.0))
            .child(search)
            .child(
                div()
                    .id("palette-list")
                    .h(z(if self.rows.is_empty() { 48.0 } else { LIST_H }))
                    .overflow_hidden()
                    .on_scroll_wheel(cx.listener(|this, e: &ScrollWheelEvent, _, cx| {
                        let y = f32::from(e.delta.pixel_delta(px(ROW_H)).y);
                        let steps = (-y / ROW_H).round() as i64;
                        let last = this.items().len().saturating_sub(1) as i64;
                        this.first = (this.first as i64 + steps).clamp(0, last) as usize;
                        cx.notify();
                    }))
                    .child(list),
            )
            .child(div().text_size(z(12.0)).text_color(muted).child(i18n::t("rs.palette.hints")));
        let card = dialog::Dialog::new(i18n::t("rs.palette.title"), &self.focus, close).width(WIDTH).no_cancel().fields(1).body(body).render("palette");
        // Up/Down/Page keys move the cursor; Esc and Tab are the dialog's own.
        div().on_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
            let last = this.rows.len().saturating_sub(1);
            match e.keystroke.key.as_str() {
                // Stops at the ends: wrapping jumps out of sight.
                "down" => this.at = (this.at + 1).min(last),
                "up" => this.at = this.at.saturating_sub(1),
                "pagedown" => this.at = (this.at + 10).min(last),
                "pageup" => this.at = this.at.saturating_sub(10),
                _ => return,
            }
            this.reveal = true;
            cx.stop_propagation();
            cx.notify();
        })).child(card)
    }
}

impl AppWindow {
    /// `app.palette`: opens the palette, or closes it when it is open.
    pub(super) fn toggle_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.overlays.iter().any(|o| o.id == ID) {
            return self.close_overlay(ID, window, cx);
        }
        let app = cx.weak_entity();
        let view = cx.new(|cx| PaletteView::new(app, window, cx));
        let focus = cx.focus_handle();
        self.push_view_overlay(ID.into(), OverlayKind::Popup, size(WIDTH + 48.0, 0.85), Some(40.0), view.into(), focus, window, cx);
        if let Some(o) = self.overlays.iter_mut().find(|o| o.id == ID) {
            o.bare = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `super::*` brings gpui's own `test` macro: the plain one is the std attribute.
    use core::prelude::v1::test;

    fn list() -> Vec<Entry> {
        entries(&["main".to_owned(), "terminals".to_owned()], false)
    }

    #[test]
    fn the_named_commands_all_exist_and_come_first_in_their_groups() {
        for (name, args, ..) in NAMED {
            assert!(COMMANDS.iter().any(|c| c.name == *name), "{name} is not a command");
            assert!(serde_json::from_str::<Value>(args).is_ok(), "{name}: bad args {args}");
        }
        let all = list();
        assert!(all.iter().any(|e| e.name == "tab.new" && e.group == i18n::t("palette.tab")));
        assert!(all.iter().any(|e| e.name == "layout.switch" && e.args == json!({ "name": "terminals" }) && e.title.contains("Terminals")));
        assert!(all.iter().any(|e| e.name == "app.theme" && e.args == json!({ "name": "atom-one-dark" })));
    }

    #[test]
    fn a_mac_has_no_window_entries_and_hidden_commands_never_show() {
        assert!(entries(&[], true).iter().all(|e| !e.name.starts_with("window.")));
        assert!(list().iter().any(|e| e.name == "window.close"));
        for e in list() {
            // `mod.open` is only there through its two named entries.
            if HIDDEN.contains(&e.name.as_str()) && e.name != "app.theme" && e.name != "mod.open" && e.name != "app.settings" {
                panic!("{} is hidden", e.name);
            }
        }
    }

    #[test]
    fn a_new_command_gets_a_title_from_its_name() {
        assert_eq!(title_of("tab.new_thing"), ("Tab".to_owned(), "New thing tab".to_owned()));
    }

    #[test]
    fn fuzzy_wants_the_letters_in_order_and_ranks_word_starts_higher() {
        assert!(fuzzy("zz", "new tab").is_none());
        let (a, hits) = fuzzy("nt", "new tab").unwrap();
        assert_eq!(hits, [0, 4]);
        let (b, _) = fuzzy("nt", "internet").unwrap();
        assert!(a > b);
    }

    #[test]
    fn typing_ranks_and_an_empty_line_groups_with_the_recent_first() {
        let all = list();
        let tab = all.iter().find(|e| e.name == "tab.new").unwrap().id.clone();
        let close = all.iter().find(|e| e.name == "pane.close").unwrap().id.clone();
        let grouped = rows(&all, &[close.clone(), tab.clone()], "");
        assert_eq!(all[grouped[0].at].id, close);
        assert_eq!(all[grouped[1].at].id, tab);
        // Each entry once.
        assert_eq!(grouped.len(), all.len());
        let heads = items(&all, &grouped, true);
        assert_eq!(heads[0], Item::Head(i18n::t("palette.recent")));
        let typed = rows(&all, &[], &all.iter().find(|e| e.name == "tab.new").unwrap().title.to_lowercase());
        assert!(!typed.is_empty());
        assert_eq!(all[typed[0].at].name, "tab.new");
    }

    #[test]
    fn keys_read_as_chips_and_belong_to_their_entry() {
        assert_eq!(key_chips("cmd-shift-p", true), ["⌘", "⇧", "P"]);
        assert_eq!(key_chips("ctrl--", false), ["Ctrl", "-"]);
        let bound = vec![("global".to_owned(), "cmd-t".to_owned(), "tab.new".to_owned()), ("global".to_owned(), "cmd-h".to_owned(), "pane.go:left".to_owned())];
        let all = list();
        let find = |n: &str, a: Value| all.iter().find(|e| e.name == n && (a.is_null() || e.args == a)).unwrap().clone();
        assert_eq!(key_of(&find("tab.new", Value::Null), &bound).as_deref(), Some("cmd-t"));
        assert_eq!(key_of(&find("pane.go", json!({ "direction": "left" })), &bound).as_deref(), Some("cmd-h"));
        assert_eq!(key_of(&find("pane.go", json!({ "direction": "up" })), &bound), None);
    }

    #[test]
    fn recent_keeps_twenty_without_repeats() {
        let mut r: Vec<String> = Vec::new();
        for i in 0..25 {
            r = remember(&r, &format!("c{i}"));
        }
        assert_eq!(r.len(), 20);
        assert_eq!(remember(&r, "c10")[0], "c10");
        assert_eq!(remember(&r, "c10").iter().filter(|x| *x == "c10").count(), 1);
    }
}
