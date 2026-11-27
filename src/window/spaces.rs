//! The sidebar's list (`{"native": "spaces"}`, once the `spaces` mod): Search and
//! Bots on top, then the projects (one header per repo or folder, its spaces and
//! their agents under it, a worktree under its checkout), grouped by machine when
//! another one is known, or a 44 px rail of project tiles. Git facts, project
//! icons, machines and bots are read in the background; what is not drawing
//! (grouping, nesting, the order spaces are put in, colors, drag moves) is plain
//! functions, tested at the end.

use super::native::{icon, r, ring, side_graphic, side_tone as tone, sidebar_separator, step_cursor, tip, z};
use super::*;
use crate::i18n;
use std::collections::{BTreeMap, BTreeSet};

mod details;
mod drag;
mod groups;
mod menus;
mod view;
mod worktree;
pub(super) use worktree::{cleanup_delete, cleanup_scan};
pub(super) use menus::COMMANDS as MENU_COMMANDS;
pub(super) use groups::{default_group_name, group_name, handles as handles_group_command};
use view::{Ctx, Filters, GroupBy, ProjectOrder, SortBy, Status, View};

macro_rules! chrome_icon {
    ($name:literal) => {
        include_bytes!(concat!("../../defaults/mods/workspace-chrome/", $name, ".svg")).as_slice()
    };
}
const BRANCH: &[u8] = chrome_icon!("branch");
const GIT_BRANCH: &[u8] = chrome_icon!("git-branch");
const CHEVRON_DOWN: &[u8] = chrome_icon!("chevron-down");
const CHEVRON_RIGHT: &[u8] = chrome_icon!("chevron-right");
const SEARCH: &[u8] = chrome_icon!("search");
const PLUS: &[u8] = chrome_icon!("plus");
const FOLDER_PLUS: &[u8] = chrome_icon!("folder-plus");
const SERVER: &[u8] = chrome_icon!("server");
const MONITOR: &[u8] = chrome_icon!("monitor");
const SPINNER: &[u8] = chrome_icon!("spinner");
const PIN: &[u8] = chrome_icon!("pin");
const CHECK: &[u8] = chrome_icon!("check");
const CIRCLE: &[u8] = chrome_icon!("square");
const LIST_FILTER: &[u8] = include_bytes!("../../assets/icons/sidebar/list-filter.svg");
const FOLD_ALL: &[u8] = include_bytes!("../../assets/icons/sidebar/chevrons-down-up.svg");
const UNFOLD_ALL: &[u8] = include_bytes!("../../assets/icons/sidebar/chevrons-up-down.svg");
const LOCATE: &[u8] = include_bytes!("../../assets/icons/sidebar/locate.svg");
const FOLDER_OPEN: &[u8] = include_bytes!("../../assets/icons/sidebar/folder-open.svg");
const FOLDER_CLOSED: &[u8] = chrome_icon!("folder");

/// Changes made outside Sunna (an editor's save, a commit in another app) are looked for this often, in repos the list shows, while the window is in front.
const FALLBACK: Duration = Duration::from_secs(60);
/// Places looked at together (two git runs each): all at once, a few hundred spaces run out of open files.
const AT_ONCE: usize = 6;
/// A new space waits (at most this long) for its git, so a worktree appears under its repo instead of jumping there.
const BORN_WAIT: Duration = Duration::from_millis(1500);
/// What a stored color name looks like on an avatar: one strong color per hue, light and dark alike.
const MARK: [(&str, &str); 6] = [("ansi6", "#22a99a"), ("ansi3", "#e3814f"), ("ansi4", "#5b8def"), ("ansi5", "#8b7cf6"), ("ansi2", "#3fae6a"), ("ansi1", "#d9649a")];
const PALETTE: [&str; 6] = ["ansi6", "ansi3", "ansi4", "ansi5", "ansi2", "ansi1"];
/// Where a repo usually keeps its icon (png or svg), most specific first.
const ICON_FILES: [&str; 30] = [
    "assets/icon.png", "assets/images/icon.png", "assets/logo.png", "assets/logo.svg", "assets/icons/icon.png", "icon.png", "icon.svg", "logo.png", "logo.svg", "app/icon.png", "app/icon.svg",
    "src/app/icon.png", "src/app/icon.svg", "public/icon.png", "public/icon.svg", "public/logo.png", "public/logo.svg", "public/favicon.png", "public/favicon.svg", "static/favicon.png",
    "static/favicon.svg", "favicon.png", "favicon.svg", "build/icon.png", "resources/icon.png", "src-tauri/icons/128x128.png", "docs/logo.png", "docs/logo.svg", ".github/logo.png", ".github/logo.svg",
];

// Pure rules ----------------------------------------------------------------

/// What git says about a place.
#[derive(Clone, Debug, PartialEq)]
struct Git {
    name: String,
    branch: String,
    ahead: u64,
    behind: u64,
    dirty: bool,
    /// `[machine, the repo's .git folder]` as JSON: what places of one repo share.
    repo: String,
    linked: bool,
}

/// What a space row's second line shows.
#[derive(Debug, PartialEq)]
struct Facts {
    branch: Option<String>,
    ahead: u64,
    behind: u64,
    dirty: bool,
}

/// The branch (when the name doesn't carry it), ahead/behind and uncommitted changes, if any.
fn git_facts(name: &str, g: Option<&Git>) -> Option<Facts> {
    let g = g?;
    let branch = (!g.branch.is_empty() && g.branch != name).then(|| g.branch.clone());
    (branch.is_some() || g.ahead > 0 || g.behind > 0 || g.dirty).then_some(Facts { branch, ahead: g.ahead, behind: g.behind, dirty: g.dirty })
}

pub(super) fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or_default()
}

pub(super) fn opt_text<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}

pub(super) fn items<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key).and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

fn base(path: &str) -> String {
    path.trim_end_matches('/').rsplit('/').next().unwrap_or_default().to_owned()
}

/// "now", "5m", "3h", "13d": how long ago a tab last changed.
fn ago(changed: u64, now: u64) -> String {
    if changed == 0 {
        return String::new();
    }
    let s = now.saturating_sub(changed) / 1000;
    match s {
        0..60 => "now".into(),
        60..3600 => format!("{}m", s / 60),
        3600..86400 => format!("{}h", s / 3600),
        _ => format!("{}d", s / 86400),
    }
}

fn changed_at(w: &Value) -> u64 {
    items(w, "tabs").iter().filter_map(|t| t.get("changed_at").and_then(Value::as_u64)).max().unwrap_or(0)
}

/// Where a space is now: its first terminal's live folder, else where it was made.
fn place(w: &Value, panes: &[Value]) -> String {
    let tabs: BTreeSet<&str> = items(w, "tabs").iter().map(|t| text(t, "id")).collect();
    panes.iter().find(|p| tabs.contains(text(p, "tab")) && !text(p, "cwd").is_empty()).map(|p| text(p, "cwd").to_owned()).unwrap_or_else(|| text(w, "folder").to_owned())
}

fn where_key(w: &Value, panes: &[Value]) -> String {
    json!([w.get("machine").cloned().unwrap_or(Value::Null), place(w, panes)]).to_string()
}

/// An unnamed space (named after its folder) shows its branch instead, when Settings > Git says so.
fn shown_name(w: &Value, g: Option<&Git>, branch_names: bool) -> String {
    let name = text(w, "name");
    let folder = text(w, "folder");
    match g {
        Some(g) if branch_names && !g.branch.is_empty() && g.branch != "(detached)" && !folder.is_empty() && name == base(folder) => g.branch.clone(),
        _ => name.to_owned(),
    }
}

/// One space as the list orders it.
#[derive(Clone, Debug)]
struct Row {
    w: Value,
    i: usize,
    g: Option<Git>,
}

impl Row {
    fn id(&self) -> &str {
        text(&self.w, "id")
    }
    fn machine(&self) -> Option<&str> {
        opt_text(&self.w, "machine")
    }
    fn active(&self) -> bool {
        self.w["active"] == true
    }
}

/// Spaces in order, each worktree space moved under its main checkout's.
fn ordered_all(spaces: &[Value], panes: &[Value], git: &HashMap<String, Option<Git>>) -> Vec<Row> {
    let list: Vec<Row> = spaces.iter().enumerate().map(|(i, w)| Row { w: w.clone(), i, g: git.get(&where_key(w, panes)).cloned().flatten() }).collect();
    let mains: BTreeSet<&str> = list.iter().filter_map(|r| r.g.as_ref().filter(|g| !g.linked).map(|g| g.repo.as_str())).collect();
    let nested = |r: &Row| r.g.as_ref().is_some_and(|g| g.linked && mains.contains(g.repo.as_str()));
    let mut out = Vec::new();
    let mut placed: BTreeSet<&str> = BTreeSet::new();
    for r in &list {
        if nested(r) {
            continue;
        }
        out.push(r.clone());
        let Some(g) = r.g.as_ref().filter(|g| !g.linked) else { continue };
        if !placed.insert(g.repo.as_str()) {
            continue;
        }
        out.extend(list.iter().filter(|k| nested(k) && k.g.as_ref().is_some_and(|kg| kg.repo == g.repo)).cloned());
    }
    out
}

/// The `space.move`s that turn `order` into `target`, each index as it is once the earlier moves landed.
fn plan_moves(order: &[String], target: &[String]) -> Vec<(String, usize)> {
    let mut order = order.to_vec();
    let mut moves = Vec::new();
    for (i, id) in target.iter().enumerate() {
        let Some(from) = order.iter().position(|o| o == id) else { continue };
        if from == i {
            continue;
        }
        moves.push((id.clone(), i + 1));
        let moved = order.remove(from);
        order.insert(i, moved);
    }
    moves
}

/// One project: a repo (or a folder) and its spaces.
struct Group {
    key: String,
    label: String,
    rows: Vec<Row>,
}

/// What a space's project is called in the saved lists: its repo, else its folder.
fn project_key(r: &Row) -> String {
    let folder = text(&r.w, "folder");
    r.g.as_ref().map_or_else(|| format!("folder:{}", if folder.is_empty() { text(&r.w, "name") } else { folder }), |g| g.repo.clone())
}

fn group_projects(rows: &[Row]) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    for r in rows {
        let folder = text(&r.w, "folder");
        let key = project_key(r);
        match groups.iter_mut().find(|g| g.key == key) {
            Some(g) => g.rows.push(r.clone()),
            None => {
                let label = r.g.as_ref().map(|g| g.name.clone()).filter(|n| !n.is_empty()).or_else(|| Some(base(folder)).filter(|b| !b.is_empty())).unwrap_or_else(|| text(&r.w, "name").to_owned());
                groups.push(Group { key, label, rows: vec![r.clone()] });
            }
        }
    }
    groups
}

/// Keeps the colors of spaces that still exist and gives each new one the least used hue (a `~` space gets none).
fn assign_colors(current: &serde_json::Map<String, Value>, spaces: &[(String, String)]) -> (serde_json::Map<String, Value>, bool) {
    let ids: BTreeSet<&str> = spaces.iter().map(|(id, _)| id.as_str()).collect();
    let mut kept: serde_json::Map<String, Value> = current.iter().filter(|(id, _)| ids.contains(id.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect();
    let mut changed = kept.len() != current.len();
    for (id, name) in spaces {
        if kept.get(id).is_some_and(|v| v.as_str().is_some_and(|s| !s.is_empty())) || name == "~" {
            continue;
        }
        let uses = |c: &str| kept.values().filter(|v| v.as_str() == Some(c)).count();
        let best = PALETTE.iter().fold(PALETTE[0], |best, c| if uses(c) < uses(best) { c } else { best });
        kept.insert(id.clone(), json!(best));
        changed = true;
    }
    (kept, changed)
}

/// `blocked` over `working` over nothing, for the agents of these spaces.
fn agent_state(agents: &[Value], ids: &BTreeSet<&str>) -> Option<&'static str> {
    let mine: Vec<&Value> = agents.iter().filter(|a| ids.contains(text(a, "space"))).collect();
    if mine.iter().any(|a| text(a, "status") == "blocked") {
        Some("blocked")
    } else if mine.iter().any(|a| text(a, "status") == "working") {
        Some("working")
    } else {
        None
    }
}

/// A machine's state, as people read it; an unknown one shows its own words.
fn state_word(state: Option<&str>) -> String {
    let state = state.unwrap_or("asleep");
    match state {
        "asleep" | "connecting" | "disconnected" | "unreachable" | "needs_login" | "needs_update" | "reconnecting" => i18n::t(&format!("spaces.{state}")),
        other => other.replace('_', " "),
    }
}

fn rebuilt(machine: Option<&Value>) -> bool {
    let Some(m) = machine else { return false };
    if text(m, "state") != "unreachable" {
        return false;
    }
    let why = text(m, "why");
    why.strip_prefix("a different core answers there (\"").and_then(|r| r.strip_suffix("\")")).is_some_and(|name| !name.is_empty() && !name.contains('"'))
}

/// The saved set of folded machines. A string is read as its letters, as the mod did.
fn parse_folded(raw: Option<&str>) -> BTreeSet<String> {
    match raw.and_then(|r| serde_json::from_str::<Value>(r).ok()) {
        // This computer is folded as `null`.
        Some(Value::Array(a)) => a.iter().filter_map(|v| if v.is_null() { Some(String::new()) } else { v.as_str().map(str::to_owned) }).collect(),
        Some(Value::String(s)) => s.chars().map(String::from).collect(),
        _ => BTreeSet::new(),
    }
}

/// The saved reconnects: a machine whose reconnect was cut off is ready to retry (not busy). Anything but a map of maps starts empty.
fn parse_reconnecting(raw: Option<&str>) -> BTreeMap<String, Value> {
    let Some(Value::Object(map)) = raw.and_then(|r| serde_json::from_str::<Value>(r).ok()) else { return BTreeMap::new() };
    if !map.values().all(Value::is_object) {
        return BTreeMap::new();
    }
    map.into_iter()
        .map(|(k, mut v)| {
            v["busy"] = json!(false);
            (k, v)
        })
        .collect()
}

/// What a space's cwd git run needs: `git`, with `env LC_ALL=C` so "not a git repository" reads as English.
fn git_args(args: &[&str], english: bool) -> (String, Vec<String>) {
    if english {
        ("env".into(), ["LC_ALL=C", "git"].iter().chain(args).map(|s| (*s).to_owned()).collect())
    } else {
        ("git".into(), args.iter().map(|s| (*s).to_owned()).collect())
    }
}

enum Looked {
    No,
    Unsure,
    Repo(Git),
}

fn run_on(on: &Option<String>, program: &str, args: &[String], cwd: &str) -> Result<(i32, String, String), String> {
    match on {
        Some(m) => mods::run_remote(m, program, args, Some(cwd.to_owned())).map(|(c, o, e)| (c, String::from_utf8_lossy(&o).into_owned(), String::from_utf8_lossy(&e).into_owned())),
        None => mods::run_blocking(program, args, Some(cwd.to_owned()), Duration::from_secs(30)),
    }
}

/// One place's git: branch, ahead/behind, its repo. Only git's own "no" is a non-repo; anything else is asked again later.
fn look(on: Option<String>, cwd: String) -> Looked {
    let ask = |args: &[&str], english: bool| {
        let (program, args) = git_args(args, english);
        run_on(&on, &program, &args, &cwd)
    };
    let (top, common) = (ask(&["rev-parse", "--path-format=absolute", "--show-toplevel"], true), ask(&["rev-parse", "--path-format=absolute", "--git-common-dir"], true));
    let (Ok(top), Ok(common)) = (top, common) else { return Looked::Unsure };
    if top.0 != 0 || common.0 != 0 {
        let err = if top.0 != 0 { &top.2 } else { &common.2 };
        return if err.to_lowercase().contains("not a git repository") { Looked::No } else { Looked::Unsure };
    }
    let (top, repo) = (top.1.trim_end_matches('\n').to_owned(), common.1.trim_end_matches('\n').to_owned());
    let (branch, counts) = (ask(&["symbolic-ref", "-q", "HEAD"], false), ask(&["rev-list", "--left-right", "--count", "HEAD...@{upstream}"], false));
    let name = match repo.strip_suffix("/.git") {
        Some(root) => base(root),
        None => base(&repo).trim_end_matches(".git").to_owned(),
    };
    let (ahead, behind) = match &counts {
        Ok((0, out, _)) => {
            let mut n = out.split_whitespace().map(|x| x.parse::<u64>().unwrap_or(0));
            (n.next().unwrap_or(0), n.next().unwrap_or(0))
        }
        _ => (0, 0),
    };
    // The same read the Changes panel makes: no locks taken, no filesystem monitor woken.
    let dirty = ask(&["--no-optional-locks", "-c", "core.fsmonitor=false", "status", "--porcelain"], false).is_ok_and(|(code, out, _)| code == 0 && !out.trim().is_empty());
    let branch = match branch {
        Ok((0, out, _)) => out.trim().trim_start_matches("refs/heads/").to_owned(),
        _ => "(detached)".to_owned(),
    };
    Looked::Repo(Git { name, branch, ahead, behind, dirty, repo: json!([on, repo]).to_string(), linked: repo != format!("{top}/.git") })
}

// State between frames --------------------------------------------------------

struct Ordering {
    key: String,
    at: Instant,
    machines: Vec<Option<String>>,
}

/// A row pressed in the list: it becomes a drag once the pointer moves a few pixels.
#[derive(Clone, Debug)]
struct Drag {
    what: groups::Dragged,
    start: Point<Pixels>,
    active: bool,
    /// Where the pointer is (what the edge scroll and the target read).
    pos: Point<Pixels>,
    /// Where it would land now, and the key of the row the indicator is drawn on.
    over: Option<(groups::Drop, String)>,
    /// What the list was when it was pressed.
    facts: drag::Facts,
}

struct State {
    started: bool,
    icons: HashMap<String, Option<String>>,
    git: HashMap<String, Option<Git>>,
    checked: HashMap<String, Instant>,
    unsure: BTreeSet<String>,
    pending: BTreeSet<String>,
    refreshing: bool,
    /// «Refrescar proyectos» shows its feedback until then (a look at the disk can end faster than an eye can see it).
    refresh_shown: Option<Instant>,
    busy: HashMap<String, bool>,
    ready: HashMap<Option<String>, bool>,
    at: HashMap<String, String>,
    watching: BTreeSet<String>,
    shown: BTreeSet<String>,
    born: HashMap<String, Instant>,
    /// A press on a row that may become a drag, and where the drag is.
    drag: Option<Drag>,
    scroll: ScrollHandle,
    active_at: Option<usize>,
    revealed: Option<(Option<String>, Option<usize>)>,
    machines: Vec<Value>,
    reconnecting: BTreeMap<String, Value>,
    folded: BTreeSet<String>,
    bots: Vec<(String, bool)>,
    bots_open: bool,
    ordering: Option<Ordering>,
    hidden_open: bool,
    copied_login: Option<String>,
    machines_at: Option<Instant>,
    bots_at: Option<Instant>,
    seeded: bool,
    menu_open: bool,
    menu_cursor: usize,
    menu_scroll: ScrollHandle,
    menu_focus: Option<FocusHandle>,
    project_query: String,
    /// The list's keyboard cursor: the rows it can stop on (in the order they were drawn), the one it is on (a `What::key`), where each is drawn, and the focus that holds the keys.
    stops: Vec<view::What>,
    cursor: Option<String>,
    bounds: HashMap<String, Bounds<Pixels>>,
    list_focus: Option<FocusHandle>,
    /// The ring shows: the list has the focus and the last input was a key.
    list_ring: bool,
    /// The next context menu opens from the keyboard (the list's menu key), and the one that did.
    list_return: bool,
    list_menu: Option<String>,
    /// The space that was shown last frame: showing another one clears its «unread» mark.
    seen_active: Option<String>,
}

thread_local! {
    static ST: RefCell<State> = RefCell::new(State {
        started: false,
        icons: HashMap::new(),
        git: HashMap::new(),
        checked: HashMap::new(),
        unsure: BTreeSet::new(),
        pending: BTreeSet::new(),
        refreshing: false,
        refresh_shown: None,
        busy: HashMap::new(),
        ready: HashMap::new(),
        at: HashMap::new(),
        watching: BTreeSet::new(),
        shown: BTreeSet::new(),
        born: HashMap::new(),
        drag: None,
        scroll: ScrollHandle::new(),
        active_at: None,
        revealed: None,
        machines: Vec::new(),
        reconnecting: parse_reconnecting(store_get("machines.reconnecting").as_deref()),
        folded: parse_folded(store_get("machines.folded").as_deref()),
        bots: Vec::new(),
        // Bots start folded; the user's own choice is kept in the store.
        bots_open: store_get("spaces.bots.open").as_deref() == Some("1"),
        ordering: None,
        hidden_open: store_get("spaces.hidden.open").as_deref() == Some("1"),
        copied_login: None,
        machines_at: None,
        bots_at: None,
        seeded: false,
        menu_open: false,
        menu_cursor: 0,
        menu_scroll: ScrollHandle::new(),
        menu_focus: None,
        project_query: String::new(),
        stops: Vec::new(),
        cursor: None,
        bounds: HashMap::new(),
        list_focus: None,
        list_ring: false,
        list_return: false,
        list_menu: None,
        seen_active: None,
    });
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    ST.with_borrow_mut(f)
}

fn store_path() -> PathBuf {
    mods::app_dir().join("data").join("spaces").join("store.json")
}

/// The mod kept its state in this file (`data/spaces/store.json`, every value a string): so does the block.
fn store_get(key: &str) -> Option<String> {
    let file: Value = serde_json::from_str(&std::fs::read_to_string(store_path()).ok()?).ok()?;
    file.get(key)?.as_str().map(str::to_owned)
}

fn store_set(key: &str, value: String) {
    let path = store_path();
    let mut file = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).filter(Value::is_object).unwrap_or_else(|| json!({}));
    file[key] = json!(value);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, serde_json::to_string_pretty(&file).unwrap_or_default());
}

fn save_folded(s: &State) {
    let list: Vec<Value> = s.folded.iter().map(|m| if m.is_empty() { Value::Null } else { json!(m) }).collect();
    store_set("machines.folded", Value::Array(list).to_string());
}

fn save_reconnecting(s: &State) {
    store_set("machines.reconnecting", serde_json::to_string(&s.reconnecting).unwrap_or_default());
}

struct Data {
    window_active: bool,
    spaces: Vec<Value>,
    panes: Vec<Value>,
    agents: Vec<Value>,
    machines: Vec<Value>,
}

fn data() -> Data {
    mods::snapshot_with(|s| {
        let list = |v: &Value| v.as_array().cloned().unwrap_or_default();
        Data { window_active: s.window["active"] == true, spaces: list(&s.spaces), panes: list(&s.panes), agents: list(&s.agents), machines: list(&s.machines) }
    })
}

fn shared_flag(key: &str, field: &str) -> bool {
    mods::shared(key).and_then(|v| v.get(field).and_then(Value::as_bool)).unwrap_or(true)
}

fn now_ms() -> u64 {
    model::now_ms()
}

fn fill_of(color: &str) -> Hsla {
    match MARK.iter().find(|(name, _)| *name == color) {
        Some((_, hex)) => c(hex),
        None => c(color),
    }
}

pub(super) fn space_color(w: &Value) -> Hsla {
    let colors = mods::shared("space.colors");
    fill_of(colors.as_ref().and_then(|m| m.get(text(w, "id"))).and_then(Value::as_str).unwrap_or("ansi6"))
}

fn alpha(color: Hsla, a: f32) -> Hsla {
    Hsla { a, ..color }
}

// The list's view: what is saved, what the commands change -----------------------

fn shared_list(key: &str) -> Vec<String> {
    mods::shared(key).and_then(|v| v.as_array().map(|a| a.iter().filter_map(|k| k.as_str().map(str::to_owned)).collect())).unwrap_or_default()
}

fn load_view() -> View {
    View {
        group_by: GroupBy::read(mods::shared("sidebar.group_by").as_ref()),
        sort_by: SortBy::read(mods::shared("sidebar.sort_by").as_ref()),
        project_order: ProjectOrder::read(mods::shared("sidebar.project_order").as_ref()),
        filters: Filters::read(mods::shared("sidebar.filters").as_ref()),
    }
}

/// Saves what changed between two views.
fn save_view(old: &View, new: &View) {
    if old.group_by != new.group_by {
        mods::set_shared("sidebar.group_by".into(), json!(new.group_by.id()));
    }
    if old.sort_by != new.sort_by {
        mods::set_shared("sidebar.sort_by".into(), json!(new.sort_by.id()));
    }
    if old.project_order != new.project_order {
        mods::set_shared("sidebar.project_order".into(), json!(new.project_order.id()));
    }
    if old.filters != new.filters {
        mods::set_shared("sidebar.filters".into(), new.filters.to_value());
    }
}

/// The per-space lists the rules read: states, pins and the spaces bots made.
struct Lists {
    status: BTreeMap<String, Status>,
    pinned: BTreeSet<String>,
    bots: BTreeSet<String>,
}

impl Lists {
    fn load() -> Self {
        Lists { status: view::read_statuses(mods::shared("space.status").as_ref()), pinned: view::read_ids(mods::shared("space.pinned").as_ref()), bots: view::read_ids(mods::shared("space.bot_made").as_ref()) }
    }
    fn facts<'a>(&'a self, d: &'a Data) -> Ctx<'a> {
        Ctx { agents: &d.agents, now: now_ms(), status: &self.status, bot_made: &self.bots, branch_names: mods::shared("space.branch_names").is_some_and(|v| v == true) }
    }
}

/// The spaces the list shows (ordered, known, not filtered out), from every machine.
fn view_rows(d: &Data, v: &View, lists: &Lists) -> Vec<Row> {
    let rows: Vec<Row> = with(|s| ordered_all(&d.spaces, &d.panes, &s.git).into_iter().filter(|r| s.shown.contains(r.id())).collect());
    view::filter_rows(&rows, &v.filters, &lists.facts(d))
}

/// The projects as the list shows them: each group with its projects, the projects in none, and the hidden ones.
struct Arranged {
    groups: Vec<(groups::PGroup, Vec<Group>)>,
    loose: Vec<Group>,
    hidden: Vec<Group>,
}

/// Groups `rest` into projects, puts them (and the spaces in each) in the chosen order, takes the hidden ones out
/// (never the one you are in), and sets each group's projects under it.
fn arrange(d: &Data, rest: Vec<Row>, view: &View, lists: &Lists) -> Arranged {
    let k = lists.facts(d);
    let mut projects = group_projects(&rest);
    view::order_projects(&mut projects, view.project_order, &shared_list("sidebar.project_keys"));
    for p in &mut projects {
        let mut rows = std::mem::take(&mut p.rows);
        if view.sort_by != SortBy::Manual {
            view::sort_rows(&mut rows, view.sort_by, &k);
        }
        p.rows = rows;
    }
    let hidden_keys = shared_list("space.hidden");
    let (hidden, shown): (Vec<Group>, Vec<Group>) = projects.into_iter().partition(|g| hidden_keys.contains(&g.key) && !g.rows.iter().any(Row::active));
    let (groups, loose) = groups::layout(&groups::read_groups(mods::shared("sidebar.groups").as_ref()), shown);
    Arranged { groups, loose, hidden }
}

/// The keys of the headers that fold, for what the list shows now.
fn fold_keys(rows: &[Row], v: &View, lists: &Lists, d: &Data) -> Vec<String> {
    let (top, rest) = view::split_pinned(rows.to_vec(), &lists.pinned);
    let mut keys: Vec<String> = Vec::new();
    if !top.is_empty() {
        keys.push("pinned".into());
    }
    match v.group_by {
        GroupBy::Project => {
            keys.extend(groups::read_groups(mods::shared("sidebar.groups").as_ref()).into_iter().map(|g| format!("group:{}", g.id)));
            keys.extend(group_projects(&rest).into_iter().map(|g| g.key));
        }
        GroupBy::Status => keys.extend(view::by_status(&rest, &lists.facts(d)).into_iter().map(|(s, _)| format!("status:{}", s.id()))),
        GroupBy::Pr => keys.extend((!rest.is_empty()).then(|| "pr:none".to_owned())),
        GroupBy::None => {}
    }
    keys
}

fn toggle_fold(key: &str) {
    let mut list = shared_list("space.folded");
    match list.iter().position(|k| k == key) {
        Some(at) => {
            list.remove(at);
        }
        None => list.push(key.to_owned()),
    }
    mods::set_shared("space.folded".into(), json!(list));
}

/// Registers a row for the list's keyboard cursor, in the order rows are drawn; true when the ring is on it.
fn stop(what: &view::What) -> bool {
    with(|s| {
        s.stops.push(what.clone());
        s.list_ring && s.cursor.as_deref() == Some(what.key().as_str())
    })
}

/// Remembers where the row is drawn (so the list can scroll to it and a menu can open on it).
fn track(row: Stateful<Div>, what: &view::What) -> Stateful<Div> {
    let key = what.key();
    row.child(canvas(move |bounds, _, _| with(|s| s.bounds.insert(key, bounds)), |_, _, _, _| {}).absolute().size_full())
}

/// Scrolls the list just enough to show a row (the last place it was drawn).
fn list_reveal(key: &str) {
    with(|s| {
        let Some(row) = s.bounds.get(key).copied() else { return };
        let view = s.scroll.bounds();
        let by = view::scroll_delta(f32::from(view.top()), f32::from(view.bottom()), f32::from(row.top()), f32::from(row.bottom()));
        if by != 0.0 {
            let mut offset = s.scroll.offset();
            offset.y += px(by);
            s.scroll.set_offset(offset);
        }
    });
}

/// The options menu's lines and the view they show, for the spaces now.
fn menu_model() -> (View, Vec<view::Entry>) {
    let (d, v) = (data(), load_view());
    let rows: Vec<Row> = with(|s| ordered_all(&d.spaces, &d.panes, &s.git).into_iter().filter(|r| s.shown.contains(r.id())).collect());
    let mut projects: Vec<(String, String)> = group_projects(&rows).into_iter().map(|g| (g.key, g.label)).collect();
    projects.sort_by_key(|(_, name)| name.to_lowercase());
    let query = with(|s| s.project_query.clone());
    let entries = view::entries(&v, &projects, &query);
    (v, entries)
}

// The window's side -------------------------------------------------------------

impl AppWindow {
    fn has_native_spaces(&self) -> bool {
        self.chrome.visible_blocks().iter().any(|n| n.content().and_then(|c| self.native_of(c)).is_some_and(|(name, _)| name == "spaces"))
    }

    /// Once a frame: the colors of new spaces, the order spaces are put in, the agents' order, and the background work's heartbeat.
    pub(super) fn sync_spaces(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_native_spaces() {
            return;
        }
        if !with(|s| std::mem::replace(&mut s.started, true)) {
            cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor().timer(Duration::from_millis(250)).await;
                    if this.update(cx, |this, cx| this.spaces_tick(cx)).is_err() {
                        break;
                    }
                }
            })
            .detach();
        }
        // The keys: the ring shows while the list has the focus and the last input was a key.
        let focus = with(|s| s.list_focus.get_or_insert_with(|| cx.focus_handle().tab_stop(true)).clone());
        let ring = focus.is_focused(window) && window.last_input_was_keyboard();
        with(|s| s.list_ring = ring);
        let d = data();
        // Showing a space reads it.
        let shown = d.spaces.iter().find(|w| w["active"] == true).map(|w| text(w, "id").to_owned());
        if with(|s| std::mem::replace(&mut s.seen_active, shown.clone()) != shown)
            && let Some(id) = shown
            && shared_list("space.unread").contains(&id)
        {
            mods::set_shared("space.unread".into(), json!(menus::listed(shared_list("space.unread"), &id, false)));
        }
        // A space that was closed takes its place in a lineage with it (but only once every machine has said what it has).
        if d.machines.iter().all(|m| text(m, "layout") == "ready") {
            let existing: BTreeSet<&str> = d.spaces.iter().map(|w| text(w, "id")).collect();
            let saved = groups::read_parents(mods::shared("space.parents").as_ref());
            let kept = groups::prune_parents(&saved, &existing);
            if kept.len() != saved.len() {
                mods::set_shared("space.parents".into(), json!(kept));
            }
        }
        let named: Vec<(String, String)> = d.spaces.iter().map(|w| (text(w, "id").to_owned(), text(w, "name").to_owned())).collect();
        let current = match mods::shared("space.colors") {
            Some(Value::Object(m)) => m,
            _ => serde_json::Map::new(),
        };
        let (kept, changed) = assign_colors(&current, &named);
        if changed {
            mods::set_shared("space.colors".into(), Value::Object(kept));
        }
        // agent.next / agent.prev follow the order the agents are listed in here.
        let order: HashMap<&str, usize> = d.spaces.iter().enumerate().map(|(n, w)| (text(w, "id"), n)).collect();
        let mut agents: Vec<&Value> = d.agents.iter().collect();
        agents.sort_by_key(|a| order.get(text(a, "space")).copied().unwrap_or(0));
        mods::set_agent_order(agents.iter().map(|a| text(a, "address").to_owned()).collect());
        self.order_spaces(&d, window, cx);
    }

    /// Puts each worktree space under its main checkout's (and waits for the echo before trying again).
    fn order_spaces(&mut self, d: &Data, window: &mut Window, cx: &mut Context<Self>) {
        let now = Instant::now();
        let moves = with(|s| {
            for w in &d.spaces {
                let id = text(w, "id");
                if !s.shown.contains(id) && (s.git.contains_key(&where_key(w, &d.panes)) || s.born.get(id).is_some_and(|b| now.duration_since(*b) > BORN_WAIT)) {
                    s.shown.insert(id.to_owned());
                }
            }
            let out = ordered_all(&d.spaces, &d.panes, &s.git);
            let order: Vec<String> = d.spaces.iter().map(|w| text(w, "id").to_owned()).collect();
            let target: Vec<String> = out.iter().map(|r| r.id().to_owned()).collect();
            let key = json!([order, target, d.machines]).to_string();
            if order == target {
                s.ordering = None;
                return Vec::new();
            }
            if s.ordering.as_ref().is_some_and(|o| o.key == key) && !ordering_due(s, &d.machines, now) {
                return Vec::new();
            }
            let moves = plan_moves(&order, &target);
            let machine_of = |id: &str| out.iter().find(|r| r.id() == id).and_then(|r| r.machine().map(str::to_owned));
            let mut machines: Vec<Option<String>> = Vec::new();
            for (id, _) in &moves {
                let m = machine_of(id);
                if !machines.contains(&m) {
                    machines.push(m);
                }
            }
            s.ordering = Some(Ordering { key, at: now, machines });
            moves
        });
        if !moves.is_empty() {
            cx.defer_in(window, move |this, window, cx| {
                for (id, index) in moves {
                    this.run("space.move", json!({ "id": id, "index": index }), window, cx);
                }
            });
        }
    }

    /// The heartbeat (every 250 ms): looks for git changes, and asks for a redraw when a waiting space may show or the order may be retried.
    fn spaces_tick(&mut self, cx: &mut Context<Self>) {
        let d = data();
        let now = Instant::now();
        let mut redraw = false;
        let (due, refresh_machines, load_bots) = with(|s| {
            if !s.seeded {
                s.seeded = true;
                s.shown = d.spaces.iter().map(|w| text(w, "id").to_owned()).collect();
            }
            for w in &d.spaces {
                if !s.shown.contains(text(w, "id")) {
                    s.born.entry(text(w, "id").to_owned()).or_insert(now);
                }
            }
            redraw |= d.spaces.iter().any(|w| !s.shown.contains(text(w, "id"))) || ordering_due(s, &d.machines, now);
            let due = due_places(s, &d, now, d.window_active);
            let refresh_machines = s.machines_at.is_none_or(|t| now.duration_since(t) >= Duration::from_secs(15));
            let load_bots = s.bots_at.is_none_or(|t| now.duration_since(t) >= Duration::from_secs(30));
            if refresh_machines {
                s.machines_at = Some(now);
            }
            if load_bots {
                s.bots_at = Some(now);
            }
            (due, refresh_machines, load_bots)
        });
        if !due.is_empty() {
            self.refresh_git(due, cx);
        }
        if refresh_machines {
            self.refresh_machines(cx);
        }
        if load_bots {
            redraw |= with(load_bots_into);
        }
        if redraw {
            cx.notify();
        }
    }

    fn refresh_machines(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let listed = cx.background_executor().spawn(async { mods::run_blocking("hc", &["machine".into(), "ls".into(), "--json".into()], None, Duration::from_secs(20)) }).await;
            let machines: Option<Vec<Value>> = listed.ok().filter(|(code, _, _)| *code == 0).and_then(|(_, out, _)| serde_json::from_str::<Value>(&out).ok()).map(|v| items(&v, "machines").iter().filter(|m| m["temporary"] != true).cloned().collect());
            let _ = this.update(cx, |_, cx| {
                if let Some(machines) = machines {
                    with(|s| s.machines = machines);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// One look at a time, each place once however often it was asked for meanwhile.
    fn refresh_git(&mut self, keys: BTreeSet<String>, cx: &mut Context<Self>) {
        let start = with(|s| {
            s.pending.extend(keys);
            !std::mem::replace(&mut s.refreshing, true)
        });
        if !start {
            return;
        }
        cx.spawn(async move |this, cx| {
            loop {
                let keys: Vec<String> = with(|s| std::mem::take(&mut s.pending).into_iter().collect());
                if keys.is_empty() {
                    break;
                }
                for batch in keys.chunks(AT_ONCE) {
                    let jobs: Vec<_> = batch
                        .iter()
                        .map(|key| {
                            let (on, cwd) = serde_json::from_str::<(Option<String>, String)>(key).unwrap_or_default();
                            let key = key.clone();
                            cx.background_executor().spawn(async move { let looked = look(on.clone(), cwd); (key, on, looked) })
                        })
                        .collect();
                    for job in jobs {
                        let (key, on, looked) = job.await;
                        let _ = this.update(cx, |this, cx| this.git_found(key, on, looked, cx));
                    }
                }
            }
            let _ = this.update(cx, |_, cx| {
                with(|s| s.refreshing = false);
                cx.notify();
            });
        })
        .detach();
    }

    fn git_found(&mut self, key: String, on: Option<String>, looked: Looked, cx: &mut Context<Self>) {
        with(|s| {
            s.checked.insert(key.clone(), Instant::now());
            s.unsure.remove(&key);
            match &looked {
                Looked::No => {
                    s.git.insert(key.clone(), None);
                }
                Looked::Unsure => {
                    s.git.entry(key.clone()).or_insert(None);
                    s.unsure.insert(key.clone());
                }
                Looked::Repo(g) => {
                    s.git.insert(key.clone(), Some(g.clone()));
                }
            }
        });
        if let (Looked::Repo(g), None) = (&looked, on) {
            let repo: String = serde_json::from_str::<(Option<String>, String)>(&g.repo).map(|(_, r)| r).unwrap_or_default();
            self.watch_repo(repo, cx);
        }
        cx.notify();
    }

    /// Looks at a local repo's places again when its refs change.
    fn watch_repo(&mut self, repo: String, cx: &mut Context<Self>) {
        if !with(|s| s.watching.insert(repo.clone())) {
            return;
        }
        cx.spawn(async move |this, cx| {
            loop {
                if crate::files::next_change(repo.clone()).await.is_err() {
                    break;
                }
                let mine = json!([null, repo]).to_string();
                let keep = this
                    .update(cx, |this, cx| {
                        let keys: BTreeSet<String> = with(|s| s.git.iter().filter(|(_, g)| g.as_ref().is_some_and(|g| g.repo == mine)).map(|(k, _)| k.clone()).collect());
                        if !with(|s| s.watching.contains(&repo)) || keys.is_empty() {
                            return false;
                        }
                        this.refresh_git(keys, cx);
                        true
                    })
                    .unwrap_or(false);
                if !keep {
                    break;
                }
            }
            let _ = this.update(cx, |_, _| with(|s| s.watching.remove(&repo)));
        })
        .detach();
    }

    /// A project's own icon: the first app icon or logo the repo ships, copied to project-icons/ (once per project, local only).
    fn project_icon(&self, key: &str, folder: &str, machine: Option<&str>, cx: &mut Context<Self>) -> Option<String> {
        if let Some(found) = with(|s| s.icons.get(key).cloned()) {
            return found;
        }
        // The one the person chose (project.icon) stays over what the repo ships.
        if let Some(file) = mods::shared("project.icons").and_then(|m| m.get(key).and_then(Value::as_str).map(str::to_owned)).filter(|f| mods::app_dir().join("project-icons").join(f).is_file()) {
            with(|s| s.icons.insert(key.to_owned(), Some(file.clone())));
            return Some(file);
        }
        if machine.is_some() || folder.is_empty() || cfg!(windows) {
            return None;
        }
        with(|s| s.icons.insert(key.to_owned(), None));
        let out = mods::app_dir().join("project-icons").to_string_lossy().into_owned();
        let name: String = key.chars().map(|ch| if ch.is_alphanumeric() || matches!(ch, '_' | '.' | '-') { ch } else { '_' }).collect();
        let name: String = name.chars().rev().take(80).collect::<Vec<_>>().into_iter().rev().collect();
        let script = format!(
            "for f in {}; do if [ -f \"$f\" ]; then ext=\"${{f##*.}}\"; mkdir -p \"$1\" && cp \"$f\" \"$1/$2.$ext\" && echo \"$2.$ext\"; exit 0; fi; done",
            ICON_FILES.join(" ")
        );
        let (key, folder) = (key.to_owned(), folder.to_owned());
        cx.spawn(async move |this, cx| {
            let args: Vec<String> = vec!["-c".into(), script, "sh".into(), out, name];
            let ran = cx.background_executor().spawn(async move { mods::run_blocking("sh", &args, Some(folder), Duration::from_secs(5)) }).await;
            let file = ran.ok().filter(|(code, _, _)| *code == 0).map(|(_, out, _)| out.trim().to_owned()).filter(|f| !f.is_empty());
            if let Some(file) = file {
                let _ = this.update(cx, |_, cx| {
                    with(|s| s.icons.insert(key, Some(file)));
                    cx.notify();
                });
            }
        })
        .detach();
        None
    }

    fn reconnect(&mut self, machine: Value, cx: &mut Context<Self>) {
        let name = text(&machine, "name").to_owned();
        let started = with(|s| {
            if s.reconnecting.get(&name).is_some_and(|r| r["busy"] == true) {
                return false;
            }
            let state = s.reconnecting.entry(name.clone()).or_insert_with(|| json!({ "machine": machine, "removed": false }));
            state["busy"] = json!(true);
            state["error"] = Value::Null;
            save_reconnecting(s);
            true
        });
        if !started {
            return;
        }
        let removed = with(|s| s.reconnecting.get(&name).is_some_and(|r| r["removed"] == true));
        cx.notify();
        cx.spawn(async move |this, cx| {
            let n = name.clone();
            let m = machine.clone();
            let result: Result<bool, String> = cx
                .background_executor()
                .spawn(async move {
                    let ask = |args: Vec<String>| -> Result<(), String> {
                        let mut args = args;
                        args.push("--json".into());
                        let (code, out, err) = mods::run_blocking("hc", &args, None, Duration::from_secs(60))?;
                        if code != 0 {
                            let body = if out.trim().is_empty() { &err } else { &out };
                            let message = serde_json::from_str::<Value>(body).ok().and_then(|v| v["error"]["message"].as_str().map(str::to_owned));
                            return Err(message.filter(|m| !m.is_empty()).unwrap_or_else(|| if !err.is_empty() { err } else if !out.is_empty() { out } else { i18n::t("spaces.couldn_t_connect") }));
                        }
                        Ok(())
                    };
                    if !removed {
                        ask(vec!["machine".into(), "rm".into(), n.clone()])?;
                    }
                    let mut args: Vec<String> = vec!["machine".into(), "add".into()];
                    if m.get("via").is_some_and(|v| !v.is_null() && v != false) {
                        args.extend(["--via".into(), text(&m, "target").to_owned()]);
                    } else {
                        args.push(text(&m, "target").to_owned());
                    }
                    args.extend(["--name".into(), n.clone()]);
                    if let Some(session) = opt_text(&m, "session") {
                        args.extend(["--session".into(), session.to_owned()]);
                    }
                    if m["temporary"] == true {
                        args.push("--temp".into());
                    }
                    ask(args)?;
                    Ok(true)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                with(|s| {
                    match &result {
                        Ok(_) => {
                            s.folded.remove(&name);
                            save_folded(s);
                            s.reconnecting.remove(&name);
                        }
                        Err(e) => {
                            if let Some(state) = s.reconnecting.get_mut(&name) {
                                state["error"] = json!(e);
                                state["busy"] = json!(false);
                            }
                        }
                    }
                    save_reconnecting(s);
                });
                this.refresh_machines(cx);
                cx.notify();
            });
        })
        .detach();
    }
}

fn ordering_due(s: &State, machines: &[Value], now: Instant) -> bool {
    s.ordering.as_ref().is_some_and(|o| {
        now.duration_since(o.at) >= Duration::from_secs(1) && o.machines.iter().all(|name| machines.iter().any(|m| m.get("name").and_then(Value::as_str) == name.as_deref() && text(m, "layout") == "ready"))
    })
}

fn load_bots_into(s: &mut State) -> bool {
    let dir = mods::app_dir().join("bots");
    let mut names: Vec<String> = std::fs::read_dir(&dir).map(|d| d.flatten().filter(|e| e.path().is_dir()).filter_map(|e| e.file_name().into_string().ok()).filter(|n| !n.starts_with('.')).collect()).unwrap_or_default();
    names.sort();
    let next: Vec<(String, bool)> = names
        .into_iter()
        .map(|name| {
            let enabled = std::fs::read_to_string(dir.join(&name).join("bot.json")).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).is_some_and(|v| v["enabled"] != false);
            (name, enabled)
        })
        .collect();
    let changed = next != s.bots;
    s.bots = next;
    changed
}

/// Places whose git could have changed: new ones (a new space, a cd), where a command or an agent's turn just ended
/// (a checkout, a commit, edits), those on a machine that is back, and repos in view whose fallback is due.
fn due_places(s: &mut State, d: &Data, now: Instant, active: bool) -> BTreeSet<String> {
    // Moved there: what it knew of that place may be old.
    let at: HashMap<String, String> = d.spaces.iter().map(|w| (text(w, "id").to_owned(), where_key(w, &d.panes))).collect();
    let keys: BTreeSet<String> = at.values().filter(|k| serde_json::from_str::<(Option<String>, String)>(k).is_ok_and(|(_, place)| !place.is_empty())).cloned().collect();
    let stale: Vec<String> = s.git.keys().filter(|k| !keys.contains(*k)).cloned().collect();
    for k in stale {
        s.git.remove(&k);
        s.checked.remove(&k);
        s.unsure.remove(&k);
    }
    let repos: BTreeSet<String> = s.git.values().flatten().map(|g| serde_json::from_str::<(Option<String>, String)>(&g.repo).map(|(_, r)| r).unwrap_or_default()).collect();
    s.watching.retain(|r| repos.contains(r));
    let mut due: BTreeSet<String> = keys.iter().filter(|k| !s.git.contains_key(*k)).cloned().collect();
    for (id, k) in &at {
        if s.at.get(id).is_some_and(|old| old != k) && keys.contains(k) {
            due.insert(k.clone());
        }
    }
    s.at = at.clone();
    let space_of: HashMap<&str, &str> = d.spaces.iter().flat_map(|w| items(w, "tabs").iter().map(move |t| (text(t, "id"), text(w, "id")))).collect();
    let mut busy = HashMap::new();
    for p in &d.panes {
        if p.get("terminal").is_none_or(Value::is_null) {
            continue;
        }
        let Some(space) = space_of.get(text(p, "tab")) else { continue };
        let program = |k: &str| p["program"][k] == true;
        let now_busy = text(p, "status") == "working" || !(program("shell") || program("agent"));
        busy.insert(text(p, "pane").to_owned(), now_busy);
        if s.busy.get(text(p, "pane")) == Some(&true) && !now_busy
            && let Some(k) = at.get(*space)
        {
            // The place and the other places in its repo (a commit there moves their ahead/behind).
            due.insert(k.clone());
            if let Some(Some(repo)) = s.git.get(k).map(|g| g.as_ref().map(|g| g.repo.clone())) {
                due.extend(keys.iter().filter(|o| s.git.get(*o).is_some_and(|g| g.as_ref().is_some_and(|g| g.repo == repo))).cloned());
            }
        }
    }
    s.busy = busy;
    for m in &d.machines {
        let name = m.get("name").and_then(Value::as_str).map(str::to_owned);
        let ready = text(m, "layout") == "ready";
        if ready && s.ready.get(&name) == Some(&false) {
            due.extend(keys.iter().filter(|k| serde_json::from_str::<(Option<String>, String)>(k).is_ok_and(|(on, _)| on == name)).cloned());
        }
        s.ready.insert(name, ready);
    }
    if active {
        for k in &keys {
            let asks = s.git.get(k).is_some_and(Option::is_some) || s.unsure.contains(k);
            let on = serde_json::from_str::<(Option<String>, String)>(k).ok().and_then(|(on, _)| on);
            let folded = s.folded.contains(on.as_deref().unwrap_or(""));
            if asks && !folded && s.checked.get(k).is_none_or(|t| now.duration_since(*t) > FALLBACK) {
                due.insert(k.clone());
            }
        }
    }
    due
}

// Drawing --------------------------------------------------------------------

pub(super) fn hf() -> Div {
    div().flex().items_center()
}

pub(super) fn vf() -> Div {
    div().flex().flex_col()
}

/// An icon in `color` that takes `over` while the element of `group` is hovered.
fn hover_icon(bytes: &'static [u8], size: f32, color: Hsla, over: Hsla, group: &SharedString) -> Svg {
    icon(bytes, size, color).group_hover(group.clone(), move |s| s.text_color(over))
}

/// A square button with an icon (the sidebar's small buttons).
fn small_button(id: impl Into<SharedString>, bytes: &'static [u8], side: f32, icon_size: f32, hint: Option<String>) -> Stateful<Div> {
    let id: SharedString = id.into();
    div()
        .id(id.clone())
        .group(id.clone())
        .size(z(side))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(r(6.0))
        .cursor_pointer()
        .hover(|s| s.bg(c("hover")))
        .when_some(hint, |el, h| el.tooltip(tip(h)))
        .child(hover_icon(bytes, icon_size, tone("dim"), c("foreground"), &id))
}

pub(super) fn click(cx: &mut Context<AppWindow>, f: impl Fn(&mut AppWindow, &mut Window, &mut Context<AppWindow>) + 'static) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    cx.listener(move |this, _: &ClickEvent, window, cx| f(this, window, cx))
}

pub(super) fn sid(prefix: &str, id: &str) -> SharedString {
    SharedString::from(format!("{prefix}-{id}"))
}

impl AppWindow {
    pub(super) fn native_spaces(&self, cx: &mut Context<Self>) -> AnyElement {
        let d = data();
        let active = d.spaces.iter().find(|w| w["active"] == true).map(|w| text(w, "id").to_owned());
        // The rows are registered again as they are drawn; the cursor settles on what last frame drew.
        let list_focus = with(|s| {
            let last = std::mem::take(&mut s.stops);
            s.cursor = view::settle(&last, s.cursor.as_deref(), active.as_deref()).map(|i| last[i].key());
            s.list_focus.get_or_insert_with(|| cx.focus_handle().tab_stop(true)).clone()
        });
        let (items, active_at) = self.space_items(&d, cx);
        let scroll = with(|s| {
            s.active_at = active_at;
            let now = (active, active_at);
            if s.revealed.as_ref() != Some(&now) {
                if let Some(at) = active_at {
                    s.scroll.scroll_to_item(at);
                }
                s.revealed = Some(now);
            }
            s.scroll.clone()
        });
        let (view, lists) = (load_view(), Lists::load());
        let all_folded = view::all_folded(&fold_keys(&view_rows(&d, &view, &lists), &view, &lists, &d), &shared_list("space.folded"));
        let menu_open = with(|s| s.menu_open);
        let badge = view.filters.count();
        let options_hint = if badge > 0 { format!("{} ({badge})", i18n::t("sidebar.options")) } else { i18n::t("sidebar.options") };
        let options = hf()
            .relative()
            .child(
                header_button("space-options", LIST_FILTER, &options_hint, cx, |this, window, cx| this.toggle_menu(window, cx))
                    .when(menu_open || badge > 0, |el| el.bg(c("hover"))),
            )
            .when(badge > 0, |el| {
                el.child(div().absolute().top(z(1.0)).right(z(1.0)).min_w(z(15.0)).h(z(15.0)).px(z(3.0)).rounded_full().flex().items_center().justify_center().bg(c("accent")).text_size(z(10.0)).font_weight(FontWeight::BOLD).text_color(on_accent()).child(badge.to_string()))
            });
        let header = hf()
            .h(z(38.0))
            .pl(z(16.0))
            .pr(z(6.0))
            .justify_between()
            .text_size(z(11.5))
            .text_color(tone("dim"))
            .child(i18n::t("spaces.spaces"))
            .child(
                hf().gap(z(2.0))
                    .child(header_button("space-collapse-all", if all_folded { UNFOLD_ALL } else { FOLD_ALL }, &i18n::t(if all_folded { "sidebar.expand_all" } else { "sidebar.collapse_all" }), cx, |this, window, cx| this.run("sidebar.collapse_all", json!({}), window, cx)))
                    .child(options)
                    .child(header_button("space-folder", FOLDER_PLUS, &i18n::t("folders.open"), cx, |this, window, cx| this.run("project.add", json!({ "step": "browse" }), window, cx)))
                    .child(header_button("space-new", PLUS, &i18n::t("add-project.title"), cx, |this, window, cx| {
                        this.open_add_project_menu(window, cx);
                    })),
            );
        vf()
            .size_full()
            .min_h(px(0.0))
            .bg(c("chrome"))
            .text_size(z(13.0))
            .child(self.sp_top(cx))
            .child(sidebar_separator())
            .child(header)
            .when(menu_open, |el| el.child(self.options_menu(cx)))
            .child(
                vf()
                    .id("space-list")
                    .track_focus(&list_focus)
                    .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| this.list_key(e, window, cx)))
                    .flex_1()
                    .min_h(px(0.0))
                    .px(z(8.0))
                    .pb(z(14.0))
                    .overflow_y_scroll()
                    .track_scroll(&scroll)
                    .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, window, cx| this.drag_move(e, window, cx)))
                    .on_mouse_up(MouseButton::Left, cx.listener(|this, _: &MouseUpEvent, window, cx| {
                        this.drag_release(window, cx);
                    }))
                    .on_mouse_up_out(MouseButton::Left, cx.listener(|_, _: &MouseUpEvent, _, cx| {
                        with(|s| s.drag = None);
                        cx.notify();
                    }))
                    .children(items),
            )
            .into_any_element()
    }

    /// Search and Bots (fold-away): a + for a new bot, a card to start with when there are none.
    fn sp_top(&self, cx: &mut Context<Self>) -> Div {
        let (bots, bots_open) = with(|s| (s.bots.clone(), s.bots_open));
        let search = shared_flag("sidebar.buttons", "search");
        let show_bots = shared_flag("sidebar.buttons", "bots");
        let open_bots = |this: &mut Self, window: &mut Window, cx: &mut Context<Self>| {
            this.run("layout.select", json!({ "stack": "panel-body", "child": "bots" }), window, cx);
            this.run("layout.show", json!({ "id": "panel" }), window, cx);
        };
        let new_bot = move |this: &mut Self, window: &mut Window, cx: &mut Context<Self>| {
            mods::set_shared("bots.new".into(), json!(now_ms()));
            open_bots(this, window, cx);
        };
        let templates = move |this: &mut Self, window: &mut Window, cx: &mut Context<Self>| {
            mods::set_shared("bots.templates".into(), json!(now_ms()));
            open_bots(this, window, cx);
        };
        let pill = |id: &'static str, label: String, primary: bool| {
            div()
                .id(id)
                .h(z(32.0))
                .px(z(8.0))
                .flex()
                .items_center()
                .rounded(r(7.0))
                .text_size(z(12.5))
                .cursor_pointer()
                .when(primary, |el| el.bg(c("accent")).text_color(on_accent()))
                .when(!primary, |el| el.bg(c("raised")).text_color(c("foreground")).hover(|s| s.bg(c("hover"))))
                .child(label)
        };
        let faces = ["ansi6", "ansi3", "ansi5"].iter().enumerate().map(|(n, color)| div().size(z(24.0)).flex_none().rounded_full().when(n > 0, |el| el.ml(z(-8.0))).border_1().border_color(c("chrome")).bg(c(color)));
        let row_style = |el: Stateful<Div>| el.h(z(32.0)).px(z(6.0)).gap(z(9.0)).flex().items_center().flex_none().rounded(r(8.0)).cursor_pointer().text_color(tone("muted")).hover(|s| s.bg(c("hover")).text_color(c("foreground")));
        let search_row = row_style(pressable(div().id("side-search").group("side-search"), cx, |this, window, cx| this.run("app.palette", json!({}), window, cx)))
            .child(hover_icon(SEARCH, 14.0, tone("muted"), c("foreground"), &"side-search".into()))
            .child(div().child(i18n::t("spaces.search")));
        let bots_row = row_style(pressable(div().id("side-bots").group("side-bots"), cx, |_, _, cx| {
            with(|s| {
                s.bots_open = !s.bots_open;
                store_set("spaces.bots.open", if s.bots_open { "1" } else { "0" }.into());
            });
            cx.notify();
        }))
            .child(hover_icon(if bots_open { CHEVRON_DOWN } else { CHEVRON_RIGHT }, 14.0, tone("muted"), c("foreground"), &"side-bots".into()))
            .child(div().flex_1().child(i18n::t("spaces.bots")))
            .child(
                pressable(small_button("side-bot-new", PLUS, 32.0, 14.0, Some(i18n::t("bots.new"))), cx, move |this, window, cx| {
                    cx.stop_propagation();
                    new_bot(this, window, cx);
                })
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()),
            );
        let card = vf()
            .w_full()
            .flex_none()
            .mt(z(2.0))
            .p(z(10.0))
            .gap(z(8.0))
            .rounded(r(10.0))
            .border_1()
            .border_color(c("border"))
            .bg(c("surface"))
            .child(hf().gap(z(8.0)).child(hf().w(z(56.0)).flex_none().children(faces)).child(div().flex_1().min_w(px(0.0)).text_size(z(13.0)).font_weight(FontWeight::SEMIBOLD).text_color(c("foreground")).child(i18n::t("spaces.bots_teammate"))))
            .child(
                hf().gap(z(6.0))
                    .child(pressable(pill("side-bot-card-new", format!("+ {}", i18n::t("bots.new")), true), cx, move |this, window, cx| new_bot(this, window, cx)))
                    .child(pressable(pill("side-bot-templates", i18n::t("spaces.bots_templates"), false), cx, move |this, window, cx| templates(this, window, cx))),
            );
        let bot_rows: Vec<Stateful<Div>> = bots
            .iter()
            .map(|(name, enabled)| {
                pressable(hf().id(sid("side-bot", name)), cx, move |this, window, cx| open_bots(this, window, cx))
                    .h(z(32.0))
                    .pl(z(30.0))
                    .pr(z(6.0))
                    .gap(z(8.0))
                    .flex_none()
                    .rounded(r(6.0))
                    .cursor_pointer()
                    .text_size(z(12.5))
                    .text_color(tone("muted"))
                    .hover(|s| s.bg(c("hover")))
                    .child(div().size(z(7.0)).flex_none().rounded(r(4.0)).bg(if *enabled { c("done") } else { alpha(c("dim"), 0.4) }))
                    .child(div().flex_1().min_w(px(0.0)).truncate().child(name.clone()))
            })
            .collect();
        vf()
            .w_full()
            .flex_none()
            .px(z(8.0))
            .pt(z(4.0))
            .pb(z(2.0))
            .when(search, |el| el.child(search_row))
            .when(show_bots, |el| el.child(bots_row))
            .when(show_bots && bots_open && bots.is_empty(), |el| el.child(card))
            .when(show_bots && bots_open, |el| el.children(bot_rows))
    }

    /// The tile: the project's icon when the repo has one, else its first letter on its color.
    fn project_tile(&self, key: &str, label: &str, fill: Hsla, size: f32, folder: &str, machine: Option<&str>, cx: &mut Context<Self>) -> AnyElement {
        let letter = {
            let initial = label.chars().next().map_or("?".to_owned(), |ch| ch.to_uppercase().collect());
            move || hf().size(z(size)).flex_none().rounded(r(5.0)).bg(fill).justify_center().text_size(z(size * 0.58)).font_weight(FontWeight::BOLD).text_color(gpui::white()).child(initial.clone()).into_any_element()
        };
        match self.project_icon(key, folder, machine, cx) {
            // The letter holds the place while the image loads (or if it can't be read), so the row never shows a hole.
            Some(file) => div()
                .size(z(size))
                .flex_none()
                .rounded(r(5.0))
                .overflow_hidden()
                .child(img(mods::app_dir().join("project-icons").join(file)).size(z(size)).with_loading(letter.clone()).with_fallback(letter))
                .into_any_element(),
            None => letter(),
        }
    }

    /// The rows, under a header per machine once one isn't this computer. Returns the children and where the active space's row lands among them.
    fn space_items(&self, d: &Data, cx: &mut Context<Self>) -> (Vec<AnyElement>, Option<usize>) {
        let rows: Vec<Row> = with(|s| ordered_all(&d.spaces, &d.panes, &s.git).into_iter().filter(|r| s.shown.contains(r.id())).collect());
        let (machines, reconnecting, folded) = with(|s| (s.machines.clone(), s.reconnecting.clone(), s.folded.clone()));
        let mut remote: BTreeSet<String> = rows.iter().filter_map(|r| r.machine().map(str::to_owned)).collect();
        remote.extend(machines.iter().filter_map(|m| opt_text(m, "name").map(str::to_owned)));
        remote.extend(reconnecting.keys().cloned());
        let loading = |m: Option<&str>| {
            let layout = d.machines.iter().find(|k| k.get("name").and_then(Value::as_str) == m).map(|k| text(k, "layout"));
            layout == Some("loading")
                || (layout.is_none() && machines.iter().any(|k| opt_text(k, "name") == m && text(k, "state") == "connected"))
                || with(|s| d.spaces.iter().any(|w| opt_text(w, "machine") == m && !s.shown.contains(text(w, "id"))))
        };
        let loading_row = |m: Option<&str>| hf().id(SharedString::from(format!("machine-loading-{}", m.unwrap_or("local")))).h(z(28.0)).flex_none().px(z(8.0)).text_size(z(12.0)).text_color(tone("dim")).child(i18n::t("spaces.loading")).into_any_element();
        if remote.is_empty() {
            if loading(None) && rows.is_empty() {
                return (vec![loading_row(None)], None);
            }
            return self.projects(d, &rows, true, cx);
        }
        let mut out: Vec<AnyElement> = Vec::new();
        let mut active_at = None;
        let names: Vec<Option<String>> = std::iter::once(None).chain(remote.into_iter().map(Some)).collect();
        for m in names {
            let mine: Vec<Row> = rows.iter().filter(|r| r.machine() == m.as_deref()).cloned().collect();
            let repair = m.as_ref().and_then(|n| reconnecting.get(n));
            let machine: Option<Value> = machines.iter().find(|k| opt_text(k, "name") == m.as_deref()).cloned().or_else(|| repair.and_then(|r| r.get("machine").cloned()));
            let connected = m.is_none() || machine.as_ref().is_some_and(|k| text(k, "state") == "connected");
            let was_rebuilt = rebuilt(machine.as_ref()) || repair.is_some();
            let login = machine.as_ref().filter(|k| text(k, "state") == "needs_login").and_then(|_| m.as_ref().map(|n| format!("hc machine login {n}")));
            let fold_key = m.clone().unwrap_or_default();
            let is_folded = folded.contains(&fold_key);
            let gid = SharedString::from(format!("machine-{}", m.as_deref().unwrap_or("local")));
            let machine_icon = |size: f32| icon(if m.is_some() { SERVER } else { MONITOR }, size, tone("dim"));
            let state = machine.as_ref().and_then(|k| opt_text(k, "state")).map(str::to_owned);
            let dot = || div().size(z(6.0)).rounded(r(3.0)).flex_none().bg(if connected { c("ansi2") } else { tone("dim") });
            let (fold_c, m_c) = (fold_key.clone(), m.clone());
            let at = view::What::Machine(m.clone());
            let on = stop(&at);
            let mut head = ring(track(hf().id(gid.clone()), &at), on)
                .h(z(36.0))
                .flex_none()
                .px(z(6.0))
                .gap(z(7.0))
                .rounded(r(7.0))
                .text_size(z(11.0))
                .text_color(tone("dim"))
                .cursor_pointer()
                .hover(|s| s.bg(c("hover")))
                .on_click(click(cx, move |_, _, cx| {
                    with(|s| {
                        if !s.folded.remove(&fold_c) {
                            s.folded.insert(fold_c.clone());
                        }
                        save_folded(s);
                    });
                    cx.notify();
                }));
            if machine.as_ref().is_some_and(|k| k["keeps_running_after_logout"] == false) {
                head = head.child(
                    pressable(div().id(SharedString::from(format!("machine-logout-{}", m.as_deref().unwrap_or("local")))), cx, |this, window, cx| {
                        cx.stop_propagation();
                        this.open_popup(json!({ "mod": "machine-add", "kind": "dialog", "width": 560 }), window, cx);
                    })
                    .size(z(32.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .flex_none()
                    .rounded(r(6.0))
                    .cursor_pointer()
                    .text_color(tone("blocked"))
                    .tooltip(tip(i18n::t("spaces.agents_stop_when_you_disconnect")))
                    .child("⚠"),
                );
            }
            head = head.child(machine_icon(13.0)).child(div().flex_1().truncate().child(m.clone().unwrap_or_else(|| i18n::t("spaces.this_computer"))));
            // How many spaces this machine has.
            if !mine.is_empty() {
                head = head.child(div().id(SharedString::from(format!("machine-count-{}", m.as_deref().unwrap_or("local")))).flex_none().tooltip(tip(i18n::tf("spaces.machine_count", &[("n", &mine.len().to_string())]))).child(mine.len().to_string()));
            }
            if !connected && !was_rebuilt && login.is_none() {
                head = head.child(div().flex_none().child(state_word(state.as_deref())));
            }
            if was_rebuilt {
                let error = repair.and_then(|r| opt_text(r, "error")).filter(|e| !e.is_empty()).map(str::to_owned);
                let busy = repair.is_some_and(|r| r["busy"] == true);
                let target = machine.clone().unwrap_or(Value::Null);
                let shown = error.clone().unwrap_or_else(|| i18n::t("spaces.rebuilt"));
                head = head.child(
                    hf().flex_none()
                        .gap(z(3.0))
                        .child(div().max_w(z(140.0)).truncate().child(shown))
                        .child(
                            pressable(div().id(SharedString::from(format!("machine-connect-{}", m.as_deref().unwrap_or("local")))), cx, move |this, _, cx| {
                                cx.stop_propagation();
                                this.reconnect(target.clone(), cx);
                            })
                                .h(z(32.0))
                                .px(z(6.0))
                                .flex()
                                .items_center()
                                .rounded(r(6.0))
                                .cursor_pointer()
                                .text_color(if error.is_some() { tone("blocked") } else { tone("muted") })
                                .hover(|s| s.bg(c("hover")).text_color(c("foreground")))
                                .tooltip(tip(error.clone().unwrap_or_else(|| i18n::t("spaces.this_machine_was_rebuilt_or"))))
                                .child(if busy { i18n::t("spaces.connecting_2") } else { i18n::t("spaces.connect") }),
                        ),
                );
            }
            // Another machine has a menu: rename, check, manage, remove.
            if let Some(name) = m.clone() {
                let (right, more) = (name.clone(), name);
                head = head
                    .on_mouse_down(MouseButton::Right, cx.listener(move |this, _: &MouseDownEvent, window, cx| this.machine_menu(&right, window, cx)))
                    .child(action_button(SharedString::from(format!("machine-more-{more}")), menus::ELLIPSIS, &i18n::t("ctx.machine_actions"), cx, {
                        let more = more.clone();
                        move |this, window, cx| {
                            cx.stop_propagation();
                            this.machine_menu(&more, window, cx);
                        }
                    }));
            }
            if connected {
                head = head.child(
                    pressable(div().id(SharedString::from(format!("machine-add-{}", m.as_deref().unwrap_or("local")))), cx, move |this, window, cx| {
                        cx.stop_propagation();
                        this.run("space.new", json!({ "machine": m_c.clone().unwrap_or_default() }), window, cx);
                    })
                    .size(z(32.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .flex_none()
                    .rounded(r(6.0))
                    .cursor_pointer()
                    .tooltip(tip(i18n::t("spaces.new_space_there")))
                    .hover(|s| s.bg(c("hover")).text_color(c("foreground")))
                    .child("+"),
                );
            }
            head = head.child(dot()).child(icon(if is_folded { CHEVRON_RIGHT } else { CHEVRON_DOWN }, 11.0, tone("dim")));
            if was_rebuilt {
                out.push(head.into_any_element());
                continue;
            }
            let before = out.len();
            out.push(head.into_any_element());
            if let Some(login) = &login {
                let copy = login.clone();
                let copied = with(|s| s.copied_login.as_deref() == Some(login.as_str()));
                let machine_name = m.clone().unwrap_or_default();
                out.push(
                    vf().px(z(8.0)).pb(z(6.0)).gap(z(3.0)).flex_none()
                        .child(div().text_size(z(11.0)).text_color(tone("blocked")).child(i18n::t("spaces.needs_a_login_run")))
                        .child(
                            hf().gap(z(5.0))
                                .child(div().id(SharedString::from(format!("machine-login-{machine_name}"))).flex_1().min_w(px(0.0)).truncate().tooltip(tip(login.clone())).text_size(z(11.0)).text_color(tone("muted")).child(login.clone()))
                                .child(
                                    pressable(div().id(SharedString::from(format!("machine-login-copy-{machine_name}"))), cx, move |_, _, cx| {
                                        cx.stop_propagation();
                                        cx.write_to_clipboard(ClipboardItem::new_string(copy.clone()));
                                        with(|s| s.copied_login = Some(copy.clone()));
                                        cx.notify();
                                    })
                                        .h(z(32.0))
                                        .flex()
                                        .items_center()
                                        .flex_none()
                                        .px(z(8.0))
                                        .rounded(r(6.0))
                                        .text_size(z(11.0))
                                        .text_color(c("foreground"))
                                        .bg(c("raised"))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(c("hover")))
                                        .child(i18n::t(if copied { "spaces.copied" } else { "spaces.copy" })),
                                ),
                        )
                        .into_any_element(),
                );
            }
            if is_folded {
                continue;
            }
            if mine.is_empty() {
                if loading(m.as_deref()) {
                    out.push(loading_row(m.as_deref()));
                } else {
                    let name = m.clone();
                    let new = pressable(hf().id(SharedString::from(format!("machine-new-{}", m.as_deref().unwrap_or("local")))), cx, move |this, window, cx| this.run("space.new", json!({ "machine": name.clone().unwrap_or_default() }), window, cx))
                        .h(z(32.0))
                        .flex_none()
                        .px(z(6.0))
                        .gap(z(7.0))
                        .rounded(r(7.0))
                        .text_size(z(12.0))
                        .text_color(tone("dim"))
                        .cursor_pointer()
                        .hover(|s| s.bg(c("hover")).text_color(c("foreground")))
                        .child(icon(PLUS, 12.0, tone("dim")))
                        .child(i18n::t("spaces.new_space_there"));
                    out.push(new.into_any_element());
                }
                continue;
            }
            let (items, first) = self.projects(d, &mine, m.is_none(), cx);
            if let Some(first) = first {
                active_at = Some(before + 1 + usize::from(login.is_some()) + first);
            }
            out.extend(items);
        }
        (out, active_at)
    }

    /// The list under the view (Group by, Sort by, Filters): the pinned ones on top, then projects, states, or one flat list.
    fn projects(&self, d: &Data, rows: &[Row], local: bool, cx: &mut Context<Self>) -> (Vec<AnyElement>, Option<usize>) {
        let (view, lists) = (load_view(), Lists::load());
        let k = lists.facts(d);
        let visible = view::filter_rows(rows, &view.filters, &k);
        if visible.is_empty() && !rows.is_empty() {
            return (vec![self.no_results(cx)], None);
        }
        let (top, rest) = view::split_pinned(visible, &lists.pinned);
        let sorted = |mut v: Vec<Row>| {
            if view.sort_by != SortBy::Manual {
                view::sort_rows(&mut v, view.sort_by, &k);
            }
            v
        };
        let folded_keys = shared_list("space.folded");
        let mut items: Vec<AnyElement> = Vec::new();
        let mut active = None;
        let lead_gap = usize::from(!top.is_empty());
        if !top.is_empty() {
            let top = sorted(top);
            let fold = folded_keys.contains(&"pinned".to_owned());
            items.push(self.section_header("pinned", i18n::t("sidebar.pinned"), Some(PIN), top.len(), fold, cx));
            if !fold {
                self.push_rows(&mut items, &mut active, &top, 18.0, d, cx);
            }
        }
        match view.group_by {
            GroupBy::None => {
                if lead_gap > 0 {
                    items.push(sidebar_separator().into_any_element());
                }
                let rest = sorted(rest);
                self.push_rows(&mut items, &mut active, &rest, 4.0, d, cx);
                return (items, active);
            }
            GroupBy::Status => {
                for (n, (status, group)) in view::by_status(&rest, &k).into_iter().enumerate() {
                    if n + lead_gap > 0 {
                        items.push(sidebar_separator().into_any_element());
                    }
                    let (key, group) = (format!("status:{}", status.id()), sorted(group));
                    let fold = folded_keys.contains(&key);
                    items.push(self.section_header(&key, i18n::t(status.label()), None, group.len(), fold, cx));
                    if !fold {
                        self.push_rows(&mut items, &mut active, &group, 18.0, d, cx);
                    }
                }
                return (items, active);
            }
            // The pull request's state needs `gh`, which Sunna doesn't run: every space is «no pull request» for now.
            GroupBy::Pr => {
                if !rest.is_empty() {
                    if lead_gap > 0 {
                        items.push(sidebar_separator().into_any_element());
                    }
                    let rest = sorted(rest);
                    let fold = folded_keys.contains(&"pr:none".to_owned());
                    items.push(self.section_header("pr:none", i18n::t("sidebar.no_pr"), None, rest.len(), fold, cx));
                    if !fold {
                        self.push_rows(&mut items, &mut active, &rest, 18.0, d, cx);
                    }
                }
                return (items, active);
            }
            GroupBy::Project => {}
        }
        let Arranged { groups: pgroups, loose, hidden } = arrange(d, rest, &view, &lists);
        let hidden_keys = shared_list("space.hidden");
        // A project folds to its header; its agents' state stays on it as a dot. A group holds its projects one level in.
        for (pg, projects) in &pgroups {
            // The groups are the person's, on this computer: another machine shows one only where it has projects.
            if projects.is_empty() && !local {
                continue;
            }
            if !items.is_empty() {
                items.push(sidebar_separator().into_any_element());
            }
            let fold = folded_keys.contains(&format!("group:{}", pg.id));
            items.push(self.project_group_header(pg, projects, fold, cx));
            if fold {
                continue;
            }
            if projects.is_empty() {
                items.push(hf().id(sid("group-empty", &pg.id)).h(z(32.0)).flex_none().pl(z(30.0)).pr(z(8.0)).text_size(z(12.0)).text_color(tone("muted")).child(i18n::t("groups.empty")).into_any_element());
            }
            for grp in projects {
                self.project_block(&mut items, &mut active, grp, 12.0, &folded_keys, d, cx);
            }
        }
        for grp in &loose {
            if !items.is_empty() {
                items.push(sidebar_separator().into_any_element());
            }
            self.project_block(&mut items, &mut active, grp, 0.0, &folded_keys, d, cx);
        }
        if !hidden.is_empty() {
            let open = with(|s| s.hidden_open);
            items.push(sidebar_separator().into_any_element());
            items.push(
                pressable(hf().id("hidden-projects"), cx, move |_, _, cx| {
                    with(|s| s.hidden_open = !open);
                    store_set("spaces.hidden.open", if open { "0" } else { "1" }.into());
                    cx.notify();
                })
                    .h(z(32.0))
                    .flex_none()
                    .px(z(6.0))
                    .gap(z(6.0))
                    .rounded(r(7.0))
                    .cursor_pointer()
                    .text_size(z(12.0))
                    .text_color(tone("dim"))
                    .hover(|s| s.bg(c("hover")).text_color(c("foreground")))
                    .child(icon(if open { CHEVRON_DOWN } else { CHEVRON_RIGHT }, 12.0, tone("dim")))
                    .child(div().child(format!("{} ({})", i18n::t("spaces.hidden_projects"), hidden.len())))
                    .into_any_element(),
            );
            if open {
                for grp in &hidden {
                    let (key, keys) = (grp.key.clone(), hidden_keys.clone());
                    items.push(
                        hf().id(sid("hidden", &grp.key))
                            .h(z(32.0))
                            .flex_none()
                            .pl(z(28.0))
                            .pr(z(8.0))
                            .gap(z(8.0))
                            .text_size(z(12.0))
                            .text_color(tone("muted"))
                            .child(div().flex_1().min_w(px(0.0)).truncate().child(grp.label.clone()))
                            .child(
                                pressable(hf().id(sid("hidden-show", &grp.key)), cx, move |_, _, _| mods::set_shared("space.hidden".into(), json!(keys.iter().filter(|k| **k != key).collect::<Vec<_>>())))
                                    .px(z(6.0))
                                    .h(z(32.0))
                                    .rounded(r(6.0))
                                    .cursor_pointer()
                                    .text_color(tone("dim"))
                                    .hover(|s| s.bg(c("hover")).text_color(c("foreground")))
                                    .child(i18n::t("spaces.show")),
                            )
                            .into_any_element(),
                    );
                }
            }
        }
        (items, active)
    }

    /// A project's header and, unless it is folded, its spaces (and their agents) one level in. `indent` is how far a group sets it.
    fn project_block(&self, items: &mut Vec<AnyElement>, active: &mut Option<usize>, grp: &Group, indent: f32, folded_keys: &[String], d: &Data, cx: &mut Context<Self>) {
        let lead = grp.rows[0].w.clone();
        let fold = folded_keys.contains(&grp.key);
        let ids: BTreeSet<&str> = grp.rows.iter().map(Row::id).collect();
        let state = agent_state(&d.agents, &ids);
        let repo = grp.rows[0].g.is_some();
        let (fold_keys, key_c, key_menu, key_more) = (folded_keys.to_vec(), grp.key.clone(), grp.key.clone(), grp.key.clone());
        let tile = self.project_tile(&grp.key, &grp.label, space_color(&lead), 18.0, text(&lead, "folder"), lead_machine(&lead), cx);
        let wt_lead = lead.clone();
        let at = view::What::Project(grp.key.clone());
        let on = stop(&at);
        items.push(
            drag::drop_style(ring(track(hf().id(sid("project", &grp.key)), &at), on), &at.key())
                .h(z(36.0))
                .flex_none()
                .pl(z(2.0 + indent))
                .pr(z(6.0))
                .mt(z(6.0))
                .gap(z(7.0))
                .rounded(r(7.0))
                .cursor_pointer()
                .hover(|s| s.bg(c("hover")))
                .on_mouse_down(MouseButton::Left, press_listener(groups::Dragged::Project(grp.key.clone()), cx))
                .on_click(click(cx, move |_, _, _| {
                    if drag::swallows_click() {
                        return;
                    }
                    let next: Vec<String> = if fold { fold_keys.iter().filter(|k| **k != key_c).cloned().collect() } else { fold_keys.iter().cloned().chain([key_c.clone()]).collect() };
                    mods::set_shared("space.folded".into(), json!(next));
                }))
                .on_mouse_down(MouseButton::Right, cx.listener(move |this, _: &MouseDownEvent, window, cx| this.project_menu_for(&key_menu, window, cx)))
                .child(icon(if fold { CHEVRON_RIGHT } else { CHEVRON_DOWN }, 14.0, tone("dim")))
                .child(tile)
                .child(div().flex_1().min_w(px(0.0)).truncate().font_weight(FontWeight::SEMIBOLD).text_color(c("foreground")).child(grp.label.clone()))
                .when(fold && state.is_some(), |el| el.child(div().size(z(7.0)).flex_none().rounded_full().bg(c(state.unwrap_or("working")))))
                // The project's actions (the same menu as the right-click).
                .child(action_button(sid("project-more", &grp.key), menus::ELLIPSIS, &i18n::t("ctx.project_actions"), cx, move |this, window, cx| {
                    cx.stop_propagation();
                    this.project_menu_for(&key_more, window, cx);
                }))
                // A repo's project can start another worktree from here.
                .when(repo, |el| {
                    el.child(pressable(small_button(sid("project-worktree", &grp.key), BRANCH, 32.0, 14.0, Some(i18n::t("menu.new_worktree"))), cx, move |this, window, cx| {
                        cx.stop_propagation();
                        this.open_worktree(json!({ "space": text(&wt_lead, "id"), "folder": wt_lead["folder"], "action": "new" }), window, cx);
                    }))
                })
                .into_any_element(),
        );
        if !fold {
            self.push_rows(items, active, &grp.rows, 18.0 + indent, d, cx);
        }
    }

    /// A group's header: fold chevron, its colored folder, name, how many spaces it holds, and its menu.
    fn project_group_header(&self, pg: &groups::PGroup, projects: &[Group], fold: bool, cx: &mut Context<Self>) -> AnyElement {
        let spaces: usize = projects.iter().map(|p| p.rows.len()).sum();
        let (key, menu_id, more_id) = (format!("group:{}", pg.id), pg.id.clone(), pg.id.clone());
        let at = view::What::Group(pg.id.clone());
        let on = stop(&at);
        let name = if pg.name.is_empty() { i18n::t("groups.untitled") } else { pg.name.clone() };
        drag::drop_style(ring(track(hf().id(sid("group", &pg.id)), &at), on), &at.key())
            .h(z(36.0))
            .flex_none()
            .px(z(4.0))
            .gap(z(7.0))
            .rounded(r(7.0))
            .cursor_pointer()
            .hover(|s| s.bg(c("hover")))
            .on_mouse_down(MouseButton::Left, press_listener(groups::Dragged::Group(pg.id.clone()), cx))
            .on_click(click(cx, move |_, _, _| {
                if !drag::swallows_click() {
                    toggle_fold(&key);
                }
            }))
            .on_mouse_down(MouseButton::Right, cx.listener(move |this, _: &MouseDownEvent, window, cx| this.group_menu(&menu_id, window, cx)))
            .child(icon(if fold { CHEVRON_RIGHT } else { CHEVRON_DOWN }, 14.0, tone("dim")))
            .child(icon(if fold { FOLDER_CLOSED } else { FOLDER_OPEN }, 15.0, fill_of(&pg.color)))
            .child(div().id(sid("group-name", &pg.id)).flex_1().min_w(px(0.0)).truncate().text_size(z(12.0)).font_weight(FontWeight::SEMIBOLD).text_color(c("foreground")).tooltip(tip(name.to_string())).child(name))
            .child(if with(|s| s.refreshing || s.refresh_shown.is_some_and(|t| t > Instant::now())) {
                div().id(sid("group-count", &pg.id)).flex_none().tooltip(tip(i18n::t("groups.refreshing"))).child(icon(SPINNER, 12.0, tone("dim")).with_animation(SharedString::from(format!("group-refresh-{}", pg.id)), Animation::new(Duration::from_millis(900)).repeat(), |s, delta| s.with_transformation(Transformation::rotate(percentage(delta)))))
            } else {
                div().id(sid("group-count", &pg.id)).flex_none().text_size(z(11.0)).text_color(tone("dim")).tooltip(tip(i18n::tf("groups.count_tip", &[("projects", &projects.len().to_string()), ("spaces", &spaces.to_string())]))).child(spaces.to_string())
            })
            .child(action_button(sid("group-more", &pg.id), menus::ELLIPSIS, &i18n::t("groups.actions"), cx, move |this, window, cx| {
                cx.stop_propagation();
                this.group_menu(&more_id, window, cx);
            }))
            .into_any_element()
    }

    /// The space's row, and under it its agents: status, program icon, title, how long ago.
    fn with_agents(&self, row: &Row, d: &Data, cx: &mut Context<Self>) -> AnyElement {
        // The space first: the keyboard cursor visits it before its agents.
        let head = self.space_row(row, d, cx);
        let mine: Vec<&Value> = d.agents.iter().filter(|a| text(a, "space") == row.id()).collect();
        if mine.is_empty() || !shared_flag("space.card", "agents") {
            return head;
        }
        let now = now_ms();
        let changed: HashMap<&str, u64> = items(&row.w, "tabs").iter().map(|t| (text(t, "id"), t.get("changed_at").and_then(Value::as_u64).unwrap_or(0))).collect();
        let agent_rows: Vec<AnyElement> = mine
            .iter()
            .map(|a| {
                let program = &a["program"];
                let pane = text(a, "pane").to_owned();
                let status = text(a, "status");
                let mark: AnyElement = match status {
                    "working" => icon(SPINNER, 12.0, c("working")).mx(z(1.0)).with_animation(SharedString::from(format!("agent-spin-{}", text(a, "address"))), Animation::new(Duration::from_millis(900)).repeat(), |s, delta| s.with_transformation(Transformation::rotate(percentage(delta)))).into_any_element(),
                    "blocked" => hf().size(z(14.0)).flex_none().rounded(r(4.0)).bg(alpha(c("blocked"), 0.15)).justify_center().text_size(z(10.5)).font_weight(FontWeight::BOLD).text_color(tone("blocked")).child("!").into_any_element(),
                    "idle" => div().size(z(7.0)).mx(z(3.0)).flex_none().rounded(r(4.0)).border(z(1.5)).border_color(c("idle")).into_any_element(),
                    "done" => div().size(z(7.0)).mx(z(3.0)).flex_none().rounded(r(4.0)).bg(c("done")).into_any_element(),
                    _ => div().size(z(7.0)).mx(z(3.0)).flex_none().into_any_element(),
                };
                let glyph: AnyElement = match opt_text(program, "icon") {
                    Some(path) if program["tint"] != false => svg().external_path(mods::app_dir().join(path).to_string_lossy().into_owned()).size(z(14.0)).flex_none().text_color(tone("muted")).into_any_element(),
                    Some(path) => img(mods::app_dir().join(path)).w(z(14.0)).h(z(14.0)).flex_none().into_any_element(),
                    None => div().size(z(14.0)).flex_none().into_any_element(),
                };
                let title = Some(text(a, "title")).filter(|t| !t.is_empty()).unwrap_or_else(|| text(program, "name")).to_owned();
                let at = view::What::Agent(pane.clone());
                let on = stop(&at);
                ring(track(hf().id(sid("space-agent", text(a, "address"))), &at), on)
                    .h(z(32.0))
                    .pl(z(28.0))
                    .pr(z(6.0))
                    .gap(z(7.0))
                    .flex_none()
                    .rounded(r(7.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(c("hover")))
                    .text_size(z(12.0))
                    .text_color(if a["focused"] == true { c("foreground") } else { tone("muted") })
                    .on_click(click(cx, move |this, window, cx| this.run("pane.focus", json!({ "pane": pane.clone() }), window, cx)))
                    .child(mark)
                    .child(glyph)
                    .child(div().flex_1().min_w(px(0.0)).truncate().child(title))
                    .child(div().flex_none().text_size(z(11.0)).text_color(tone("dim")).child(ago(changed.get(text(a, "tab")).copied().unwrap_or(0), now)))
                    .into_any_element()
            })
            .collect();
        vf().w_full().flex_none().child(head).children(agent_rows).into_any_element()
    }

    fn space_row(&self, row: &Row, _d: &Data, cx: &mut Context<Self>) -> AnyElement {
        let w = &row.w;
        let id = text(w, "id").to_owned();
        let on = row.active();
        let label = shown_name(w, row.g.as_ref(), mods::shared("space.branch_names").is_some_and(|v| v == true));
        let git_on = shared_flag("space.card", "git");
        let ago_on = shared_flag("space.card", "ago");
        let facts = if git_on { git_facts(&label, row.g.as_ref()) } else { None };
        let status = text(w, "status");
        let (up_id, menu_id) = (id.clone(), id.clone());
        let unread = view::read_ids(mods::shared("space.unread").as_ref()).contains(&id);
        let note = mods::shared("space.notes").and_then(|m| m.get(&id).and_then(Value::as_str).map(str::to_owned));
        let at = view::What::Space(id.clone());
        let at_cursor = stop(&at);
        drag::drop_style(ring(track(vf().relative().id(sid("space", &id)), &at), at_cursor), &at.key())
            .px(z(6.0))
            .h(z(if facts.is_some() { 42.0 } else { 32.0 }))
            .flex_none()
            .justify_center()
            .rounded(r(8.0))
            // The shown space keeps its quiet outline; the keyboard's ring (accent, drawn by `ring`) wins over it.
            .when(on && !at_cursor, |el| el.border_color(c("border")))
            .when(on, |el| el.bg(c("selected")))
            .when(!on, |el| el.hover(|s| s.bg(c("hover"))))
            .on_mouse_down(MouseButton::Left, press_listener(groups::Dragged::Space(id.clone()), cx))
            // A press that did not become a drag shows the space.
            .on_mouse_up(MouseButton::Left, cx.listener(move |this, _: &MouseUpEvent, window, cx| {
                if drag::dragging() || !with(|s| s.drag.as_ref().is_some_and(|d| d.what == groups::Dragged::Space(up_id.clone()))) {
                    return;
                }
                this.run("space.select", json!({ "id": up_id }), window, cx);
                cx.notify();
            }))
            .on_mouse_down(MouseButton::Right, cx.listener(move |this, e: &MouseDownEvent, window, cx| this.space_menu(&menu_id, e.modifiers.alt, window, cx)))
            .when_some(note, |el, n| el.tooltip(tip(n)))
            .child(
                hf().gap(z(9.0))
                    .child(icon(if row.g.as_ref().is_some_and(|g| g.linked) { GIT_BRANCH } else { BRANCH }, 13.0, if status == "blocked" { c("blocked") } else { tone("dim") }))
                    .child(div().flex_1().min_w(px(0.0)).truncate().text_color(c("foreground")).child(label))
                    .when(unread, |el| el.child(div().id(sid("space-unread", &id)).size(z(8.0)).flex_none().rounded_full().bg(c("accent")).tooltip(tip(i18n::t("ctx.unread")))))
                    .when(ago_on, |el| el.child(div().flex_none().text_size(z(11.0)).text_color(tone("dim")).child(ago(changed_at(w), now_ms())))),
            )
            .when_some(facts, |el, f| {
                el.child(
                    hf().id(sid("space-git", &id))
                        .pl(z(22.0))
                        .gap(z(8.0))
                        .h(z(14.0))
                        .mt(z(2.0))
                        .text_size(z(11.0))
                        .text_color(tone("dim"))
                        .when_some(f.branch, |r, b| r.child(div().min_w(px(0.0)).truncate().child(b)))
                        .when(f.ahead > 0, |r| r.child(div().flex_none().text_color(c("accent")).child(format!("↑{}", f.ahead))))
                        .when(f.behind > 0, |r| r.child(div().flex_none().text_color(tone("muted")).child(format!("↓{}", f.behind))))
                        .when(f.dirty, |r| r.child(div().id(sid("space-dirty", &id)).size(z(6.0)).flex_none().rounded_full().bg(c("working")).tooltip(tip(i18n::t("spaces.uncommitted"))))),
                )
            })
            .into_any_element()
    }

}

/// A press on a row that may turn into a drag.
fn press_listener(what: groups::Dragged, cx: &mut Context<AppWindow>) -> impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static {
    cx.listener(move |_, e: &MouseDownEvent, _, _| drag::press(what.clone(), e.position, &data(), &load_view()))
}

/// Makes an element a control for the keyboard too: a tab stop that wears the accent ring while it has the keys
/// (`ring` keeps the 2 px of border, so the element is built 2 px tighter) and that Enter or Space press, like a click.
fn pressable(el: Stateful<Div>, cx: &mut Context<AppWindow>, act: impl Fn(&mut AppWindow, &mut Window, &mut Context<AppWindow>) + Clone + 'static) -> Stateful<Div> {
    let key_act = act.clone();
    ring(el, false)
        .focusable()
        .tab_stop(true)
        .focus_visible(|s| s.border_color(graphic("accent")))
        .on_key_down(cx.listener(move |this, e: &KeyDownEvent, window, cx| {
            if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                key_act(this, window, cx);
                cx.stop_propagation();
            }
        }))
        .on_click(click(cx, act))
}

/// A header button: 32 px, a ring while it has the keyboard, Enter or Space press it.
fn header_button(id: &'static str, bytes: &'static [u8], hint: &str, cx: &mut Context<AppWindow>, act: impl Fn(&mut AppWindow, &mut Window, &mut Context<AppWindow>) + Clone + 'static) -> Stateful<Div> {
    let id: SharedString = id.into();
    pressable(div().id(id.clone()).group(id.clone()).size(z(32.0)).flex_none().flex().items_center().justify_center().rounded(r(7.0)).cursor_pointer().hover(|s| s.bg(c("hover"))).tooltip(tip(hint.to_owned())), cx, act)
        .child(hover_icon(bytes, 15.0, tone("muted"), c("foreground"), &id))
}

/// A 32 px icon button in a header (the «⋯» of a project or machine): a ring while it has the keyboard, Enter or Space press it.
fn action_button(id: SharedString, bytes: &'static [u8], hint: &str, cx: &mut Context<AppWindow>, act: impl Fn(&mut AppWindow, &mut Window, &mut Context<AppWindow>) + Clone + 'static) -> Stateful<Div> {
    pressable(div().id(id.clone()).group(id.clone()).size(z(32.0)).flex_none().flex().items_center().justify_center().rounded(r(7.0)).cursor_pointer().hover(|s| s.bg(c("hover"))).tooltip(tip(hint.to_owned())), cx, act)
        .child(hover_icon(bytes, 15.0, tone("muted"), c("foreground"), &id))
}

/// One cell of the lineage guides: a 1.5 px stroke (3:1 on the sidebar, in every theme) down the cell's middle,
/// and the elbow that reaches the row. Drawn, not typed, so the strokes join from row to row.
fn guide_cell(glyph: &'static str) -> Div {
    let (st, ink) = (groups::stroke(glyph), side_graphic("dim"));
    div()
        .relative()
        .w(z(14.0))
        .flex_none()
        .self_stretch()
        .when(st.full, |d| d.child(div().absolute().left(z(6.0)).w(z(1.5)).top(px(0.0)).bottom(px(0.0)).bg(ink)))
        .when(st.half, |d| d.child(div().absolute().left(z(6.0)).w(z(1.5)).top(px(0.0)).h(z(16.75)).bg(ink)))
        .when(st.elbow, |d| d.child(div().absolute().left(z(6.0)).right(px(0.0)).top(z(16.0)).h(z(1.5)).bg(ink)))
}

impl AppWindow {
    /// A fold-away heading over a set of spaces (Pinned, a state, «no PR»).
    fn section_header(&self, key: &str, label: String, icon_bytes: Option<&'static [u8]>, count: usize, fold: bool, cx: &mut Context<Self>) -> AnyElement {
        let toggle_key = key.to_owned();
        let toggle = move |_: &mut AppWindow, _: &mut Window, _: &mut Context<AppWindow>| {
            if !drag::swallows_click() {
                toggle_fold(&toggle_key);
            }
        };
        let at = view::What::Section(key.to_owned());
        let on = stop(&at);
        drag::drop_style(ring(track(hf().id(sid("section", key)), &at), on), &at.key())
            .h(z(32.0))
            .flex_none()
            .mt(z(4.0))
            .px(z(4.0))
            .gap(z(7.0))
            .rounded(r(7.0))
            .cursor_pointer()
            .hover(|s| s.bg(c("hover")))
            .on_click(click(cx, toggle))
            .child(icon(if fold { CHEVRON_RIGHT } else { CHEVRON_DOWN }, 14.0, tone("dim")))
            .when_some(icon_bytes, |el, b| el.child(icon(b, 13.0, tone("muted"))))
            .child(div().flex_1().min_w(px(0.0)).truncate().text_size(z(12.0)).font_weight(FontWeight::SEMIBOLD).text_color(tone("muted")).child(label))
            .child(div().flex_none().text_size(z(11.0)).text_color(tone("dim")).child(count.to_string()))
            .into_any_element()
    }

    /// The spaces' rows, each child under its parent (when both are here) with the guides that join them.
    fn push_rows(&self, items: &mut Vec<AnyElement>, active: &mut Option<usize>, rows: &[Row], indent: f32, d: &Data, cx: &mut Context<Self>) {
        let parents = groups::read_parents(mods::shared("space.parents").as_ref());
        for n in groups::nest(rows, &parents) {
            if n.row.active() {
                *active = Some(items.len());
            }
            let cells = groups::guides(&n).into_iter().map(guide_cell);
            let line = hf().w_full().children(cells).child(div().flex_1().min_w(px(0.0)).child(self.with_agents(&n.row, d, cx)));
            items.push(vf().w_full().flex_none().pl(z(indent)).child(line).into_any_element());
        }
    }

    /// Every space is hidden by the filters.
    fn no_results(&self, cx: &mut Context<Self>) -> AnyElement {
        vf().id("space-no-results")
            .w_full()
            .flex_none()
            .items_center()
            .gap(z(10.0))
            .px(z(12.0))
            .py(z(20.0))
            .child(icon(LIST_FILTER, 18.0, tone("dim")))
            .child(div().text_size(z(12.5)).text_color(tone("muted")).child(i18n::t("sidebar.no_results")))
            .child(
                pressable(hf().id("space-clear-filters"), cx, |this, window, cx| this.run("sidebar.filter", json!({ "clear": true }), window, cx))
                    .h(z(32.0))
                    .px(z(10.0))
                    .rounded(r(7.0))
                    .cursor_pointer()
                    .bg(c("raised"))
                    .text_size(z(12.5))
                    .text_color(c("foreground"))
                    .hover(|s| s.bg(c("hover")))
                    .child(i18n::t("sidebar.clear_filters")),
            )
            .into_any_element()
    }

    fn toggle_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focus = with(|s| {
            s.menu_open = !s.menu_open;
            s.menu_cursor = 0;
            s.project_query.clear();
            s.menu_open.then(|| s.menu_focus.get_or_insert_with(|| cx.focus_handle().tab_stop(true)).clone())
        });
        if let Some(focus) = focus {
            focus.focus(window, cx);
        }
        cx.notify();
    }

    /// Runs one line of the options menu.
    fn menu_act(&mut self, act: view::Act, window: &mut Window, cx: &mut Context<Self>) {
        if act == view::Act::Reveal {
            with(|s| s.menu_open = false);
            self.run("sidebar.reveal_current", json!({}), window, cx);
            return;
        }
        let old = load_view();
        save_view(&old, &view::apply(&old, &act));
        cx.notify();
    }

    /// Keys in the options menu: arrows move, Enter or Space picks, Esc closes, letters search the projects.
    fn menu_key(&mut self, e: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = e.keystroke.key.as_str();
        let (_, entries) = menu_model();
        let acts: Vec<(usize, view::Act)> = entries.iter().enumerate().filter_map(|(i, en)| en.act().map(|a| (i, a.clone()))).collect();
        let cursor = with(|s| s.menu_cursor).min(acts.len().saturating_sub(1));
        let typing = with(|s| !s.project_query.is_empty());
        let m = &e.keystroke.modifiers;
        match key {
            "escape" => with(|s| s.menu_open = false),
            "enter" => {
                if let Some((_, act)) = acts.get(cursor) {
                    self.menu_act(act.clone(), window, cx);
                }
            }
            "space" if !typing => {
                if let Some((_, act)) = acts.get(cursor) {
                    self.menu_act(act.clone(), window, cx);
                }
            }
            "backspace" => with(|s| {
                s.project_query.pop();
            }),
            _ => {
                if let Some(to) = step_cursor(cursor, key, acts.len()) {
                    with(|s| {
                        s.menu_cursor = to;
                        s.menu_scroll.scroll_to_item(acts[to].0);
                    });
                } else if let Some(ch) = e.keystroke.key_char.as_ref().filter(|c| !m.control && !m.platform && !m.alt && c.chars().all(|ch| !ch.is_control())) {
                    with(|s| s.project_query.push_str(ch));
                } else {
                    return;
                }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    /// The options menu: Group by, Sort by, Project order, Filters, under the header.
    fn options_menu(&self, cx: &mut Context<Self>) -> AnyElement {
        let (_, entries) = menu_model();
        let (cursor, scroll, focus) = with(|s| (s.menu_cursor, s.menu_scroll.clone(), s.menu_focus.get_or_insert_with(|| cx.focus_handle().tab_stop(true)).clone()));
        let total = entries.iter().filter(|e| e.act().is_some()).count();
        let cursor = cursor.min(total.saturating_sub(1));
        let mut n = 0usize;
        let mut rows: Vec<AnyElement> = Vec::new();
        for entry in &entries {
            match entry {
                view::Entry::Heading(label) if label.is_empty() => rows.push(fade_line()),
                view::Entry::Heading(label) => rows.push(div().flex_none().px(z(8.0)).pt(z(8.0)).pb(z(2.0)).text_size(z(11.0)).font_weight(FontWeight::SEMIBOLD).text_color(tone("muted")).child(label.clone()).into_any_element()),
                view::Entry::Note(text) => rows.push(div().flex_none().px(z(8.0)).py(z(6.0)).text_size(z(12.0)).text_color(tone("muted")).child(text.clone()).into_any_element()),
                view::Entry::Search(query) => rows.push(
                    hf().id("sb-project-search")
                        .h(z(32.0))
                        .flex_none()
                        .mx(z(2.0))
                        .px(z(8.0))
                        .gap(z(8.0))
                        .rounded(r(7.0))
                        .border_1()
                        .border_color(c("border"))
                        .text_size(z(12.5))
                        .child(icon(SEARCH, 13.0, tone("muted")))
                        .child(if query.is_empty() {
                            div().flex_1().text_color(tone("muted")).child(i18n::t("sidebar.search_project"))
                        } else {
                            div().flex_1().min_w(px(0.0)).truncate().text_color(c("foreground")).child(query.clone())
                        })
                        .into_any_element(),
                ),
                view::Entry::Item { act, label, on, radio } => {
                    let mine = n;
                    n += 1;
                    let act_click = act.clone();
                    rows.push(
                        ring(hf().id(sid("sb-opt", &mine.to_string())).h(z(32.0)).flex_none().px(z(8.0)).gap(z(9.0)).rounded(r(7.0)).cursor_pointer().text_size(z(12.5)).text_color(c("foreground")).hover(|s| s.bg(c("hover"))), cursor == mine)
                            .on_click(click(cx, move |this, window, cx| {
                                with(|s| s.menu_cursor = mine);
                                this.menu_act(act_click.clone(), window, cx);
                            }))
                            .child(div().size(z(14.0)).flex_none().flex().items_center().justify_center().when(*on, |d| d.child(icon(CHECK, 14.0, c("accent")))).when(!*on && !*radio && *act != view::Act::Reveal, |d| d.child(icon(CIRCLE, 13.0, tone("dim")))).when(*act == view::Act::Reveal, |d| d.child(icon(LOCATE, 14.0, tone("muted")))))
                            .child(div().flex_1().min_w(px(0.0)).truncate().child(label.clone()))
                            .into_any_element(),
                    );
                }
            }
        }
        vf().id("sb-menu")
            .flex_none()
            .mx(z(8.0))
            .mb(z(6.0))
            .rounded(r(8.0))
            .bg(c("popover"))
            .border_1()
            .border_color(c("border"))
            .child(
                vf().id("sb-menu-scroll")
                    .track_focus(&focus)
                    .p(z(5.0))
                    .max_h(z(360.0))
                    .overflow_y_scroll()
                    .track_scroll(&scroll)
                    .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| this.menu_key(e, window, cx)))
                    .children(rows),
            )
            .into_any_element()
    }

    /// Keys on the list while it has the focus: arrows, Home, End, PageUp and PageDown move the cursor; Enter or
    /// Space shows the space (or folds the header, or focuses the agent); Right and Left unfold and fold a header;
    /// the menu key or Shift+F10 opens the row's menu.
    fn list_key(&mut self, e: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let (key, m) = (e.keystroke.key.as_str(), &e.keystroke.modifiers);
        if key == "escape" && drag::cancel() {
            cx.stop_propagation();
            cx.notify();
            return;
        }
        let (stops, cursor, focus) = with(|s| (s.stops.clone(), s.cursor.clone(), s.list_focus.clone()));
        // A button inside the list has its own keys; the plain list reads only its own.
        if !focus.is_some_and(|f| f.is_focused(window)) || m.control || m.platform || stops.is_empty() {
            return;
        }
        let active = data().spaces.iter().find(|w| w["active"] == true).map(|w| text(w, "id").to_owned());
        // Alt+Up and Alt+Down move the row the cursor is on (a space, a project or a group) one place.
        if m.alt {
            if m.shift || !matches!(key, "up" | "down") {
                return;
            }
            let Some(at) = view::settle(&stops, cursor.as_deref(), active.as_deref()) else { return };
            let what = stops[at].clone();
            self.list_move(&what, key == "down", window, cx);
            cx.stop_propagation();
            return;
        }
        if !m.shift
            && let Some(to) = view::cursor_to(&stops, cursor.as_deref(), active.as_deref(), key)
        {
            let at = stops[to].key();
            with(|s| s.cursor = Some(at.clone()));
            list_reveal(&at);
            cx.stop_propagation();
            cx.notify();
            return;
        }
        let Some(at) = view::settle(&stops, cursor.as_deref(), active.as_deref()) else { return };
        let what = stops[at].clone();
        match key {
            "enter" | "space" if !m.shift => self.list_activate(&what, window, cx),
            "right" if what.folds() && !m.shift => self.list_fold(&what, Some(false)),
            "left" if what.folds() && !m.shift => self.list_fold(&what, Some(true)),
            "menu" => self.list_menu(&what, window, cx),
            "f10" if m.shift => self.list_menu(&what, window, cx),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn list_activate(&mut self, what: &view::What, window: &mut Window, cx: &mut Context<Self>) {
        match what {
            view::What::Space(id) => self.run("space.select", json!({ "id": id }), window, cx),
            view::What::Agent(pane) => self.run("pane.focus", json!({ "pane": pane }), window, cx),
            _ => self.list_fold(what, None),
        };
    }

    /// Folds (`Some(true)`), unfolds (`Some(false)`) or flips (`None`) a header.
    fn list_fold(&mut self, what: &view::What, fold: Option<bool>) {
        let folded = match what {
            view::What::Project(key) | view::What::Section(key) => shared_list("space.folded").contains(key),
            view::What::Group(id) => shared_list("space.folded").contains(&format!("group:{id}")),
            view::What::Machine(name) => with(|s| s.folded.contains(name.as_deref().unwrap_or(""))),
            _ => return,
        };
        if fold.is_some_and(|f| f == folded) {
            return;
        }
        match what {
            view::What::Project(key) | view::What::Section(key) => toggle_fold(key),
            view::What::Group(id) => toggle_fold(&format!("group:{id}")),
            view::What::Machine(name) => with(|s| {
                let key = name.clone().unwrap_or_default();
                if !s.folded.remove(&key) {
                    s.folded.insert(key);
                }
                save_folded(s);
            }),
            _ => {}
        }
    }

    /// Opens the row's menu just under it, so it reads as the row's own.
    fn list_menu(&mut self, what: &view::What, window: &mut Window, cx: &mut Context<Self>) {
        let under = with(|s| s.bounds.get(&what.key()).map(|b| point(b.left() + b.size.width * 0.4, b.bottom() - px(4.0))));
        if let Some(under) = under {
            self.pointer = under;
        }
        with(|s| s.list_return = true);
        match what {
            view::What::Space(id) => self.space_menu(id, false, window, cx),
            view::What::Project(key) => self.project_menu_for(key, window, cx),
            view::What::Machine(Some(name)) => self.machine_menu(name, window, cx),
            view::What::Group(id) => self.group_menu(id, window, cx),
            _ => with(|s| s.list_return = false),
        }
    }

    /// The menu `overlay` that was just opened came from the list's keys (true): when it closes by a key, the list gets them back.
    pub(super) fn opened_from_list(overlay: &str) -> bool {
        with(|s| {
            let from_list = std::mem::take(&mut s.list_return);
            s.list_menu = from_list.then(|| overlay.to_owned());
            from_list
        })
    }

    /// An overlay closed: if it was the list's menu and a key closed it, the keys go back to the list (a click lands where it clicked).
    pub(super) fn list_menu_closed(&self, overlay: &str, window: &mut Window, cx: &mut App) {
        let mine = with(|s| (s.list_menu.as_deref() == Some(overlay)).then(|| s.list_menu.take()).is_some());
        if mine && window.last_input_was_keyboard() {
            self.focus_list(window, cx);
        }
    }

    /// Gives the keys to the list (after its menu closed with Esc).
    pub(super) fn focus_list(&self, window: &mut Window, cx: &mut App) {
        if let Some(focus) = with(|s| s.list_focus.clone()) {
            focus.focus(window, cx);
        }
    }

    /// Shows the shown space: opens what folds it and scrolls to it.
    fn reveal_current(&mut self) {
        let d = data();
        let Some(row) = with(|s| ordered_all(&d.spaces, &d.panes, &s.git)).into_iter().find(Row::active) else { return };
        let lists = Lists::load();
        // The headings that can hold it: its pin section, or its project, state and «no PR» group.
        let mut open = vec![project_key(&row), "pr:none".to_owned(), format!("status:{}", lists.facts(&d).status_of(&row).id())];
        let pgs = groups::read_groups(mods::shared("sidebar.groups").as_ref());
        open.extend(groups::group_of(&pgs, &project_key(&row)).map(|g| format!("group:{}", g.id)));
        if lists.pinned.contains(row.id()) {
            open = vec!["pinned".to_owned()];
        }
        let folded: Vec<String> = shared_list("space.folded").into_iter().filter(|k| !open.contains(k)).collect();
        mods::set_shared("space.folded".into(), json!(folded));
        with(|s| s.revealed = None);
    }

    /// `sidebar.*`, `space.status` and `space.pin`: they only change what `shared()` keeps, so every client of this window shows it.
    pub(super) fn sidebar_command(&mut self, name: &str, a: &Value) -> Result<(), String> {
        let flag = |k: &str| -> Result<Option<bool>, String> { a.get(k).map(|v| v.as_bool().ok_or(format!("{k} must be true or false"))).transpose() };
        let d = data();
        match name {
            "sidebar.collapse_all" => {
                let (view, lists) = (load_view(), Lists::load());
                let keys = fold_keys(&view_rows(&d, &view, &lists), &view, &lists, &d);
                mods::set_shared("space.folded".into(), json!(view::toggle_folded(&keys, &shared_list("space.folded"))));
            }
            "sidebar.reveal_current" => self.reveal_current(),
            "sidebar.view" => {
                let old = load_view();
                let mut new = old.clone();
                if let Some(v) = a.get("group_by") {
                    new.group_by = v.as_str().and_then(GroupBy::parse).ok_or("group_by must be none, status, pr or project")?;
                }
                if let Some(v) = a.get("sort_by") {
                    new.sort_by = v.as_str().and_then(SortBy::parse).ok_or("sort_by must be name, agents, recent, project or manual")?;
                }
                if let Some(v) = a.get("project_order") {
                    new.project_order = v.as_str().and_then(ProjectOrder::parse).ok_or("project_order must be manual or recent")?;
                }
                save_view(&old, &new);
            }
            "sidebar.filter" => {
                let old = load_view();
                let mut new = old.clone();
                if a.get("clear").and_then(Value::as_bool) == Some(true) {
                    new.filters = Filters::default();
                }
                if let Some(v) = flag("hide_idle")? {
                    new.filters.hide_idle = v;
                }
                if let Some(v) = flag("hide_default_branch")? {
                    new.filters.hide_default_branch = v;
                }
                if let Some(v) = flag("hide_bots")? {
                    new.filters.hide_bots = v;
                }
                if let Some(v) = a.get("projects") {
                    new.filters.projects = v.as_array().ok_or("projects must be a list of project keys")?.iter().filter_map(|k| k.as_str().map(str::to_owned)).collect();
                }
                save_view(&old, &new);
            }
            "space.status" => {
                let id = a.get("id").and_then(Value::as_str).filter(|id| d.spaces.iter().any(|w| text(w, "id") == *id)).ok_or("no such space")?;
                let status = a.get("status").and_then(Value::as_str).and_then(Status::parse).ok_or("status must be todo, in_progress, in_review or done")?;
                let mut map = mods::shared("space.status").and_then(|v| v.as_object().cloned()).unwrap_or_default();
                if status == Status::default() {
                    map.remove(id);
                } else {
                    map.insert(id.to_owned(), json!(status.id()));
                }
                mods::set_shared("space.status".into(), Value::Object(map));
            }
            "space.pin" => {
                let id = a.get("id").and_then(Value::as_str).filter(|id| d.spaces.iter().any(|w| text(w, "id") == *id)).ok_or("no such space")?;
                let pinned = flag("pinned")?.ok_or("pinned must be true or false")?;
                let mut list = shared_list("space.pinned");
                list.retain(|k| k != id);
                if pinned {
                    list.push(id.to_owned());
                }
                mods::set_shared("space.pinned".into(), json!(list));
            }
            _ => return Err(format!("unknown command {name:?}")),
        }
        Ok(())
    }
}

/// The menu's divider: the sidebar's own fade.
fn fade_line() -> AnyElement {
    super::native::fade(4.0).my(z(4.0)).into_any_element()
}

fn lead_machine(w: &Value) -> Option<&str> {
    opt_text(w, "machine")
}

#[cfg(test)]
mod tests {
    use super::{BTreeMap, BTreeSet, Data, Git, HashMap, Instant, Row, State, Value, agent_state, ago, assign_colors, base, due_places, git_facts, group_projects, json, ordered_all, parse_folded, parse_reconnecting, place, plan_moves, rebuilt, shown_name, state_word, where_key};
    use std::time::Duration;

    fn space(id: &str, name: &str, folder: &str, machine: Option<&str>, active: bool) -> Value {
        json!({ "id": id, "name": name, "folder": folder, "machine": machine, "active": active, "tabs": [{ "id": format!("t-{id}") }] })
    }
    fn git(name: &str, branch: &str, repo: &str, linked: bool) -> Git {
        Git { name: name.into(), branch: branch.into(), ahead: 0, behind: 0, dirty: false, repo: json!([null, repo]).to_string(), linked }
    }
    fn known(spaces: &[Value], gits: &[(&str, Git)]) -> HashMap<String, Option<Git>> {
        spaces.iter().zip(gits).map(|(w, (_, g))| (where_key(w, &[]), Some(g.clone()))).collect()
    }
    fn ids(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(Row::id).collect()
    }
    fn state() -> State {
        super::ST.with(|_| ());
        State {
            started: false, icons: HashMap::new(), git: HashMap::new(), checked: HashMap::new(), unsure: BTreeSet::new(), pending: BTreeSet::new(), refreshing: false, refresh_shown: None, busy: HashMap::new(), ready: HashMap::new(),
            at: HashMap::new(), watching: BTreeSet::new(), shown: BTreeSet::new(), born: HashMap::new(), drag: None, scroll: gpui::ScrollHandle::new(), active_at: None, revealed: None,
            machines: Vec::new(), reconnecting: BTreeMap::new(), folded: BTreeSet::new(), bots: Vec::new(), bots_open: false, ordering: None, hidden_open: false, copied_login: None, machines_at: None, bots_at: None, seeded: false,
            menu_open: false, menu_cursor: 0, menu_scroll: gpui::ScrollHandle::new(), menu_focus: None, project_query: String::new(), stops: Vec::new(), cursor: None, bounds: HashMap::new(), list_focus: None, list_ring: false, list_return: false, list_menu: None, seen_active: None,
        }
    }

    #[test]
    fn a_second_line_shows_only_what_the_name_does_not_say() {
        assert_eq!(git_facts("api", None), None);
        assert_eq!(git_facts("api", Some(&git("api", "api", "/r/.git", false))), None);
        let g = Git { ahead: 2, behind: 1, dirty: true, ..git("api", "feat", "/r/.git", false) };
        let facts = git_facts("api", Some(&g)).unwrap();
        assert_eq!((facts.branch.as_deref(), facts.ahead, facts.behind, facts.dirty), (Some("feat"), 2, 1, true));
        // Only dirty: no branch line, still a second line.
        let dirty = Git { dirty: true, ..git("api", "api", "/r/.git", false) };
        assert_eq!(git_facts("api", Some(&dirty)).unwrap().branch, None);
    }

    #[test]
    fn an_unnamed_space_shows_its_branch_when_settings_say_so() {
        let w = space("s1", "api", "/w/api", None, false);
        let g = git("api", "feat/login", "/w/api/.git", false);
        assert_eq!(shown_name(&w, Some(&g), true), "feat/login");
        assert_eq!(shown_name(&w, Some(&g), false), "api");
        assert_eq!(shown_name(&space("s2", "mine", "/w/api", None, false), Some(&g), true), "mine");
        assert_eq!(shown_name(&w, Some(&git("api", "(detached)", "/w/api/.git", false)), true), "api");
    }

    #[test]
    fn a_worktree_goes_under_its_main_checkout_and_orphans_stay_in_order() {
        let spaces = [space("a", "app", "/r/app", None, true), space("b", "other", "/r/other", None, false), space("c", "app-wt", "/r/app-wt", None, false), space("d", "orphan", "/r/orphan", None, false)];
        let gits = [("a", git("app", "main", "/r/app/.git", false)), ("b", git("other", "main", "/r/other/.git", false)), ("c", git("app", "feat", "/r/app/.git", true)), ("d", git("gone", "feat", "/r/gone/.git", true))];
        let rows = ordered_all(&spaces, &[], &known(&spaces, &gits));
        assert_eq!(ids(&rows), ["a", "c", "b", "d"]);
        // The moves that get the model there, as indexes after the earlier moves.
        let order: Vec<String> = ["a", "b", "c", "d"].iter().map(|s| s.to_string()).collect();
        let target: Vec<String> = ids(&rows).iter().map(|s| s.to_string()).collect();
        assert_eq!(plan_moves(&order, &target), [("c".to_owned(), 2)]);
        assert!(plan_moves(&target, &target).is_empty());
        // Without git nothing moves.
        assert_eq!(ids(&ordered_all(&spaces, &[], &HashMap::new())), ["a", "b", "c", "d"]);
    }

    #[test]
    fn projects_group_by_repo_else_by_folder_and_keep_the_first_seen_order() {
        let spaces = [space("a", "app", "/r/app", None, false), space("b", "notes", "/n/notes", None, false), space("c", "wt", "/r/wt", None, false), space("d", "plain", "", None, false)];
        let gits = [("a", git("app", "main", "/r/app/.git", false)), ("b", git("notes", "x", "/n/.git", false)), ("c", git("app", "feat", "/r/app/.git", true)), ("d", git("plain", "x", "/p/.git", false))];
        let mut map = known(&spaces, &gits);
        // `b` is no repo: it groups by its folder.
        map.insert(where_key(&spaces[1], &[]), None);
        let groups = group_projects(&ordered_all(&spaces, &[], &map));
        let shown: Vec<(&str, Vec<&str>)> = groups.iter().map(|g| (g.label.as_str(), ids(&g.rows))).collect();
        assert_eq!(shown, [("app", vec!["a", "c"]), ("notes", vec!["b"]), ("plain", vec!["d"])]);
        assert_eq!(groups[1].key, "folder:/n/notes");
    }

    #[test]
    fn a_space_is_where_its_first_terminal_is_else_where_it_was_made() {
        let w = space("a", "app", "/made", Some("box"), false);
        assert_eq!(place(&w, &[]), "/made");
        let panes = [json!({ "tab": "other", "cwd": "/x" }), json!({ "tab": "t-a", "cwd": "" }), json!({ "tab": "t-a", "cwd": "/now" })];
        assert_eq!(place(&w, &panes), "/now");
        assert_eq!(where_key(&w, &panes), r#"["box","/now"]"#);
    }

    #[test]
    fn new_spaces_get_the_least_used_color_and_closed_ones_lose_theirs() {
        let current: serde_json::Map<String, Value> = serde_json::from_value(json!({ "a": "ansi6", "gone": "ansi3" })).unwrap();
        let named = [("a".to_owned(), "app".to_owned()), ("b".to_owned(), "api".to_owned()), ("h".to_owned(), "~".to_owned())];
        let (kept, changed) = assign_colors(&current, &named);
        assert!(changed);
        assert_eq!(kept["a"], "ansi6");
        assert_eq!(kept["b"], "ansi3");
        assert!(!kept.contains_key("gone") && !kept.contains_key("h"));
        assert!(!assign_colors(&kept, &named).1);
    }

    #[test]
    fn how_long_ago_reads_now_minutes_hours_days() {
        let now = 10_000_000_000;
        assert_eq!(ago(0, now), "");
        assert_eq!(ago(now - 30_000, now), "now");
        assert_eq!(ago(now - 5 * 60_000, now), "5m");
        assert_eq!(ago(now - 3 * 3_600_000, now), "3h");
        assert_eq!(ago(now - 13 * 86_400_000, now), "13d");
        assert_eq!(base("/a/b/"), "b");
    }

    #[test]
    fn a_project_shows_blocked_over_working_over_nothing() {
        let agents = [json!({ "space": "a", "status": "working" }), json!({ "space": "b", "status": "blocked" }), json!({ "space": "c", "status": "idle" })];
        let set = |ids: &[&'static str]| ids.iter().copied().collect::<BTreeSet<&str>>();
        assert_eq!(agent_state(&agents, &set(&["a"])), Some("working"));
        assert_eq!(agent_state(&agents, &set(&["a", "b"])), Some("blocked"));
        assert_eq!(agent_state(&agents, &set(&["c"])), None);
    }

    #[test]
    fn saved_machine_state_recovers_from_bad_files_and_keeps_good_ones() {
        for raw in ["{{{not json", "null", "42", "\"old shape\"", "[]", r#"{"cloud":null}"#, r#"{"cloud":42}"#] {
            assert!(parse_reconnecting(Some(raw)).is_empty(), "{raw}");
        }
        // A reconnect that was cut off is ready to retry.
        let kept = parse_reconnecting(Some(r#"{"cloud":{"busy":true,"removed":true}}"#));
        assert_eq!((kept["cloud"]["busy"].clone(), kept["cloud"]["removed"].clone()), (json!(false), json!(true)));
        for raw in ["{{{not json", "null", "42", "{}"] {
            assert!(parse_folded(Some(raw)).is_empty(), "{raw}");
        }
        assert_eq!(parse_folded(Some("\"old shape\"")), "old shape".chars().map(String::from).collect::<BTreeSet<_>>());
        assert_eq!(parse_folded(Some(r#"["cloud",null]"#)), BTreeSet::from(["cloud".to_owned(), String::new()]));
    }

    #[test]
    fn a_machine_that_answers_as_another_core_is_rebuilt() {
        let m = json!({ "name": "box", "state": "unreachable", "why": "a different core answers there (\"new\")" });
        assert!(rebuilt(Some(&m)));
        assert!(!rebuilt(Some(&json!({ "state": "unreachable", "why": "timeout" }))));
        assert!(!rebuilt(None));
        assert_eq!(state_word(Some("needs_login")), crate::i18n::t("spaces.needs_login"));
        assert_eq!(state_word(Some("some_new_state")), "some new state");
    }

    #[test]
    fn places_are_looked_at_when_new_moved_or_after_a_command_ends() {
        let mut s = state();
        let now = Instant::now();
        let w = space("a", "app", "/made", None, true);
        let mut d = Data { window_active: false, spaces: vec![w.clone()], panes: Vec::new(), agents: Vec::new(), machines: Vec::new() };
        let key = where_key(&w, &[]);
        // New: due.
        assert_eq!(due_places(&mut s, &d, now, false), BTreeSet::from([key.clone()]));
        s.git.insert(key.clone(), Some(git("app", "main", "/made/.git", false)));
        s.checked.insert(key.clone(), now);
        assert!(due_places(&mut s, &d, now, false).is_empty());
        // The fallback only runs while the window is in front.
        let later = now + Duration::from_secs(61);
        assert!(due_places(&mut s, &d, later, false).is_empty());
        assert_eq!(due_places(&mut s, &d, later, true), BTreeSet::from([key.clone()]));
        // A command that ends in a terminal of the space.
        d.panes = vec![json!({ "pane": "p1", "terminal": "t1", "tab": "t-a", "status": null, "program": { "shell": false } })];
        due_places(&mut s, &d, now, false);
        s.checked.insert(key.clone(), now);
        d.panes[0]["program"]["shell"] = json!(true);
        assert!(due_places(&mut s, &d, now, false).contains(&key));
        // Moved (a cd): the new place is due, the old one is forgotten.
        d.panes[0]["cwd"] = json!("/elsewhere");
        let moved = where_key(&w, &d.panes);
        assert!(due_places(&mut s, &d, now, false).contains(&moved));
        assert!(!s.git.contains_key(&key));
    }

    #[test]
    fn every_text_the_spaces_block_shows_exists_in_both_locales() {
        let keys = ["spaces.agents_stop_when_you_disconnect", "spaces.asleep", "spaces.bots", "spaces.bots_teammate", "spaces.bots_templates", "spaces.connect", "spaces.connected", "spaces.connecting", "spaces.connecting_2", "spaces.copied",
            "spaces.copy", "spaces.couldn_t_connect", "spaces.disconnected", "spaces.hidden_projects", "spaces.loading", "spaces.uncommitted", "spaces.name_needs_a_login_run", "spaces.needs_a_login_run", "spaces.needs_login", "spaces.needs_update",
            "spaces.new_space_on_name", "spaces.new_space_there", "spaces.rebuilt", "spaces.reconnecting", "spaces.search", "spaces.show", "spaces.spaces", "spaces.this_computer", "spaces.this_computer_2", "spaces.unreachable",
            "bots.new", "folders.open", "add-project.title", "menu.new_worktree", "workspace-chrome.help"];
        for lang in ["en", "es"] {
            let file = format!("{}/defaults/locales/{lang}.json", env!("CARGO_MANIFEST_DIR"));
            let table: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
            for k in keys {
                assert!(table[k].as_str().is_some_and(|s| !s.is_empty()), "{lang}:{k}");
            }
        }
    }
}
