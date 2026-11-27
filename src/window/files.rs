//! The Files panel (`{"native": "files"}`, once the `files` mod): a lazy tree of the
//! focused pane's folder. A file opens in the editor (`file.open`); git's status
//! marks changed files (A new, D deleted, U conflict, M changed); what `.gitignore`
//! names is hidden unless Settings > Look says otherwise; on another machine the
//! folders are listed through the core's exec. Listing, marks and sorting are plain
//! functions (tested at the end); the view only draws them.

use super::native::{fade, icon, keyed, r, ring, step_cursor, tip, tone, z};
use super::*;
use crate::i18n;
use gpui::{UniformListScrollHandle, uniform_list};
use std::collections::BTreeSet;

macro_rules! file_icon {
    ($name:literal) => {
        include_bytes!(concat!("../../assets/icons/files/", $name, ".svg")).as_slice()
    };
}
const CHEVRON_DOWN: &[u8] = file_icon!("chevron-down");
const CHEVRON_RIGHT: &[u8] = file_icon!("chevron-right");
const FOLDER: &[u8] = file_icon!("folder");
const FILE: &[u8] = file_icon!("file");

pub(super) const PER_FOLDER: usize = 500;
pub(super) const TOTAL: usize = 5000;
const ROW_H: f32 = 32.0;
const DEPTH_MAX: usize = 40;

/// Where a panel looks: the focused pane's folder and machine (`None`: this computer), and the space.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Place {
    pub cwd: Option<String>,
    pub on: Option<String>,
    pub space: Option<String>,
}

impl Place {
    /// The folder as the disk spells it (a link such as /tmp resolved), so git's marks and the open
    /// folders agree whichever pane (a terminal's real path, a space's typed one) is focused.
    pub(super) fn normalized(mut self) -> Self {
        if self.on.is_none() && !cfg!(windows) && let Some(real) = self.cwd.as_ref().and_then(|c| std::fs::canonicalize(c).ok()) {
            self.cwd = Some(real.to_string_lossy().into_owned());
        }
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Entry {
    pub name: String,
    pub dir: bool,
}

// Running programs ----------------------------------------------------------------

/// A program's `(exit code, stdout, stderr)`, here or on another machine, from `cwd`.
pub(super) fn exec(on: Option<&str>, command: &str, args: &[&str], cwd: &str, secs: u64) -> Result<(i32, String, String), String> {
    let args: Vec<String> = args.iter().map(|s| (*s).to_owned()).collect();
    let limit = Duration::from_secs(secs);
    match on {
        Some(machine) => mods::run_remote_with_limit(machine, command, &args, Some(cwd.to_owned()), limit)
            .map(|(code, out, err)| (code, String::from_utf8_lossy(&out).into_owned(), String::from_utf8_lossy(&err).into_owned())),
        None => mods::run_blocking(command, &args, Some(cwd.to_owned()), limit),
    }
}

/// Reads write nothing under .git (no index refresh or lock, no fsmonitor cookie): the folder
/// watch would see it, and every refresh would start the next one.
pub(super) const READ: [&str; 6] = ["--no-optional-locks", "-c", "core.fsmonitor=false", "-c", "diff.autoRefreshIndex=false", "--no-pager"];

// Pure rules ------------------------------------------------------------------------

/// Folders first, then names without regard to case.
fn sort_entries(entries: &mut [Entry]) {
    entries.sort_by(|a, b| b.dir.cmp(&a.dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())).then_with(|| a.name.cmp(&b.name)));
}

/// `kind\0name\0` pairs as the listing script prints them (`d` a folder, `f` a file).
fn parse_listing(out: &str) -> Vec<Entry> {
    let fields: Vec<&str> = out.split('\0').collect();
    fields.chunks_exact(2).filter(|c| !c[1].is_empty()).map(|c| Entry { name: c[1].to_owned(), dir: c[0] == "d" }).collect()
}

/// `{name, dir}` objects from PowerShell's ConvertTo-Json (one object when there is one entry).
fn parse_powershell(out: &str) -> Result<Vec<Entry>, String> {
    let value: Value = serde_json::from_str(if out.trim().is_empty() { "[]" } else { out }).map_err(|e| e.to_string())?;
    let list = match value {
        Value::Array(a) => a,
        one => vec![one],
    };
    Ok(list.iter().filter_map(|v| Some(Entry { name: v.get("name")?.as_str()?.to_owned(), dir: v.get("dir").and_then(Value::as_bool).unwrap_or(false) })).collect())
}

/// What a status line means for the tree: A new, U conflict, D deleted, M the rest.
pub(super) fn mark_of(xy: &str) -> char {
    if xy == "??" || xy.contains('A') {
        'A'
    } else if xy.contains('U') {
        'U'
    } else if xy.contains('D') {
        'D'
    } else {
        'M'
    }
}

/// The marks of `git status -z` output, keyed by `top/path`.
fn parse_marks(top: &str, out: &str) -> HashMap<String, char> {
    out.split('\0').filter(|e| e.len() > 3).take(TOTAL).map(|e| (format!("{top}/{}", &e[3..]), mark_of(&e[..2]))).collect()
}

/// One folder's entries as the tree keeps them: sorted, within what is left of the total.
/// The second value says the folder or the total was cut.
fn limit_entries(mut entries: Vec<Entry>, used: usize) -> (Vec<Entry>, bool) {
    sort_entries(&mut entries);
    let room = PER_FOLDER.min(TOTAL.saturating_sub(used));
    let cut = entries.len() > room || used + entries.len() > TOTAL;
    entries.truncate(room);
    (entries, cut)
}

#[derive(Clone, Debug, PartialEq)]
struct Row {
    path: String,
    name: String,
    depth: usize,
    dir: bool,
    open: bool,
    mark: Option<char>,
}

/// The visible rows: folders that are open show their children, up to the total.
fn flatten(entries: &HashMap<String, Vec<Entry>>, open: &BTreeSet<String>, marks: &HashMap<String, char>, root: &str) -> Vec<Row> {
    fn walk(prefix: &str, depth: usize, entries: &HashMap<String, Vec<Entry>>, open: &BTreeSet<String>, marks: &HashMap<String, char>, root: &str, out: &mut Vec<Row>) {
        if out.len() >= TOTAL || depth > DEPTH_MAX {
            return;
        }
        for entry in entries.get(prefix).into_iter().flatten() {
            if out.len() >= TOTAL {
                break;
            }
            let path = if prefix.is_empty() { entry.name.clone() } else { format!("{prefix}/{}", entry.name) };
            let is_open = open.contains(&path);
            let mark = marks.get(&format!("{root}/{path}")).copied();
            out.push(Row { path: path.clone(), name: entry.name.clone(), depth, dir: entry.dir, open: is_open, mark });
            if entry.dir && is_open {
                walk(&path, depth + 1, entries, open, marks, root, out);
            }
        }
    }
    let mut out = Vec::new();
    walk("", 0, entries, open, marks, root, &mut out);
    out
}

/// What `file.open` takes for a file of the tree: its path and its machine ("" is this computer). None with no folder.
fn open_args(root: Option<&str>, on: Option<&str>, path: &str) -> Option<Value> {
    Some(json!({ "path": format!("{}/{path}", root?), "on": on.unwrap_or_default() }))
}

/// The entries that stay once git's ignored names (`skip`) are out; `names` are the entries' paths from the root.
fn without_ignored(listed: Vec<Entry>, names: &[String], skip: &HashSet<String>) -> Vec<Entry> {
    listed.into_iter().zip(names).filter(|(_, n)| !skip.contains(*n)).map(|(e, _)| e).collect()
}

/// Closing a folder forgets it and everything under it.
fn close_folder(open: &mut BTreeSet<String>, entries: &mut HashMap<String, Vec<Entry>>, path: &str) {
    let under = |p: &str| p == path || p.starts_with(&format!("{path}/"));
    open.retain(|p| !under(p));
    entries.retain(|p, _| !under(p));
}

// Reading the disk (background thread) ---------------------------------------------------

/// The entries of one folder (`path` is relative to `root`; "" is the root), at most one past the cap.
fn list_dir(root: &str, path: &str, on: Option<&str>) -> Result<Vec<Entry>, String> {
    let full = if path.is_empty() { root.to_owned() } else { format!("{root}/{path}") };
    let fail = |e: String| if e.trim().is_empty() { i18n::t("files.cannot_list_this_folder") } else { e };
    let Some(machine) = on else {
        let read = std::fs::read_dir(&full).map_err(|e| fail(e.to_string()))?;
        // A link to a folder is a folder; a dangling link is a file.
        return Ok(read
            .filter_map(Result::ok)
            .filter(|e| e.file_name() != ".git")
            .map(|e| Entry { name: e.file_name().to_string_lossy().into_owned(), dir: std::fs::metadata(e.path()).is_ok_and(|m| m.is_dir()) })
            .collect());
    };
    list_remote(&full, &|command, args| exec(Some(machine), command, args, "/", 30))
}

/// A folder on another machine: `sh` lists it (kind and name pairs); a Windows machine has no `sh`,
/// so PowerShell does. A real failure of `sh` is not masked by trying PowerShell.
fn list_remote(full: &str, run: &dyn Fn(&str, &[&str]) -> Result<(i32, String, String), String>) -> Result<Vec<Entry>, String> {
    let fail = |e: String| if e.trim().is_empty() { i18n::t("files.cannot_list_this_folder") } else { e };
    let script = "n=0; for p in \"$1\"/* \"$1\"/.[!.]* \"$1\"/..?*; do [ -e \"$p\" ] || [ -L \"$p\" ] || continue; name=${p##*/}; [ \"$name\" = .git ] && continue; if [ -d \"$p\" ]; then kind=d; else kind=f; fi; printf \"%s\\0%s\\0\" \"$kind\" \"$name\"; n=$((n+1)); [ \"$n\" -gt 500 ] && break; done; test -d \"$1\"";
    match run("sh", &["-c", script, "files", full]) {
        Ok((0, out, _)) => Ok(parse_listing(&out)),
        Ok((_, _, err)) => Err(fail(err)),
        Err(e) if e.contains("cannot start sh: program not found") => {
            // Curly single quotes are quotes to PowerShell too: they are doubled like '.
            let quoted: String = full.chars().flat_map(|c| if "'\u{2018}\u{2019}\u{201A}\u{201B}".contains(c) { vec![c, c] } else { vec![c] }).collect();
            let command = format!(
                "[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); & {{ param($p) @(Get-ChildItem -LiteralPath $p -Force -ErrorAction Stop | Where-Object Name -ne '.git' | Select-Object -First 501 | ForEach-Object {{ @{{ name=$_.Name; dir=($_.PSIsContainer -and (-not ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -or -not $_.LinkType -or ($_.Target -and [IO.Directory]::Exists(([IO.Path]::Combine([IO.Path]::GetDirectoryName($_.FullName), $_.Target[0])))))) }} }}) | ConvertTo-Json -Compress }} '{quoted}'"
            );
            match run("powershell", &["-NoProfile", "-Command", &command])? {
                (0, out, _) => parse_powershell(&out),
                (_, _, err) => Err(fail(err)),
            }
        }
        Err(e) => Err(e),
    }
}

/// The one `git check-ignore` that asks about all of a folder's names.
fn ignore_args(names: &[String]) -> Vec<String> {
    ["check-ignore", "-z", "--"].iter().map(|s| (*s).to_owned()).chain(names.iter().cloned()).collect()
}

/// Names git ignores among `names` (relative to `cwd`): one `git check-ignore`, nothing when it is no repo.
fn ignored(names: &[String], cwd: &str, on: Option<&str>) -> HashSet<String> {
    if names.is_empty() {
        return HashSet::new();
    }
    let args = ignore_args(names);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match exec(on, "git", &args, cwd, 30) {
        Ok((0, out, _)) => out.split('\0').filter(|s| !s.is_empty()).map(str::to_owned).collect(),
        _ => HashSet::new(),
    }
}

/// Everything one refresh reads: the open folders and git's marks.
#[derive(Default)]
struct Loaded {
    entries: HashMap<String, Vec<Entry>>,
    marks: HashMap<String, char>,
    limited: bool,
}

fn load(cwd: &str, on: Option<&str>, folders: &[String], hide_ignored: bool) -> Result<Loaded, String> {
    let mut out = Loaded::default();
    let mut used = 0;
    for path in std::iter::once(&String::new()).chain(folders) {
        let listed = match list_dir(cwd, path, on) {
            Ok(l) => l,
            // A folder that went away is dropped; the root failing is the error.
            Err(e) if path.is_empty() => return Err(e),
            Err(_) => continue,
        };
        let listed = if hide_ignored {
            let names: Vec<String> = listed.iter().map(|e| if path.is_empty() { e.name.clone() } else { format!("{path}/{}", e.name) }).collect();
            let skip = ignored(&names, cwd, on);
            without_ignored(listed, &names, &skip)
        } else {
            listed
        };
        let (kept, cut) = limit_entries(listed, used);
        used += kept.len();
        out.limited |= cut;
        out.entries.insert(path.clone(), kept);
    }
    if let Ok((0, top, _)) = exec(on, "git", &["rev-parse", "--show-toplevel"], cwd, 30) {
        let args: Vec<&str> = READ.iter().copied().chain(["status", "--porcelain=v1", "-z", "--no-renames", "--untracked-files=all"]).collect();
        if let Ok((0, status, _)) = exec(on, "git", &args, cwd, 30) {
            out.marks = parse_marks(top.trim(), &status);
        }
    }
    Ok(out)
}

// The view ---------------------------------------------------------------------------------

pub(super) struct FilesView {
    app: WeakEntity<AppWindow>,
    place: Place,
    /// The place as the window gave it (before `normalized`): the cheap check each frame.
    raw: Place,
    /// Bumped when the place changes: answers that were for another place are dropped.
    generation: u64,
    entries: HashMap<String, Vec<Entry>>,
    open: BTreeSet<String>,
    saved: HashMap<String, BTreeSet<String>>,
    marks: HashMap<String, char>,
    rows: Vec<Row>,
    error: Option<String>,
    limited: bool,
    loading: bool,
    again: bool,
    picked: Option<String>,
    shown_ignored: Option<bool>,
    watched: Option<String>,
    seen: Instant,
    loaded: Option<Instant>,
    scroll: UniformListScrollHandle,
    focus: FocusHandle,
    /// The row the keyboard is on (Up/Down move it, Enter opens it).
    cursor: usize,
}

impl FilesView {
    pub(super) fn new(app: WeakEntity<AppWindow>, cx: &mut Context<Self>) -> Self {
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_secs(10)).await;
            if this.update(cx, |this, cx| this.refresh_if_seen(cx)).is_err() {
                break;
            }
        })
        .detach();
        Self {
            app, place: Place::default(), raw: Place::default(), generation: 0, entries: HashMap::new(), open: BTreeSet::new(), saved: HashMap::new(), marks: HashMap::new(), rows: Vec::new(), error: None,
            limited: false, loading: false, again: false, picked: None, shown_ignored: None, watched: None, seen: Instant::now(), loaded: None, scroll: UniformListScrollHandle::new(), focus: cx.focus_handle().tab_stop(true), cursor: 0,
        }
    }

    fn key(place: &Place) -> String {
        format!("{:?}", (&place.cwd, &place.on, &place.space))
    }

    /// The window says where the focused pane is; a new place starts over (open folders are remembered per space).
    pub(super) fn set_place(&mut self, place: Place, cx: &mut Context<Self>) {
        if place == self.raw {
            return;
        }
        self.raw = place.clone();
        let place = place.normalized();
        if place == self.place {
            return;
        }
        self.saved.insert(Self::key(&self.place), std::mem::take(&mut self.open));
        self.open = self.saved.get(&Self::key(&place)).cloned().unwrap_or_default();
        self.place = place;
        self.generation += 1;
        self.entries.clear();
        self.marks.clear();
        self.rows.clear();
        self.error = None;
        self.limited = false;
        self.watched = None;
        self.loading = false;
        self.picked = None;
        self.refresh(cx);
        cx.notify();
    }

    fn refresh_if_seen(&mut self, cx: &mut Context<Self>) {
        if self.seen.elapsed() < Duration::from_secs(15) {
            self.refresh(cx);
        }
    }

    fn rebuild(&mut self) {
        let root = self.place.cwd.clone().unwrap_or_default();
        self.rows = flatten(&self.entries, &self.open, &self.marks, &root);
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let Some(cwd) = self.place.cwd.clone() else { return };
        if self.loading {
            self.again = true;
            return;
        }
        self.loading = true;
        let (generation, on) = (self.generation, self.place.on.clone());
        let folders: Vec<String> = self.open.iter().cloned().collect();
        let hide = mods::shared("files.ignored").is_some_and(|v| v == false);
        self.shown_ignored = Some(!hide);
        let from = cwd.clone();
        cx.spawn(async move |this, cx| {
            let loaded = cx.background_executor().spawn(async move { load(&from, on.as_deref(), &folders, hide) }).await;
            let _ = this.update(cx, |this, cx| {
                if this.generation != generation {
                    return;
                }
                this.loading = false;
                this.loaded = Some(Instant::now());
                match loaded {
                    Ok(l) => {
                        this.entries = l.entries;
                        this.marks = l.marks;
                        this.limited = l.limited;
                        this.error = None;
                        this.watch(cx);
                    }
                    Err(e) => this.error = Some(e),
                }
                this.rebuild();
                cx.notify();
                if std::mem::take(&mut this.again) {
                    this.refresh(cx);
                }
            });
        })
        .detach();
    }

    /// Re-reads when the folder changes on disk (only this computer's folders can be watched).
    fn watch(&mut self, cx: &mut Context<Self>) {
        let Some(root) = self.place.cwd.clone().filter(|_| self.place.on.is_none()) else { return };
        if self.watched.as_deref() == Some(root.as_str()) {
            return;
        }
        self.watched = Some(root.clone());
        let generation = self.generation;
        cx.spawn(async move |this, cx| {
            loop {
                if crate::files::next_change(root.clone()).await.is_err() {
                    let _ = this.update(cx, |this, _| if this.generation == generation { this.watched = None });
                    break;
                }
                // One refresh per batch, never faster than the disk answers.
                let Ok(go) = this.update(cx, |this, cx| {
                    if this.generation != generation {
                        return false;
                    }
                    this.refresh_if_seen(cx);
                    true
                }) else { break };
                if !go {
                    break;
                }
            }
        })
        .detach();
    }

    fn toggle(&mut self, path: &str, cx: &mut Context<Self>) {
        if self.open.contains(path) {
            close_folder(&mut self.open, &mut self.entries, path);
            self.rebuild();
            cx.notify();
            return;
        }
        self.open.insert(path.to_owned());
        let (Some(cwd), generation, on) = (self.place.cwd.clone(), self.generation, self.place.on.clone()) else { return };
        let hide = mods::shared("files.ignored").is_some_and(|v| v == false);
        let used: usize = self.entries.values().map(Vec::len).sum();
        let path = path.to_owned();
        cx.spawn(async move |this, cx| {
            let name = path.clone();
            let listed = cx
                .background_executor()
                .spawn(async move {
                    let listed = list_dir(&cwd, &name, on.as_deref())?;
                    Ok::<_, String>(if hide {
                        let names: Vec<String> = listed.iter().map(|e| format!("{name}/{}", e.name)).collect();
                        let skip = ignored(&names, &cwd, on.as_deref());
                        without_ignored(listed, &names, &skip)
                    } else {
                        listed
                    })
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.generation != generation || !this.open.contains(&path) {
                    return;
                }
                match listed {
                    Ok(l) => {
                        let (kept, cut) = limit_entries(l, used);
                        this.limited |= cut;
                        this.entries.insert(path, kept);
                    }
                    Err(e) => this.error = Some(e),
                }
                this.rebuild();
                cx.notify();
            });
        })
        .detach();
        self.rebuild();
        cx.notify();
    }

    /// The editor reads the file itself (here, or read only on another machine: "" is this computer).
    fn show(&self, path: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(args) = open_args(self.place.cwd.as_deref(), self.place.on.as_deref(), path) else { return };
        let _ = self.app.update(cx, |app, cx| app.run("file.open", args, window, cx));
    }

    /// Arrows move the cursor, Right/Left open and close a folder, Enter or Space opens the row.
    fn press(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if let Some(to) = step_cursor(self.cursor, key, self.rows.len()) {
            self.cursor = to;
            self.scroll.scroll_to_item(to, gpui::ScrollStrategy::Nearest);
            cx.notify();
            return true;
        }
        let Some(row) = self.rows.get(self.cursor).cloned() else { return false };
        match key {
            "right" if row.dir && !row.open => self.toggle(&row.path, cx),
            "left" if row.dir && row.open => self.toggle(&row.path, cx),
            "enter" | "space" if row.dir => self.toggle(&row.path, cx),
            "enter" | "space" => self.show(&row.path, window, cx),
            _ => return false,
        }
        true
    }

    fn row(&self, i: usize, double: bool, focused: bool, cx: &mut Context<Self>) -> AnyElement {
        let Some(row) = self.rows.get(i).cloned() else { return div().into_any_element() };
        let path = row.path.clone();
        let mark_color = match row.mark {
            Some('A') => tone("ansi2"),
            Some('D' | 'U') => tone("ansi1"),
            _ => tone("blocked"),
        };
        let picked = double && !row.dir && self.picked.as_deref() == Some(row.path.as_str());
        let (single, multi, is_dir) = (path.clone(), path.clone(), row.dir);
        let on_cursor = focused && self.cursor == i;
        ring(div().id(SharedString::from(format!("file-{path}"))), on_cursor)
            .h(z(ROW_H))
            .w_full()
            .flex_none()
            .flex()
            .items_center()
            .pl(z(8.0 + row.depth as f32 * 14.0))
            .pr(z(8.0))
            .gap(z(7.0))
            .rounded(r(7.0))
            .cursor_pointer()
            .text_size(z(13.0))
            .text_color(c("foreground"))
            .hover(|s| s.bg(c("hover")))
            .when(picked, |d| d.bg(c("selected")))
            .when(!double || row.dir, |d| {
                d.on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.cursor = i;
                    this.focus.focus(window, cx);
                    if is_dir { this.toggle(&single, cx) } else { this.show(&single, window, cx) }
                }))
            })
            .when(double && !row.dir, |d| {
                d.on_mouse_down(MouseButton::Left, cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                    this.cursor = i;
                    this.focus.focus(window, cx);
                    if e.click_count >= 2 {
                        this.show(&multi, window, cx);
                    } else {
                        this.picked = Some(multi.clone());
                        cx.notify();
                    }
                }))
            })
            .child(if row.dir { icon(if row.open { CHEVRON_DOWN } else { CHEVRON_RIGHT }, 12.0, graphic("muted")).into_any_element() } else { div().w(z(12.0)).flex_none().into_any_element() })
            .child(icon(if row.dir { FOLDER } else { FILE }, 14.0, graphic("muted")))
            .child(div().id("file-name").flex_1().min_w(px(0.0)).truncate().tooltip(tip(row.name.to_string())).child(row.name))
            .when_some(row.mark, |d, m| d.child(div().flex_none().w(z(14.0)).text_size(z(12.0)).font_weight(FontWeight::SEMIBOLD).text_color(mark_color).child(m.to_string())))
            .into_any_element()
    }
}

impl Render for FilesView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.seen = Instant::now();
        self.cursor = self.cursor.min(self.rows.len().saturating_sub(1));
        // Settings > Look > Show ignored files changed: read the folders again.
        let shown = !mods::shared("files.ignored").is_some_and(|v| v == false);
        if self.shown_ignored.is_some_and(|s| s != shown) {
            self.refresh(cx);
        }
        if self.loaded.is_some_and(|at| at.elapsed() > Duration::from_secs(10)) {
            self.refresh(cx);
        }
        let double = mods::shared("files.open").is_some_and(|v| v == "double");
        let empty = match (&self.error, &self.place.cwd) {
            (Some(e), _) => Some((e.clone(), true)),
            (None, None) => Some((i18n::t("welcome.empty_files"), false)),
            (None, Some(_)) if self.rows.is_empty() => Some((i18n::t(if self.loaded.is_none() { "files.loading_files" } else { "files.no_files" }), false)),
            _ => None,
        };
        let folder = self.place.cwd.as_deref().and_then(|p| p.rsplit(['/', '\\']).find(|s| !s.is_empty())).unwrap_or_default().to_owned();
        let count = self.rows.len();
        div()
            .id("files")
            .size_full()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .rounded(r(10.0))
            .overflow_hidden()
            .bg(c("surface"))
            .border_1()
            .border_color(c("border"))
            .text_color(c("foreground"))
            .text_size(z(13.0))
            .child(
                div()
                    .h(z(42.0))
                    .flex_none()
                    .px(z(14.0))
                    .gap(z(8.0))
                    .flex()
                    .items_center()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(i18n::t("files.files")))
                    .child(div().flex_1().min_w(px(0.0)).truncate().text_right().text_size(z(12.0)).text_color(tone("muted")).child(folder)),
            )
            .child(fade(8.0).my(z(0.0)))
            .when_some(empty, |d, (text, error)| {
                let ring = c("accent");
                let app = cx.entity();
                d.child(div().p(z(14.0)).flex().flex_col().items_start().gap(z(8.0)).text_size(z(12.0)).text_color(tone(if error { "ansi1" } else { "muted" })).child(text)
                    .when(error, |d| d.child(keyed(
                        div().id("files-retry").h(z(32.0)).px(z(10.0)).flex().items_center().rounded(r(6.0)).border_2().border_color(Hsla { a: 0.0, ..ring })
                            .text_color(c("foreground")).bg(c("raised")).cursor_pointer().hover(|s| s.opacity(0.85)),
                        ring,
                        move |_, cx| app.update(cx, |this, cx| this.refresh(cx)),
                    ).child(i18n::t("rs.setup.retry")))))
            })
            .child(
                div().id("files-tree-focus").track_focus(&self.focus).flex_1().min_h(px(0.0)).p(z(6.0))
                .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                    if this.press(e.keystroke.key.as_str(), window, cx) {
                        cx.stop_propagation();
                    }
                })).child(
                    uniform_list("files-tree", count, cx.processor(move |this, range: std::ops::Range<usize>, window, cx| {
                        let focused = this.focus.is_focused(window);
                        range.map(|i| this.row(i, double, focused, cx)).collect::<Vec<_>>()
                    }))
                        .track_scroll(&self.scroll)
                        .size_full(),
                ),
            )
            .when(self.limited, |d| d.child(div().px(z(14.0)).py(z(8.0)).text_size(z(12.0)).text_color(tone("muted")).child(i18n::t("files.file_list_limited_to_500"))))
    }
}

#[cfg(test)]
mod tests {
    use super::{BTreeSet, Entry, HashMap, HashSet, close_folder, flatten, ignore_args, limit_entries, list_dir, list_remote, mark_of, open_args, parse_listing, parse_marks, parse_powershell, sort_entries, without_ignored};
    use serde_json::json;
    use std::cell::RefCell;

    fn e(name: &str, dir: bool) -> Entry {
        Entry { name: name.into(), dir }
    }

    #[test]
    fn folders_come_first_and_names_ignore_case() {
        let mut list = vec![e("b.rs", false), e("Zed", true), e("a.rs", false), e("alpha", true), e("B.md", false)];
        sort_entries(&mut list);
        assert_eq!(list.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), ["alpha", "Zed", "a.rs", "B.md", "b.rs"]);
    }

    #[test]
    fn the_listing_script_output_is_read_in_pairs() {
        let list = parse_listing("d\0src\0f\0a b.txt\0");
        assert_eq!(list, vec![e("src", true), e("a b.txt", false)]);
        assert!(parse_listing("").is_empty());
        assert!(parse_listing("d\0").is_empty());
    }

    #[test]
    fn powershell_gives_an_object_for_one_entry_and_a_list_for_many() {
        assert_eq!(parse_powershell(r#"{"name":"x","dir":true}"#).unwrap(), vec![e("x", true)]);
        assert_eq!(parse_powershell(r#"[{"name":"x","dir":true},{"name":"y","dir":false}]"#).unwrap(), vec![e("x", true), e("y", false)]);
        assert!(parse_powershell("").unwrap().is_empty());
        assert!(parse_powershell("nope").is_err());
    }

    #[test]
    fn status_codes_become_the_four_marks() {
        assert_eq!(mark_of("??"), 'A');
        assert_eq!(mark_of("A "), 'A');
        assert_eq!(mark_of("UU"), 'U');
        assert_eq!(mark_of(" D"), 'D');
        assert_eq!(mark_of(" M"), 'M');
        assert_eq!(mark_of("MM"), 'M');
    }

    #[test]
    fn marks_are_keyed_by_the_repositorys_top() {
        let marks = parse_marks("/r", " M src/a.rs\0?? new.md\0");
        assert_eq!(marks.get("/r/src/a.rs"), Some(&'M'));
        assert_eq!(marks.get("/r/new.md"), Some(&'A'));
    }

    #[test]
    fn a_folder_is_cut_at_500_and_the_whole_tree_at_5000() {
        let many: Vec<Entry> = (0..600).map(|i| e(&format!("f{i:03}"), false)).collect();
        let (kept, cut) = limit_entries(many, 0);
        assert_eq!((kept.len(), cut), (500, true));
        let (kept, cut) = limit_entries(vec![e("a", false), e("b", false), e("c", false)], 4999);
        assert_eq!((kept.len(), cut), (1, true));
        let (kept, cut) = limit_entries(vec![e("a", false)], 0);
        assert_eq!((kept.len(), cut), (1, false));
    }

    #[test]
    fn open_folders_show_their_children_indented_with_their_marks() {
        let mut entries = HashMap::new();
        entries.insert(String::new(), vec![e("src", true), e("README.md", false)]);
        entries.insert("src".to_owned(), vec![e("main.rs", false)]);
        let marks = HashMap::from([("/r/src/main.rs".to_owned(), 'M')]);
        let closed = flatten(&entries, &BTreeSet::new(), &marks, "/r");
        assert_eq!(closed.len(), 2);
        let open = flatten(&entries, &BTreeSet::from(["src".to_owned()]), &marks, "/r");
        assert_eq!(open.iter().map(|r| (r.path.as_str(), r.depth, r.mark)).collect::<Vec<_>>(), [("src", 0, None), ("src/main.rs", 1, Some('M')), ("README.md", 0, None)]);
    }

    #[test]
    fn closing_a_folder_forgets_what_is_under_it_but_not_a_namesake() {
        let mut open = BTreeSet::from(["a".to_owned(), "a/b".to_owned(), "ab".to_owned()]);
        let mut entries = HashMap::from([("a".to_owned(), vec![]), ("a/b".to_owned(), vec![]), ("ab".to_owned(), vec![])]);
        close_folder(&mut open, &mut entries, "a");
        assert_eq!(open.into_iter().collect::<Vec<_>>(), ["ab"]);
        assert_eq!(entries.keys().cloned().collect::<Vec<_>>(), ["ab"]);
    }

    #[test]
    fn listing_a_real_folder_skips_git_and_follows_links_to_folders() {
        let dir = std::env::temp_dir().join(format!("sunna-files-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.txt"), "x").unwrap();
        let mut list = list_dir(dir.to_str().unwrap(), "", None).unwrap();
        sort_entries(&mut list);
        assert_eq!(list, vec![e("sub", true), e("a.txt", false)]);
        assert!(list_dir(dir.to_str().unwrap(), "missing", None).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// What a stub machine answered, and what it was asked.
    type Asked = RefCell<Vec<(String, Vec<String>)>>;

    fn ask<'a>(asked: &'a Asked, reply: impl Fn(&str) -> Result<(i32, String, String), String> + 'a) -> impl Fn(&str, &[&str]) -> Result<(i32, String, String), String> + 'a {
        move |command, args| {
            asked.borrow_mut().push((command.to_owned(), args.iter().map(|a| (*a).to_owned()).collect()));
            reply(command)
        }
    }

    #[test]
    fn a_remote_windows_folder_is_listed_with_powershell_on_that_machine() {
        let asked = Asked::default();
        let run = ask(&asked, |c| match c {
            "sh" => Err("cannot start sh: program not found".into()),
            _ => Ok((0, r#"[{"name":"it's","dir":true},{"name":"a.txt","dir":false}]"#.into(), String::new())),
        });
        let list = list_remote("C:/it's here", &run).unwrap();
        assert_eq!(list, vec![e("it's", true), e("a.txt", false)]);
        let asked = asked.borrow();
        assert_eq!(asked.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>(), ["sh", "powershell"]);
        // The quote in the path is doubled for PowerShell.
        assert!(asked[1].1[2].ends_with("'C:/it''s here'"));
    }

    #[test]
    fn a_remote_failure_is_not_masked_by_powershell() {
        let asked = Asked::default();
        let run = ask(&asked, |_| Ok((1, String::new(), "permission denied".into())));
        assert_eq!(list_remote("/x", &run), Err("permission denied".into()));
        assert_eq!(asked.borrow().len(), 1);
        let asked = Asked::default();
        let run = ask(&asked, |_| Err("the machine went away".into()));
        assert_eq!(list_remote("/x", &run), Err("the machine went away".into()));
        assert_eq!(asked.borrow().len(), 1);
    }

    #[test]
    fn a_remote_posix_folder_stays_on_sh_and_keeps_unicode_and_literal_names() {
        let asked = Asked::default();
        let listing = "d\0carpeta ñ\0f\0$(rm -rf).txt\0f\0*\0";
        let run = ask(&asked, |_| Ok((0, listing.into(), String::new())));
        let list = list_remote("/home/a b", &run).unwrap();
        assert_eq!(list, vec![e("carpeta ñ", true), e("$(rm -rf).txt", false), e("*", false)]);
        let asked = asked.borrow();
        assert_eq!(asked.len(), 1);
        // The folder is an argument of the script, never part of it.
        assert_eq!(asked[0].1[2..], ["files".to_owned(), "/home/a b".to_owned()]);
    }

    #[test]
    fn local_names_are_literal_and_unicode_survives() {
        let dir = std::env::temp_dir().join(format!("sunna-files-names-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("año $HOME")).unwrap();
        std::fs::write(dir.join("año $HOME").join("[x]*.txt"), "x").unwrap();
        let top = list_dir(dir.to_str().unwrap(), "", None).unwrap();
        assert_eq!(top, vec![e("año $HOME", true)]);
        let inner = list_dir(dir.to_str().unwrap(), "año $HOME", None).unwrap();
        assert_eq!(inner, vec![e("[x]*.txt", false)]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_to_a_folder_is_a_folder_and_a_dangling_one_is_a_file() {
        let dir = std::env::temp_dir().join(format!("sunna-files-links-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("real")).unwrap();
        std::os::unix::fs::symlink(dir.join("real"), dir.join("alias")).unwrap();
        std::os::unix::fs::symlink(dir.join("nowhere"), dir.join("dangling")).unwrap();
        let mut list = list_dir(dir.to_str().unwrap(), "", None).unwrap();
        sort_entries(&mut list);
        assert_eq!(list, vec![e("alias", true), e("real", true), e("dangling", false)]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ignored_names_are_asked_once_and_left_out_unless_shown() {
        let names: Vec<String> = ["src", "a.ts", "node_modules", "debug.log"].map(String::from).into();
        assert_eq!(ignore_args(&names), ["check-ignore", "-z", "--", "src", "a.ts", "node_modules", "debug.log"]);
        let listed = vec![e("src", true), e("a.ts", false), e("node_modules", true), e("debug.log", false)];
        let skip: HashSet<String> = ["node_modules", "debug.log"].map(String::from).into();
        assert_eq!(without_ignored(listed, &names, &skip), vec![e("src", true), e("a.ts", false)]);
    }

    #[test]
    fn a_file_opens_with_its_path_and_machine() {
        assert_eq!(open_args(Some("/r"), None, "src/a.ts"), Some(json!({ "path": "/r/src/a.ts", "on": "" })));
        assert_eq!(open_args(Some("/r"), Some("vm"), "b.md"), Some(json!({ "path": "/r/b.md", "on": "vm" })));
        assert_eq!(open_args(None, None, "c.md"), None);
    }
}
