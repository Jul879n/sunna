//! Worktrees, native: the dialog that makes a new worktree (a branch checked out in its own folder, opened as a
//! space), opens one of the repo's checkouts, removes one, or cleans up the merged ones. It replaces the
//! `worktree` mod: a mod that still opens an overlay named `worktree` (props `{ space, action, folder,
//! machine, branch, tab, argv }`) gets this dialog, unless the person keeps a mod folder of that name.
//!
//! Everything git runs through `files::exec`, here or on the space's machine, off the UI thread. The rules
//! (folder names, branch names, the checkout list, which space shows a checkout) are plain functions at the
//! top, tested at the bottom. Settings > Git and General are read from `shared()` (`worktree.template`,
//! `worktree.prefix_mode`, `worktree.prefix`, `worktree.nest`, `worktree.fresh`, `worktree.external`) and
//! `settings.json` (`worktrees.directory`).

use super::super::dialog::{self, Act, Dialog};
use super::super::files::exec;
use super::super::native::{icon, r, tone, z};
use super::*;
use fancy_regex::Regex;
use gpui_base::input::{Input, InputEvent, InputState};

const X: &[u8] = include_bytes!("../../../defaults/mods/workspace-chrome/x.svg");
const CHECK: &[u8] = include_bytes!("../../../defaults/mods/workspace-chrome/check.svg");
/// Rows of the open list shown at once.
const SHOWN: usize = 7;

// Rules ------------------------------------------------------------------------------

/// Hangul syllables written as separate jamo (NFD) joined back (NFC), so a typed name has one folder.
fn compose_hangul(text: &str) -> String {
    let mut out: Vec<char> = Vec::new();
    for ch in text.chars() {
        let c = ch as u32;
        if let Some(&last) = out.last() {
            let l = last as u32;
            // Leading + vowel -> a syllable.
            if (0x1100..0x1113).contains(&l) && (0x1161..0x1176).contains(&c) {
                let s = 0xAC00 + ((l - 0x1100) * 21 + (c - 0x1161)) * 28;
                *out.last_mut().expect("last") = char::from_u32(s).expect("hangul");
                continue;
            }
            // Syllable without a final + a final.
            if (0xAC00..0xD7A4).contains(&l) && (l - 0xAC00).is_multiple_of(28) && (0x11A8..0x11C3).contains(&c) {
                *out.last_mut().expect("last") = char::from_u32(l + (c - 0x11A7)).expect("hangul");
                continue;
            }
        }
        out.push(ch);
    }
    out.into_iter().collect()
}

/// The folder a branch gets: its name with anything but letters, marks, digits, `.`, `_` and `-` as `-`.
/// A name with none of those gets `branch-<hash>` so symbol-only branches do not share a folder.
pub(super) fn slug(branch: &str) -> String {
    static KEEP: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let normalized = compose_hangul(branch);
    let re = KEEP.get_or_init(|| Regex::new(r"[^\p{L}\p{M}\p{N}._-]+").expect("slug pattern"));
    let replaced = re.replace_all(&normalized, "-");
    let name = replaced.trim_matches('-');
    if !name.is_empty() {
        return name.to_owned();
    }
    let mut hash: u32 = 2_166_136_261;
    for ch in normalized.chars() {
        hash = (hash ^ ch as u32).wrapping_mul(16_777_619);
    }
    format!("branch-{hash:08x}")
}

/// `path` with the home folder as `~` (git says `C:/Users/x` where Windows says `C:\Users\x`).
pub(super) fn tilde(path: &str, home: &str) -> String {
    let (p, h) = (path.replace('\\', "/"), home.replace('\\', "/"));
    if !h.is_empty() && p.starts_with(&h) { format!("~{}", &p[h.len()..]) } else { path.to_owned() }
}

/// The folder worktrees go in, from `settings.json`'s `worktrees.directory` (default `~/.herdr/worktrees`);
/// paths belong to the machine of the space, so `~` is that machine's home. Err is a locale key.
pub(super) fn worktrees_root(settings: Option<&str>, home: &str) -> Result<String, &'static str> {
    let directory = settings.and_then(|raw| serde_json::from_str::<Value>(raw).ok()).and_then(|v| v.get("worktrees")?.get("directory").cloned());
    let directory = match directory {
        None | Some(Value::Null) => "~/.herdr/worktrees".to_owned(),
        Some(Value::String(s)) if !s.trim().is_empty() => s,
        Some(_) => return Err("worktree.worktrees_directory_must_be_a_non_empty"),
    };
    let mut path = directory.replace('\\', "/");
    if path == "~" || path.starts_with("~/") {
        path = format!("{}{}", home.replace('\\', "/").trim_end_matches('/'), &path[1..]);
    }
    let drive = path.len() >= 3 && path.as_bytes()[0].is_ascii_alphabetic() && &path[1..3] == ":/";
    if !path.starts_with('/') && !drive {
        return Err("worktree.worktrees_directory_must_be_an_absolute");
    }
    Ok(path.trim_end_matches('/').to_owned())
}

/// Settings > Git: how a typed name becomes a branch.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Rules {
    /// `{name}`, `{date}`, `{repo}`; empty means `{name}`.
    pub template: String,
    /// `none`, `git` (the person's git user name) or `custom`.
    pub mode: Option<String>,
    pub prefix: String,
}

/// The branch a typed name becomes: the template, then the prefix. A name that is already a branch is used as typed.
pub(super) fn branch_name(typed: &str, rules: &Rules, repo: &str, date: &str, git_user: impl FnOnce() -> Option<String>, exists: impl FnOnce() -> bool) -> String {
    let template = if rules.template.trim().is_empty() { "{name}" } else { rules.template.trim() };
    let mode = rules.mode.as_deref().unwrap_or("none");
    if template == "{name}" && mode == "none" {
        return typed.to_owned();
    }
    if exists() {
        return typed.to_owned();
    }
    let filled = template.replace("{date}", date).replace("{repo}", repo);
    let name = if filled.contains("{name}") { filled.replace("{name}", typed) } else { format!("{filled}-{typed}") };
    let mut prefix = match mode {
        "custom" => rules.prefix.trim().to_owned(),
        "git" => {
            static NOT: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
            let re = NOT.get_or_init(|| Regex::new(r"[^\p{L}\p{N}._-]+").expect("prefix pattern"));
            git_user().map(|u| re.replace_all(&u.to_lowercase(), "-").trim_matches('-').to_owned()).unwrap_or_default()
        }
        _ => String::new(),
    };
    if !prefix.is_empty() && !prefix.ends_with(['/', '_', '-']) {
        prefix.push('/');
    }
    if !prefix.is_empty() && !name.starts_with(&prefix) { format!("{prefix}{name}") } else { name }
}

/// One checkout of a repo.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Checkout {
    pub path: String,
    pub branch: Option<String>,
    pub main: bool,
    /// Unix time of the branch's last commit (0: none).
    pub at: i64,
}

/// Every checkout from `git worktree list --porcelain` (`list`) and `for-each-ref` lines `branch\tunix` (`refs`):
/// the main one first, then the most recently committed.
pub(super) fn parse_checkouts(list: &str, refs: &str) -> Vec<Checkout> {
    let when: HashMap<&str, i64> = refs.lines().filter_map(|l| l.split_once('\t')).map(|(b, at)| (b, at.trim().parse().unwrap_or(0))).collect();
    let mut out: Vec<Checkout> = Vec::new();
    for block in list.split("\n\n") {
        let lines: Vec<&str> = block.lines().collect();
        let Some(path) = lines.iter().find_map(|l| l.strip_prefix("worktree ")) else { continue };
        if lines.iter().any(|l| *l == "bare" || l.starts_with("prunable")) {
            continue;
        }
        let branch = lines.iter().find_map(|l| l.strip_prefix("branch ")).map(|b| b.strip_prefix("refs/heads/").unwrap_or(b).to_owned());
        let at = branch.as_deref().and_then(|b| when.get(b).copied()).unwrap_or(0);
        out.push(Checkout { path: path.to_owned(), branch, main: out.is_empty(), at });
    }
    out.sort_by(|a, b| b.main.cmp(&a.main).then(b.at.cmp(&a.at)));
    out
}

/// How long ago, short: `5m`, `3h`, `12d`, `4mo`.
pub(super) fn ago(seconds: i64) -> String {
    let d = seconds.max(0);
    if d < 3600 {
        i18n::tf("worktree.age_minutes", &[("n", &(d as f64 / 60.0).round().max(1.0).to_string())])
    } else if d < 86_400 {
        format!("{}h", (d as f64 / 3600.0).round())
    } else if d < 86_400 * 60 {
        format!("{}d", (d as f64 / 86_400.0).round())
    } else {
        i18n::tf("worktree.age_months", &[("n", &(d as f64 / 86_400.0 / 30.0).round().to_string())])
    }
}

fn inside(path: &str, dir: &str) -> bool {
    path == dir || path.strip_prefix(dir).is_some_and(|rest| rest.starts_with('/'))
}

/// The space showing a checkout: its first terminal is in it (the deepest checkout wins).
pub(super) fn space_of(path: &str, all: &[Checkout], on: &Option<String>, spaces: &[Value], panes: &[Value]) -> Option<String> {
    spaces
        .iter()
        .find(|s| {
            if opt_text(s, "machine").map(str::to_owned) != *on {
                return false;
            }
            let at = place(s, panes).replace('\\', "/");
            !at.is_empty() && inside(&at, path) && !all.iter().any(|c| c.path != path && c.path.len() > path.len() && inside(&at, &c.path))
        })
        .map(|s| text(s, "id").to_owned())
}

/// The branches `git branch --merged` lists.
pub(super) fn merged_branches(out: &str) -> HashSet<String> {
    out.lines().map(|l| l.trim_start_matches(['*', '+']).trim().to_owned()).filter(|l| !l.is_empty()).collect()
}

// Git --------------------------------------------------------------------------------

struct Git {
    ok: bool,
    out: String,
    err: String,
}

fn git(cwd: &str, args: &[&str], on: &Option<String>, secs: u64) -> Git {
    match exec(on.as_deref(), "git", args, cwd, secs) {
        Ok((code, out, err)) => Git { ok: code == 0, out: out.trim().to_owned(), err: if err.trim().is_empty() { out.trim().to_owned() } else { err.trim().to_owned() } },
        Err(e) => Git { ok: false, out: String::new(), err: e },
    }
}

const STATUS: [&str; 5] = ["--no-optional-locks", "-c", "core.fsmonitor=false", "status", "--porcelain"];

#[derive(Clone, Debug, PartialEq)]
struct Repo {
    top: String,
    main: String,
    name: String,
    linked: bool,
}

/// The repo a folder belongs to: its main checkout and name, and whether the folder is itself a linked worktree.
fn repo_of(folder: &str, on: &Option<String>) -> Option<Repo> {
    let top = git(folder, &["rev-parse", "--show-toplevel"], on, 30);
    let common = git(folder, &["rev-parse", "--path-format=absolute", "--git-common-dir"], on, 30);
    if !top.ok || !common.ok {
        return None;
    }
    let main = common.out.strip_suffix("/.git").unwrap_or(&common.out).to_owned();
    Some(Repo { name: main.rsplit('/').next().unwrap_or_default().to_owned(), linked: main != top.out, top: top.out, main })
}

fn checkouts(repo: &Repo, on: &Option<String>) -> Vec<Checkout> {
    let list = git(&repo.main, &["worktree", "list", "--porcelain"], on, 30);
    if !list.ok {
        return Vec::new();
    }
    let refs = git(&repo.main, &["for-each-ref", "--format=%(refname:short)%09%(committerdate:unix)", "refs/heads"], on, 30);
    parse_checkouts(&list.out, &refs.out)
}

/// The submodule repos git keeps for a checkout (nested ones too, checked out or not): name, git dir, pinned commit.
fn modules(git_dir: &str, config: &[&str], on: &Option<String>, out: &mut Vec<(String, String, String)>, parent: &str) {
    let mut args = vec!["--git-dir", git_dir, "config"];
    args.extend_from_slice(config);
    args.extend(["-z", "--get-regexp", "^submodule\\..*\\.path$"]);
    let keys = git(git_dir, &args, on, 30);
    if !keys.ok {
        return;
    }
    // -z: "key\nvalue" records ended by NUL, so names and paths may hold spaces.
    for record in keys.out.split('\0').filter(|r| !r.is_empty()) {
        let Some((key, value)) = record.split_once('\n') else { continue };
        let Some(name) = key.strip_prefix("submodule.").and_then(|k| k.strip_suffix(".path")) else { continue };
        let dir = format!("{git_dir}/modules/{name}");
        if !git(git_dir, &["--git-dir", &dir, "rev-parse", "--git-dir"], on, 30).ok {
            continue;
        }
        let pinned = git(git_dir, &["--git-dir", git_dir, "rev-parse", &format!("HEAD:{value}")], on, 30);
        out.push((format!("{parent}{name}"), dir.clone(), if pinned.ok { pinned.out } else { String::new() }));
        if git(git_dir, &["--git-dir", &dir, "cat-file", "-e", "HEAD:.gitmodules"], on, 30).ok {
            modules(&dir, &["--blob", "HEAD:.gitmodules"], on, out, &format!("{parent}{name}/"));
        }
    }
}

/// The checkout's submodules, and the names of those with commits that exist nowhere else (not on a remote, and
/// not the commit the parent pins).
fn submodules(top: &str, on: &Option<String>) -> Result<(usize, Vec<String>), String> {
    let dir = git(top, &["rev-parse", "--absolute-git-dir"], on, 30);
    if !dir.ok {
        return Err(if dir.err.is_empty() { i18n::t("worktree.git_rev_parse_failed") } else { dir.err });
    }
    let mut mods = Vec::new();
    let file = format!("{top}/.gitmodules");
    modules(&dir.out, &["--file", &file], on, &mut mods, "");
    let mut unpushed = Vec::new();
    for (name, module_dir, pinned) in &mods {
        let mut args = vec!["--git-dir", module_dir.as_str(), "rev-list", "-n1", "--all", "--not", "--remotes"];
        if !pinned.is_empty() {
            args.push(pinned);
        }
        let local = git(&dir.out, &args, on, 30);
        if !local.ok {
            return Err(if local.err.is_empty() { i18n::tf("worktree.couldn_t_check_submodule_name", &[("name", name)]) } else { local.err });
        }
        if !local.out.is_empty() {
            unpushed.push(name.clone());
        }
    }
    Ok((mods.len(), unpushed))
}

/// Settings > Git and General, read when the dialog opens.
#[derive(Clone, Debug, Default)]
struct Prefs {
    rules: Rules,
    /// A folder per project under the worktrees folder (default on).
    nest: bool,
    /// New branches start from the remote's default branch, brought up to date.
    fresh: bool,
    /// The open list shows worktrees from outside the worktrees folder (default on).
    external: bool,
}

fn prefs() -> Prefs {
    let get = |k: &str| mods::shared(&format!("worktree.{k}"));
    let string = |k: &str| get(k).and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default();
    Prefs {
        rules: Rules { template: string("template"), mode: get("prefix_mode").and_then(|v| v.as_str().map(str::to_owned)), prefix: string("prefix") },
        nest: get("nest").and_then(|v| v.as_bool()) != Some(false),
        fresh: get("fresh").and_then(|v| v.as_bool()) == Some(true),
        external: get("external").and_then(|v| v.as_bool()) != Some(false),
    }
}

/// Where a new branch starts: the remote's default branch, brought up to date, else HEAD.
fn fresh_start(repo: &Repo, on: &Option<String>) -> String {
    let head = git(&repo.main, &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"], on, 30);
    let mut default = if head.ok { head.out.trim_start_matches("origin/").to_owned() } else { String::new() };
    if default.is_empty() {
        for name in ["main", "master"] {
            if git(&repo.main, &["rev-parse", "--verify", "--quiet", &format!("refs/remotes/origin/{name}")], on, 30).ok {
                default = name.to_owned();
                break;
            }
        }
    }
    if default.is_empty() || !git(&repo.main, &["fetch", "origin", &default], on, 120).ok {
        return "HEAD".to_owned();
    }
    // Refused while that branch is checked out: fine.
    git(&repo.main, &["fetch", "origin", &format!("{default}:{default}")], on, 120);
    format!("origin/{default}")
}

/// Makes the worktree for `typed`; the folder it is in, or why not (already shown text).
fn create(typed: &str, repo: &Repo, root: &str, prefs: &Prefs, on: &Option<String>) -> Result<String, String> {
    let typed = typed.trim();
    if typed.is_empty() {
        return Err(i18n::t("worktree.type_a_branch_name"));
    }
    let (y, m, d, ..) = settings_ui::timefmt::local(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |t| t.as_secs() as i64));
    let date = format!("{y}-{m:02}-{d:02}");
    let branch = branch_name(
        typed,
        &prefs.rules,
        &repo.name,
        &date,
        || Some(git(&repo.top, &["config", "user.name"], on, 30)).filter(|g| g.ok).map(|g| g.out),
        || git(&repo.main, &["show-ref", "--verify", "--quiet", &format!("refs/heads/{typed}")], on, 30).ok,
    );
    let path = format!("{root}/{}{}", if prefs.nest { format!("{}/", repo.name) } else { String::new() }, slug(&branch));
    let exists = git(&repo.main, &["show-ref", "--verify", "--quiet", &format!("refs/heads/{branch}")], on, 30).ok;
    let start = if !exists && prefs.fresh { fresh_start(repo, on) } else { "HEAD".to_owned() };
    let args: Vec<&str> = if exists { vec!["worktree", "add", &path, &branch] } else { vec!["worktree", "add", "-b", &branch, &path, &start] };
    let added = git(&repo.top, &args, on, 600);
    if added.ok { Ok(path) } else { Err(if added.err.is_empty() { i18n::t("worktree.git_worktree_add_failed") } else { added.err }) }
}

// The dialog -------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Mode {
    New,
    Open,
    Remove,
    Cleanup,
}

impl Mode {
    fn parse(action: Option<&str>) -> Self {
        match action {
            Some("open") => Mode::Open,
            Some("remove") => Mode::Remove,
            Some("cleanup") => Mode::Cleanup,
            _ => Mode::New,
        }
    }
}

/// A checkout with the evidence for removing it.
#[derive(Clone, Debug)]
struct Cand {
    c: Checkout,
    dirty: bool,
    busy: bool,
    merged: bool,
    space: Option<String>,
}

impl Cand {
    fn locked(&self) -> bool {
        self.dirty || self.busy
    }
}

/// What the dialog was opened with (the overlay's props).
#[derive(Clone, Debug, Default)]
struct Props {
    space: Option<String>,
    folder: Option<String>,
    machine: Option<String>,
    name: Option<String>,
    branch: Option<String>,
    tab: Option<String>,
    argv: Option<Value>,
    /// Opened from a space's menu: «Create from this space» starts on.
    parent: bool,
}

impl Props {
    fn from(v: &Value) -> Self {
        let s = |k: &str| v.get(k).and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_owned);
        Self { space: s("space"), folder: s("folder"), machine: s("machine"), name: s("name"), branch: s("branch"), tab: s("tab"), argv: v.get("argv").cloned().filter(|a| !a.is_null()), parent: v.get("parent").and_then(Value::as_bool) == Some(true) }
    }
}

/// What reading the repo found.
struct Loaded {
    user_home: String,
    root: Result<String, String>,
    folder: String,
    repo: Option<Repo>,
    dirty: bool,
    unpushed: Vec<String>,
    list: Option<Vec<Checkout>>,
    cands: Option<Vec<Cand>>,
}

struct Snapshot {
    spaces: Vec<Value>,
    panes: Vec<Value>,
    agents: Vec<Value>,
}

/// Everything the dialog reads about the repo, off the UI thread.
fn load(mode: Mode, props: &Props, space: Option<Value>, snap: &Snapshot, prefs: &Prefs) -> Result<Loaded, String> {
    let on = props.machine.clone().or_else(|| space.as_ref().and_then(|s| opt_text(s, "machine").map(str::to_owned))).filter(|m| !m.is_empty());
    let user_home = match &on {
        None => mods::home().to_string_lossy().into_owned(),
        Some(machine) => {
            let ask = |program: &str, args: &[&str]| exec(Some(machine), program, args, "", 30).ok().filter(|(c, o, _)| *c == 0 && !o.trim().is_empty());
            let found = ask("sh", &["-c", "printf %s \"$HOME\""]).or_else(|| ask("powershell.exe", &["-NoProfile", "-Command", "[Console]::Write($HOME)"]));
            found.ok_or_else(|| i18n::t("worktree.could_not_find_the_remote"))?.1.trim().replace('\\', "/")
        }
    };
    let root = if matches!(mode, Mode::New | Mode::Open) {
        let raw = std::fs::read_to_string(mods::app_dir().join("settings.json")).ok();
        worktrees_root(raw.as_deref(), &user_home).map_err(i18n::t)
    } else {
        Err(String::new())
    };
    if let (Err(e), Mode::New) = (&root, mode) {
        return Err(e.clone());
    }
    let folder = props.folder.clone().or_else(|| space.as_ref().map(|s| place(s, &snap.panes)).filter(|p| !p.is_empty())).unwrap_or_else(|| user_home.clone());
    let repo = repo_of(&folder, &on);
    let (mut dirty, mut unpushed) = (false, Vec::new());
    if let (Some(repo), Mode::Remove) = (&repo, mode) {
        dirty = !git(&repo.top, &STATUS, &on, 30).out.is_empty();
        unpushed = submodules(&repo.top, &on)?.1;
    }
    let mut list = None;
    if let (Some(repo), Mode::Open | Mode::Cleanup) = (&repo, mode) {
        let all = checkouts(repo, &on);
        let hide = mode == Mode::Open && !prefs.external && root.is_ok();
        list = Some(if hide { all.into_iter().filter(|c| c.main || root.as_ref().is_ok_and(|r| c.path.replace('\\', "/").starts_with(&format!("{r}/")))).collect() } else { all });
    }
    let mut cands = None;
    if let (Some(repo), Some(list), Mode::Cleanup) = (&repo, &list, mode) {
        let merged = merged_branches(&git(&repo.main, &["branch", "--merged"], &on, 30).out);
        let mut out: Vec<Cand> = list
            .iter()
            .filter(|c| !c.main)
            .map(|c| {
                let dirty = !git(&c.path, &STATUS, &on, 30).out.is_empty();
                let there = space_of(&c.path, list, &on, &snap.spaces, &snap.panes);
                let busy = there.as_ref().is_some_and(|id| snap.agents.iter().any(|a| text(a, "space") == id && text(a, "status") == "working"));
                Cand { dirty, busy, merged: c.branch.as_ref().is_some_and(|b| merged.contains(b)), space: there, c: c.clone() }
            })
            .collect();
        out.sort_by(|a, b| b.merged.cmp(&a.merged).then(b.c.at.cmp(&a.c.at)));
        cands = Some(out);
    }
    Ok(Loaded { user_home, root, folder, repo, dirty, unpushed, list, cands })
}

pub(super) struct WorktreeView {
    app: WeakEntity<AppWindow>,
    id: String,
    mode: Mode,
    props: Props,
    prefs: Prefs,
    on: Option<String>,
    /// None while reading; Some(None) outside git.
    repo: Option<Option<Repo>>,
    user_home: String,
    root: Option<String>,
    folder: String,
    dirty: bool,
    unpushed: Vec<String>,
    list: Option<Vec<Checkout>>,
    cands: Option<Vec<Cand>>,
    sel: HashSet<String>,
    armed: bool,
    at: usize,
    busy: bool,
    error: String,
    /// «Create from this space»: the new space becomes this one's child (`space.parent`).
    from_here: bool,
    input: Entity<InputState>,
    focus: dialog::Focus,
}

impl WorktreeView {
    fn new(app: WeakEntity<AppWindow>, id: String, mode: Mode, props: Props, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let opening = mode == Mode::Open;
        let placeholder = i18n::t(if opening { "worktree.filter_worktrees" } else { "worktree.branch_name" });
        let input = dialog::input(props.branch.as_deref().unwrap_or(""), &placeholder, window, cx, move |this, ev, window, cx| match ev {
            InputEvent::PressEnter { .. } => this.go(window, cx),
            InputEvent::Change => {
                this.error.clear();
                this.at = 0;
                cx.notify();
            }
            _ => {}
        });
        let has_input = matches!(mode, Mode::New | Mode::Open);
        if has_input {
            input.update(cx, |s, cx| s.focus(window, cx));
        }
        let space = props.space.as_deref().and_then(|id| data().spaces.into_iter().find(|s| text(s, "id") == id));
        let on = props.machine.clone().or_else(|| space.as_ref().and_then(|s| opt_text(s, "machine").map(str::to_owned))).filter(|m| !m.is_empty());
        let prefs = prefs();
        let this = Self {
            app, id, mode, props: props.clone(), prefs: prefs.clone(), on, repo: None, user_home: String::new(), root: None, folder: String::new(), dirty: false, unpushed: Vec::new(),
            list: None, cands: None, sel: HashSet::new(), armed: false, at: 0, busy: false, error: String::new(), from_here: props.parent && props.space.is_some(), input, focus: dialog::Focus::new(cx),
        };
        let d = data();
        let snap = Snapshot { spaces: d.spaces, panes: d.panes, agents: d.agents };
        cx.spawn_in(window, async move |this, cx| {
            let loaded = cx.background_executor().spawn(async move { load(mode, &props, space, &snap, &prefs) }).await;
            let _ = this.update_in(cx, |this, window, cx| {
                match loaded {
                    Ok(l) => {
                        this.user_home = l.user_home;
                        this.root = l.root.ok();
                        this.folder = l.folder;
                        this.dirty = l.dirty;
                        this.unpushed = l.unpushed;
                        this.list = l.list;
                        if let Some(c) = &l.cands {
                            this.sel = c.iter().filter(|x| x.merged && !x.locked()).map(|x| x.c.path.clone()).collect();
                        }
                        this.cands = l.cands;
                        this.repo = Some(l.repo);
                        // The branch came with the request: make it without asking.
                        if this.mode == Mode::New && this.props.branch.is_some() {
                            this.work(window, cx);
                        }
                    }
                    Err(e) => {
                        this.error = e;
                        this.repo = Some(None);
                    }
                }
                cx.notify();
            });
        })
        .detach();
        this
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.id.clone();
        let _ = self.app.update(cx, |app, cx| app.close_overlay(&id, window, cx));
    }

    fn repo(&self) -> Option<&Repo> {
        self.repo.as_ref().and_then(Option::as_ref)
    }

    fn typed(&self, cx: &App) -> String {
        self.input.read(cx).value().to_string()
    }

    /// Enter: Open runs the highlighted row; the others do their one thing.
    fn go(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy || self.repo().is_none() {
            return;
        }
        match self.mode {
            Mode::Open => {
                let shown = self.shown(cx);
                if let Some(c) = shown.get(self.at).cloned() {
                    self.open(&c, window, cx);
                }
            }
            Mode::Cleanup => {
                if self.sel.is_empty() {
                    return;
                }
                if !self.armed {
                    self.armed = true;
                    cx.notify();
                    return;
                }
                self.work(window, cx);
            }
            _ => self.work(window, cx),
        }
    }

    fn shown(&self, cx: &App) -> Vec<Checkout> {
        let f = self.typed(cx).trim().to_lowercase();
        self.list.iter().flatten().filter(|c| f.is_empty() || format!("{} {}", c.branch.clone().unwrap_or_default(), c.path).to_lowercase().contains(&f)).cloned().collect()
    }

    fn open(&mut self, c: &Checkout, window: &mut Window, cx: &mut Context<Self>) {
        let d = data();
        let there = space_of(&c.path, self.list.as_deref().unwrap_or_default(), &self.on, &d.spaces, &d.panes);
        let (path, on) = (c.path.clone(), self.on.clone().unwrap_or_default());
        self.close(window, cx);
        let _ = self.app.update(cx, |app, cx| match there {
            Some(id) => app.run("space.select", json!({ "id": id }), window, cx),
            None => app.run("space.new", json!({ "folder": path, "machine": on }), window, cx),
        });
    }

    /// A checkout's own remove dialog, from its row: the same checks as from the space's menu.
    fn remove_row(&mut self, c: &Checkout, window: &mut Window, cx: &mut Context<Self>) {
        let d = data();
        let there = space_of(&c.path, self.list.as_deref().unwrap_or_default(), &self.on, &d.spaces, &d.panes);
        let props = json!({ "action": "remove", "space": there, "folder": c.path, "machine": self.on, "name": c.branch.clone().unwrap_or_else(|| c.path.rsplit('/').next().unwrap_or_default().to_owned()) });
        self.close(window, cx);
        let _ = self.app.update(cx, |app, cx| app.open_worktree(props, window, cx));
    }

    fn new_from_list(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let props = json!({ "action": "new", "space": self.props.space, "folder": self.folder, "machine": self.on });
        self.close(window, cx);
        let _ = self.app.update(cx, |app, cx| app.open_worktree(props, window, cx));
    }

    fn toggle(&mut self, path: &str, cx: &mut Context<Self>) {
        if !self.sel.remove(path) {
            self.sel.insert(path.to_owned());
        }
        self.armed = false;
        cx.notify();
    }

    /// Does the dialog's work (new, remove or cleanup) off the UI thread, then closes or shows what went wrong.
    fn work(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(repo) = self.repo().cloned() else { return };
        if self.busy {
            return;
        }
        self.busy = true;
        self.error.clear();
        cx.notify();
        let (mode, on, prefs, root, typed) = (self.mode, self.on.clone(), self.prefs.clone(), self.root.clone().unwrap_or_default(), self.typed(cx));
        let space_id = self.props.space.clone();
        let parent = if self.from_here { self.props.space.clone() } else { None };
        let (dirty, unpushed) = (self.dirty, self.unpushed.clone());
        let picked: Vec<Cand> = self.cands.iter().flatten().filter(|x| self.sel.contains(&x.c.path) && !x.locked()).cloned().collect();
        let (tab, argv) = (self.props.tab.clone(), self.props.argv.clone());
        cx.spawn_in(window, async move |this, cx| {
            let result: Result<(), String> = match mode {
                Mode::New => {
                    let (r2, on2) = (repo.clone(), on.clone());
                    match cx.background_executor().spawn(async move { create(&typed, &r2, &root, &prefs, &on2) }).await {
                        Ok(path) => {
                            // Leave the space unnamed so it follows the checkout folder and nests under the repo.
                            let mut args = json!({ "folder": path, "machine": on.clone().unwrap_or_default() });
                            if let Some(t) = tab {
                                args["tab"] = json!(t);
                            }
                            if let Some(a) = argv {
                                args["argv"] = a;
                            }
                            let _ = this.update_in(cx, |this, window, cx| {
                                let _ = this.app.update(cx, |app, cx| {
                                    let made = app.run_as("space.new", args, true, window, cx);
                                    // The new space is its source's child: the sidebar nests it there.
                                    if let (Some(parent), Some(child)) = (parent, made.get("space").and_then(Value::as_str)) {
                                        app.run("space.parent", json!({ "id": child, "parent": parent }), window, cx);
                                    }
                                    if let Some(error) = made.get("error").and_then(Value::as_str) {
                                        app.command_error("space.new", error, window, cx);
                                    }
                                });
                            });
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                }
                Mode::Remove => {
                    let (r2, on2, u2) = (repo.clone(), on.clone(), unpushed.clone());
                    // Risk that appeared since the dialog opened gets one more warning.
                    let checked = cx
                        .background_executor()
                        .spawn(async move {
                            let status = git(&r2.top, &STATUS, &on2, 30);
                            if !status.ok {
                                return Err(if status.err.is_empty() { i18n::t("worktree.git_status_failed") } else { status.err });
                            }
                            let (mods, now) = submodules(&r2.top, &on2)?;
                            let changed = (!status.out.is_empty() && !dirty) || now.iter().any(|n| !u2.contains(n));
                            Ok((changed, !status.out.is_empty(), now, mods))
                        })
                        .await;
                    match checked {
                        Err(e) => Err(e),
                        Ok((true, now_dirty, now, _)) => {
                            let _ = this.update_in(cx, |this, _, _| {
                                this.dirty |= now_dirty;
                                for n in now {
                                    if !this.unpushed.contains(&n) {
                                        this.unpushed.push(n);
                                    }
                                }
                            });
                            Err(i18n::t("worktree.it_changed_since_this_opened"))
                        }
                        Ok((false, now_dirty, _, mods)) => {
                            // Terminals first: a shell sitting in the folder keeps it busy on Windows.
                            if let Some(id) = space_id {
                                let _ = this.update_in(cx, |this, window, cx| {
                                    let _ = this.app.update(cx, |app, cx| app.run("space.close", json!({ "id": id, "confirmed": true }), window, cx));
                                });
                            }
                            let force = dirty || now_dirty || mods > 0;
                            let (r2, on2) = (repo.clone(), on.clone());
                            let removed = cx
                                .background_executor()
                                .spawn(async move {
                                    let mut args = vec!["worktree", "remove"];
                                    if force {
                                        args.push("--force");
                                    }
                                    args.push(&r2.top);
                                    git(&r2.main, &args, &on2, 600)
                                })
                                .await;
                            if removed.ok { Ok(()) } else { Err(if removed.err.is_empty() { i18n::t("worktree.git_worktree_remove_failed") } else { removed.err }) }
                        }
                    }
                }
                Mode::Cleanup => {
                    let mut failed: Vec<String> = Vec::new();
                    for x in picked {
                        if let Some(id) = x.space.clone() {
                            let _ = this.update_in(cx, |this, window, cx| {
                                let _ = this.app.update(cx, |app, cx| app.run("space.close", json!({ "id": id, "confirmed": true }), window, cx));
                            });
                        }
                        let (main, on2, path) = (repo.main.clone(), on.clone(), x.c.path.clone());
                        let removed = cx.background_executor().spawn(async move { git(&main, &["worktree", "remove", &path], &on2, 600) }).await;
                        if !removed.ok {
                            failed.push(format!("{}: {}", x.c.branch.clone().unwrap_or(x.c.path.clone()), if removed.err.is_empty() { i18n::t("worktree.git_worktree_remove_failed") } else { removed.err }));
                        }
                    }
                    if failed.is_empty() { Ok(()) } else { Err(failed.join("\n")) }
                }
                Mode::Open => Ok(()),
            };
            let _ = this.update_in(cx, |this, window, cx| {
                this.busy = false;
                match result {
                    Ok(()) => this.close(window, cx),
                    Err(e) => this.error = e,
                }
                cx.notify();
            });
        })
        .detach();
    }

    // Drawing ------------------------------------------------------------------------

    fn not_git(&self) -> String {
        let folder = if self.folder.is_empty() { i18n::t("worktree.this_folder") } else { tilde(&self.folder, &self.user_home) };
        i18n::tf("worktree.folder_is_not_in_a", &[("folder", &folder)])
    }

    fn open_row(&self, i: usize, c: &Checkout, there: bool, cx: &mut Context<Self>) -> Stateful<Div> {
        let on = i == self.at;
        let (open_c, remove_c) = (c.clone(), c.clone());
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |t| t.as_secs() as i64);
        div()
            .id(SharedString::from(format!("wt-{i}")))
            .h(z(48.0))
            .flex_none()
            .px(z(10.0))
            .flex()
            .flex_col()
            .justify_center()
            .rounded(r(8.0))
            .cursor_pointer()
            .when(on, |d| d.bg(c_("selected")))
            .hover(|s| s.bg(c_("hover")))
            .on_click(cx.listener(move |this, _, w, cx| this.open(&open_c, w, cx)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(z(8.0))
                    .child(div().id("wt-branch").flex_1().min_w_0().truncate().tooltip(super::super::native::tip(c.branch.clone().unwrap_or_else(|| i18n::t("worktree.detached")))).child(c.branch.clone().unwrap_or_else(|| i18n::t("worktree.detached"))))
                    .when(c.main, |d| d.child(div().flex_none().text_size(z(12.0)).text_color(tone("muted")).child(i18n::t("worktree.main_checkout"))))
                    .when(there, |d| d.child(div().flex_none().text_size(z(12.0)).text_color(tone("accent")).child(i18n::t("worktree.open"))))
                    .when(!there && c.at > 0, |d| d.child(div().flex_none().text_size(z(12.0)).text_color(tone("muted")).child(ago(now - c.at))))
                    // The main checkout cannot be removed: git refuses it, so there is no button.
                    .when(!c.main, |d| {
                        d.child(
                            div()
                                .id(SharedString::from(format!("wt-remove-{i}")))
                                .size(z(32.0))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(r(8.0))
                                .hover(|s| s.bg(c_("raised")))
                                .tooltip(super::super::native::tip(i18n::t("menu.remove_worktree")))
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .on_click(cx.listener(move |this, _, w, cx| {
                                    cx.stop_propagation();
                                    this.remove_row(&remove_c, w, cx)
                                }))
                                .child(icon(X, 12.0, tone("muted"))),
                        )
                    }),
            )
            .child(div().id("wt-path").truncate().text_size(z(12.0)).text_color(tone("muted")).tooltip(super::super::native::tip(tilde(&c.path, &self.user_home))).child(tilde(&c.path, &self.user_home)))
    }

    fn open_body(&mut self, window: &Window, cx: &mut Context<Self>) -> Div {
        let focused = self.input.read(cx).focus_handle(cx).is_focused(window);
        let rows = self.shown(cx);
        self.at = self.at.min(rows.len().saturating_sub(1));
        let first = self.at.saturating_sub(SHOWN - 1).min(rows.len().saturating_sub(SHOWN));
        let all = self.list.clone().unwrap_or_default();
        let d = data();
        let items: Vec<Stateful<Div>> = rows.iter().enumerate().skip(first).take(SHOWN).map(|(i, c)| self.open_row(i, c, space_of(&c.path, &all, &self.on, &d.spaces, &d.panes).is_some(), cx)).collect();
        let this = cx.entity();
        div()
            .flex()
            .flex_col()
            .gap(z(8.0))
            .child(dialog::field_box(focused, false).child(div().flex_1().h_full().child(Input::new(&self.input))))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .children(items)
                    .when(rows.is_empty(), |d| d.child(div().p(z(10.0)).text_size(z(13.0)).text_color(tone("muted")).child(i18n::t("worktree.no_matching_worktrees"))))
                    // Only the main checkout exists: say what a worktree is for, and offer the first one.
                    .when(all.len() <= 1 && self.typed(cx).trim().is_empty(), |d| {
                        d.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(z(8.0))
                                .p(z(10.0))
                                .items_start()
                                .child(dialog::message(i18n::t("worktree.only_main_hint"), false))
                                .child(dialog::side_button("wt-empty-new", i18n::t("menu.new_worktree"), dialog::run(&this, Self::new_from_list))),
                        )
                    }),
            )
            .child(dialog::message(i18n::t("worktree.choose_open"), false))
    }

    fn cleanup_body(&mut self, cx: &mut Context<Self>) -> Div {
        let list = self.cands.clone().unwrap_or_default();
        let rows: Vec<Stateful<Div>> = list
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let locked = x.locked();
                let on = self.sel.contains(&x.c.path) && !locked;
                let path = x.c.path.clone();
                let key_path = x.c.path.clone();
                let state = if x.busy { "worktree.clean_busy" } else if x.dirty { "worktree.clean_dirty" } else if x.merged { "worktree.clean_merged" } else { "worktree.clean_unmerged" };
                div()
                    .id(SharedString::from(format!("wt-clean-{i}")))
                    .h(z(48.0))
                    .flex_none()
                    .px(z(10.0))
                    .flex()
                    .items_center()
                    .gap(z(10.0))
                    .rounded(r(8.0))
                    .border_2()
                    .border_color(Hsla { a: 0.0, ..c_("accent") })
                    .when(!locked, |d| {
                        d.cursor_pointer()
                            .focusable()
                            .tab_stop(true)
                            .focus_visible(|s| s.border_color(graphic("accent")))
                            .hover(|s| s.bg(c_("hover")))
                            .on_key_down(cx.listener(move |this, e: &KeyDownEvent, _, cx| {
                                if matches!(e.keystroke.key.as_str(), "space" | "enter") {
                                    this.toggle(&key_path, cx);
                                    cx.stop_propagation();
                                }
                            }))
                            .on_click(cx.listener(move |this, _, _, cx| this.toggle(&path, cx)))
                    })
                    .child(
                        div()
                            .size(z(18.0))
                            .flex_none()
                            .rounded(r(5.0))
                            .border_1()
                            .border_color(if locked { tone("muted") } else { c_("input") })
                            .bg(if on { c_("accent") } else { c_("background") })
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(on, |d| d.child(icon(CHECK, 12.0, c_("accent_foreground")))),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().id("wt-row-branch").truncate().text_color(if locked { tone("muted") } else { c_("foreground") }).tooltip(super::super::native::tip(x.c.branch.clone().unwrap_or_else(|| i18n::t("worktree.detached")))).child(x.c.branch.clone().unwrap_or_else(|| i18n::t("worktree.detached"))))
                            .child(div().id("wt-row-path").truncate().text_size(z(12.0)).text_color(tone("muted")).tooltip(super::super::native::tip(tilde(&x.c.path, &self.user_home))).child(tilde(&x.c.path, &self.user_home))),
                    )
                    .child(div().flex_none().text_size(z(12.0)).text_color(if locked { tone("blocked") } else if x.merged { tone("accent") } else { tone("muted") }).child(i18n::t(state)))
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .gap(z(8.0))
            .when(list.is_empty(), |d| d.child(dialog::message(i18n::t("worktree.clean_none"), false)))
            .child(div().id("wt-clean-list").flex().flex_col().max_h(z(300.0)).overflow_y_scroll().children(rows))
            .child(dialog::message(i18n::t("worktree.clean_note"), false))
    }
}

/// A theme color.
fn c_(name: &str) -> Hsla {
    super::super::c(name)
}

impl Render for WorktreeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();
        let repo = self.repo().cloned();
        let name = self.props.name.clone().or_else(|| self.props.space.as_deref().and_then(|id| data().spaces.into_iter().find(|s| text(s, "id") == id).map(|s| text(&s, "name").to_owned()))).unwrap_or_default();
        let (title, detail): (String, String) = match (self.mode, &self.repo, &repo) {
            (_, None, _) => (i18n::t(match self.mode { Mode::Open => "worktree.open_worktree", Mode::Cleanup => "worktree.clean_title", Mode::Remove => "worktree.remove_worktree_name", Mode::New => "worktree.new_worktree" }).replace("{name}", &name), "…".into()),
            (_, Some(None), _) => (i18n::t(if self.mode == Mode::Open { "worktree.open_worktree" } else { "worktree.new_worktree" }), self.not_git()),
            (Mode::New, _, Some(r)) => (
                i18n::tf("worktree.new_worktree_of_name", &[("name", &r.name)]),
                i18n::tf("worktree.a_new_space_on_a", &[("path", &format!("{}/{}", tilde(self.root.as_deref().unwrap_or_default(), &self.user_home), if self.prefs.nest { format!("{}/", r.name) } else { String::new() }))]),
            ),
            (Mode::Remove, _, Some(r)) => (
                i18n::tf("worktree.remove_worktree_name", &[("name", &name)]),
                if r.linked { i18n::tf("worktree.closes_the_space_and_deletes", &[("path", &tilde(&r.top, &self.user_home))]) } else { i18n::t("worktree.this_is_the_repository_s_main") },
            ),
            (Mode::Open, _, Some(r)) => {
                let (shown, all) = (self.shown(cx).len(), self.list.as_ref().map_or(0, Vec::len));
                (i18n::tf("worktree.open_worktree_of_name", &[("name", &r.name)]), if shown == all { i18n::tf("worktree.checkouts", &[("n", &all.to_string())]) } else { i18n::tf("worktree.checkouts_of", &[("n", &shown.to_string()), ("total", &all.to_string())]) })
            }
            (Mode::Cleanup, _, Some(r)) => (i18n::tf("worktree.clean_title", &[("name", &r.name)]), String::new()),
            (_, Some(Some(_)), None) => (String::new(), String::new()),
        };
        let ready = repo.is_some();
        let mut body = div().flex().flex_col().gap(z(10.0));
        match self.mode {
            Mode::New if ready => {
                let focused = self.input.read(cx).focus_handle(cx).is_focused(window);
                body = body.child(dialog::field_box(focused, !self.error.is_empty()).child(div().flex_1().h_full().child(Input::new(&self.input))));
                // Only a space can be a parent.
                if let Some(source) = self.props.space.as_deref().and_then(|id| data().spaces.into_iter().find(|s| text(s, "id") == id)) {
                    let this = this.clone();
                    let label = i18n::tf("worktree.from_here_of", &[("name", text(&source, "name"))]);
                    body = body.child(dialog::toggle("worktree-from-here", label, self.from_here, move |_, cx| {
                        this.update(cx, |v, cx| {
                            v.from_here = !v.from_here;
                            cx.notify();
                        })
                    }));
                }
            }
            Mode::Open if ready && self.list.is_some() => body = self.open_body(window, cx),
            Mode::Cleanup if ready && self.cands.is_some() => body = self.cleanup_body(cx),
            Mode::Remove if ready => {
                body = body
                    .when(self.dirty, |d| d.child(dialog::message(i18n::t("worktree.it_has_uncommitted_changes_they"), true)))
                    .when(!self.unpushed.is_empty(), |d| d.child(dialog::message(i18n::tf("worktree.submodule_name_has_commits_that", &[("name", &self.unpushed.join(", "))]), true)));
            }
            _ => {}
        }
        body = body.when(!self.error.is_empty() && !self.busy, |d| d.child(dialog::message(self.error.clone(), true))).when(self.busy, |d| d.child(dialog::message(i18n::t("worktree.working"), false)));
        let mut dlg = Dialog::new(title, &self.focus, dialog::run(&this, Self::close)).description(detail).body(body);
        dlg = match self.mode {
            Mode::New => dlg.fields(1).action(Act::primary(i18n::t("worktree.create"), dialog::run(&this, Self::go)).enabled(ready && !self.busy)),
            Mode::Open => dlg.fields(1).width(560.0).no_cancel(),
            Mode::Remove => dlg.action(Act::danger(i18n::t("menu.remove_worktree"), dialog::run(&this, Self::go)).enabled(repo.as_ref().is_some_and(|r| r.linked) && !self.busy)),
            Mode::Cleanup => {
                let n = self.cands.iter().flatten().filter(|x| self.sel.contains(&x.c.path) && !x.locked()).count();
                let label = i18n::tf(if self.armed { "worktree.clean_confirm" } else { "worktree.clean_go" }, &[("count", &n.to_string())]);
                dlg.width(560.0).action(Act::danger(label, dialog::run(&this, Self::go)).enabled(n > 0 && !self.busy))
            }
        };
        // Up and Down move the open list's highlight (wrapping); the dialog keeps Esc, Tab and Enter.
        let opening = self.mode == Mode::Open;
        div().on_key_down(cx.listener(move |this, e: &KeyDownEvent, _, cx| {
            if !opening {
                return;
            }
            let n = this.shown(cx).len();
            match e.keystroke.key.as_str() {
                "down" if n > 0 => this.at = (this.at + 1) % n,
                "up" if n > 0 => this.at = (this.at + n - 1) % n,
                _ => return,
            }
            cx.stop_propagation();
            cx.notify();
        })).child(dlg.render("worktree"))
    }
}

impl AppWindow {
    /// Opens the worktree dialog (`props` as the mod took them).
    pub(in crate::window) fn open_worktree(&mut self, props: Value, window: &mut Window, cx: &mut Context<Self>) {
        let mode = Mode::parse(props.get("action").and_then(Value::as_str));
        let id = "worktree".to_owned();
        if self.overlays.iter().any(|o| o.id == id) {
            self.close_overlay(&id, window, cx);
        }
        let app = cx.weak_entity();
        let props = Props::from(&props);
        let view = cx.new(|cx| WorktreeView::new(app, id.clone(), mode, props, window, cx));
        let width = if matches!(mode, Mode::Open | Mode::Cleanup) { 560.0 } else { dialog::NARROW };
        self.push_dialog_overlay(id, width + 48.0, view.into(), window, cx);
    }
}

// Automatic cleanup --------------------------------------------------------------------------------

/// The branch checkouts are compared with: the remote's default branch, else a local `main` or `master`.
fn default_ref(repo: &Repo, on: &Option<String>) -> Option<String> {
    let head = git(&repo.main, &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"], on, 30);
    if head.ok && !head.out.is_empty() {
        return Some(head.out);
    }
    ["main", "master"].into_iter().find(|n| git(&repo.main, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{n}")], on, 30).ok).map(str::to_owned)
}

/// Days since a checkout's folder was made (its birth time, else its last change); None when unknown.
fn folder_age_days(path: &str) -> Option<f64> {
    let meta = std::fs::metadata(path).ok()?;
    let at = meta.created().or_else(|_| meta.modified()).ok()?;
    Some(at.elapsed().ok()?.as_secs_f64() / 86_400.0)
}

/// Every checkout (but a repo's main one) of the repos the open spaces on this computer belong to, with what the
/// cleanup rules say about it. Off the UI thread.
pub(in crate::window) fn cleanup_scan(spaces: &[Value], panes: &[Value], agents: &[Value], pinned: &[String], done: &[String], prefs: &super::super::cleanup::Prefs) -> Vec<(super::super::cleanup::Cand, super::super::cleanup::Verdict)> {
    use super::super::cleanup::{Cand, judge};
    let on: Option<String> = None;
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for sp in spaces.iter().filter(|s| opt_text(s, "machine").is_none_or(str::is_empty)) {
        let folder = place(sp, panes);
        let Some(repo) = (!folder.is_empty()).then(|| repo_of(&folder, &on)).flatten() else { continue };
        if !seen.insert(repo.main.clone()) {
            continue;
        }
        let list = checkouts(&repo, &on);
        let default = default_ref(&repo, &on);
        let merged = default.as_ref().map(|d| merged_branches(&git(&repo.main, &["branch", "--merged", d], &on, 30).out)).unwrap_or_default();
        let tip = |r: &str| git(&repo.main, &["rev-parse", "--verify", "--quiet", r], &on, 30);
        let default_tip = default.as_deref().map(tip).filter(|t| t.ok).map(|t| t.out);
        for c in list.iter().filter(|c| !c.main) {
            let id = space_of(&c.path, &list, &on, spaces, panes);
            let space = id.as_deref().and_then(|id| spaces.iter().find(|s| text(s, "id") == id));
            let branch_tip = c.branch.as_deref().map(|b| tip(&format!("refs/heads/{b}"))).filter(|t| t.ok).map(|t| t.out);
            let own_commits = branch_tip.is_some() && branch_tip != default_tip;
            let ahead = match (&default, &c.branch) {
                (Some(d), Some(b)) => Some(git(&repo.main, &["rev-list", "--count", &format!("{d}..refs/heads/{b}")], &on, 30)).filter(|g| g.ok).and_then(|g| g.out.parse::<u64>().ok()),
                _ => None,
            };
            let cand = Cand {
                path: c.path.clone(),
                repo: repo.main.clone(),
                branch: c.branch.clone(),
                space: id.clone(),
                main: false,
                pinned: id.as_ref().is_some_and(|i| pinned.contains(i)),
                busy: id.as_ref().is_some_and(|i| agents.iter().any(|a| text(a, "space") == i && text(a, "status") == "working")),
                dirty: !git(&c.path, &STATUS, &on, 30).out.is_empty(),
                merged: own_commits && c.branch.as_ref().is_some_and(|b| merged.contains(b)),
                ahead,
                age_days: folder_age_days(&c.path),
                bot_done: space.is_some_and(|s| done.iter().any(|n| n == text(s, "name"))),
            };
            let verdict = judge(&cand, prefs);
            out.push((cand, verdict));
        }
    }
    out
}

/// Removes the checkouts (`git worktree remove`, never --force: what changed since the scan stays). Off the UI thread.
pub(in crate::window) fn cleanup_delete(items: Vec<(super::super::cleanup::Cand, super::super::cleanup::Reason)>) -> Vec<(super::super::cleanup::Cand, super::super::cleanup::Reason, Result<(), String>)> {
    items
        .into_iter()
        .map(|(c, r)| {
            let removed = git(&c.repo, &["worktree", "remove", &c.path], &None, 600);
            let result = if removed.ok { Ok(()) } else { Err(if removed.err.is_empty() { i18n::t("worktree.git_worktree_remove_failed") } else { removed.err }) };
            (c, r, result)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    // `super::*` brings gpui's own `test` macro: the plain one is the std attribute.
    use core::prelude::v1::test;

    #[test]
    fn a_branch_gets_a_folder_of_letters_marks_and_digits() {
        for (branch, folder) in [
            ("음악수행", "음악수행"), ("쪼지학", "쪼지학"), ("हिन्दी", "हिन्दी"), ("हिंदी", "हिंदी"), ("का", "का"), ("कि", "कि"), ("বাংলা", "বাংলা"),
            ("feature/हिन्दी", "feature-हिन्दी"), ("feature/음악수행-１２٣", "feature-음악수행-１２٣"),
            ("\u{110B}\u{1173}\u{11B7}\u{110B}\u{1161}\u{11A8}\u{1109}\u{116E}\u{1112}\u{1162}\u{11BC}", "음악수행"),
            ("feature/ASCII_1.2--fix", "feature-ASCII_1.2--fix"),
        ] {
            assert_eq!(slug(branch), folder, "{branch}");
        }
    }

    #[test]
    fn symbol_only_branches_get_stable_distinct_non_empty_folders() {
        let (a, b) = (slug("///"), slug("!!!"));
        assert!(a.starts_with("branch-") && b.starts_with("branch-"));
        assert_ne!(a, b);
        assert_eq!(a, slug("///"));
        assert_eq!(a.len(), "branch-".len() + 8);
    }

    #[test]
    fn the_worktrees_folder_defaults_expands_home_and_must_be_absolute() {
        assert_eq!(worktrees_root(None, "/home/u"), Ok("/home/u/.herdr/worktrees".into()));
        assert_eq!(worktrees_root(Some(r#"{"worktrees":{"directory":"~/Developer/wt/"}}"#), "/home/u"), Ok("/home/u/Developer/wt".into()));
        assert_eq!(worktrees_root(Some(r#"{"worktrees":{"directory":"C:\\wt"}}"#), "C:\\Users\\u"), Ok("C:/wt".into()));
        assert_eq!(worktrees_root(Some(r#"{"worktrees":{"directory":"rel/wt"}}"#), "/h"), Err("worktree.worktrees_directory_must_be_an_absolute"));
        assert_eq!(worktrees_root(Some(r#"{"worktrees":{"directory":"  "}}"#), "/h"), Err("worktree.worktrees_directory_must_be_a_non_empty"));
        assert_eq!(worktrees_root(Some(r#"{"worktrees":{"directory":3}}"#), "/h"), Err("worktree.worktrees_directory_must_be_a_non_empty"));
    }

    #[test]
    fn home_shows_as_a_tilde_even_when_git_and_windows_disagree_on_slashes() {
        assert_eq!(tilde("/home/u/x", "/home/u"), "~/x");
        assert_eq!(tilde("C:/Users/x/p", "C:\\Users\\x"), "~/p");
        assert_eq!(tilde("/etc/x", "/home/u"), "/etc/x");
    }

    fn rules(template: &str, mode: Option<&str>, prefix: &str) -> Rules {
        Rules { template: template.into(), mode: mode.map(str::to_owned), prefix: prefix.into() }
    }

    #[test]
    fn a_typed_name_becomes_a_branch_by_the_template_and_the_prefix() {
        let never = || -> bool { panic!("not asked") };
        // Defaults: the name as typed, no git asked.
        assert_eq!(branch_name("fix", &rules("", None, ""), "repo", "2026-01-02", || None, never), "fix");
        assert_eq!(branch_name("fix", &rules("{name}", Some("none"), ""), "repo", "2026-01-02", || None, never), "fix");
        assert_eq!(branch_name("fix", &rules("{date}-{name}", None, ""), "repo", "2026-01-02", || None, || false), "2026-01-02-fix");
        assert_eq!(branch_name("fix", &rules("{repo}", None, ""), "repo", "d", || None, || false), "repo-fix");
        assert_eq!(branch_name("fix", &rules("", Some("custom"), "feat"), "repo", "d", || None, || false), "feat/fix");
        assert_eq!(branch_name("fix", &rules("", Some("custom"), "feat-"), "repo", "d", || None, || false), "feat-fix");
        assert_eq!(branch_name("fix", &rules("", Some("git"), ""), "repo", "d", || Some("Ada Lovelace".into()), || false), "ada-lovelace/fix");
        // A name that is already a branch is used as typed.
        assert_eq!(branch_name("fix", &rules("", Some("custom"), "feat"), "repo", "d", || None, || true), "fix");
        // Already carrying the prefix.
        assert_eq!(branch_name("feat/x", &rules("", Some("custom"), "feat/"), "repo", "d", || None, || false), "feat/x");
    }

    #[test]
    fn checkouts_come_main_first_then_the_latest_commit_and_skip_bare_and_prunable() {
        let list = "worktree /r\nHEAD aaa\nbranch refs/heads/main\n\nworktree /r-old\nHEAD bbb\nbranch refs/heads/old\n\nworktree /r-new\nHEAD ccc\nbranch refs/heads/new\n\nworktree /gone\nHEAD ddd\nprunable gitdir file points to non-existent location\n\nworktree /bare\nbare\n\nworktree /det\nHEAD eee\ndetached";
        let refs = "main\t10\nold\t100\nnew\t200";
        let out = parse_checkouts(list, refs);
        let names: Vec<_> = out.iter().map(|c| c.path.as_str()).collect();
        assert_eq!(names, ["/r", "/r-new", "/r-old", "/det"]);
        assert!(out[0].main && !out[1].main);
        assert_eq!(out[3].branch, None);
    }

    #[test]
    fn ages_are_short() {
        assert_eq!(ago(30), "1m");
        assert_eq!(ago(600), "10m");
        assert_eq!(ago(2 * 3600), "2h");
        assert_eq!(ago(3 * 86_400), "3d");
        assert_eq!(ago(90 * 86_400), "3mo");
    }

    #[test]
    fn a_checkout_is_shown_by_the_space_whose_terminal_is_in_the_deepest_one() {
        let all = vec![
            Checkout { path: "/repo".into(), branch: Some("main".into()), main: true, at: 0 },
            Checkout { path: "/repo/wt".into(), branch: Some("wt".into()), main: false, at: 0 },
        ];
        let spaces = vec![json!({ "id": "s1", "tabs": [{ "id": "t1" }] }), json!({ "id": "s2", "tabs": [{ "id": "t2" }], "machine": "srv" })];
        let panes = vec![json!({ "tab": "t1", "cwd": "/repo/wt/src" }), json!({ "tab": "t2", "cwd": "/repo" })];
        assert_eq!(space_of("/repo/wt", &all, &None, &spaces, &panes).as_deref(), Some("s1"));
        // The main checkout is not the one a terminal inside the worktree is in.
        assert_eq!(space_of("/repo", &all, &None, &spaces, &panes), None);
        // A space on another machine is another place.
        assert_eq!(space_of("/repo", &all, &Some("srv".into()), &spaces, &panes).as_deref(), Some("s2"));
        // Without a live folder the space's own folder counts.
        let by_folder = vec![json!({ "id": "s3", "folder": "/repo/wt", "tabs": [] })];
        assert_eq!(space_of("/repo/wt", &all, &None, &by_folder, &[]).as_deref(), Some("s3"));
    }

    #[test]
    fn merged_branches_drop_the_markers() {
        let out = merged_branches("* main\n  done\n+ linked\n\n");
        assert!(out.contains("main") && out.contains("done") && out.contains("linked") && out.len() == 3);
    }

    fn sh(cwd: &std::path::Path, args: &[&str]) {
        let status = std::process::Command::new("git").args(args).current_dir(cwd).env("GIT_AUTHOR_NAME", "t").env("GIT_AUTHOR_EMAIL", "t@t").env("GIT_COMMITTER_NAME", "t").env("GIT_COMMITTER_EMAIL", "t@t").status().expect("git");
        assert!(status.success(), "git {args:?}");
    }

    /// A real repo: a new worktree is made under the worktrees folder, listed, and removed again.
    #[test]
    fn a_worktree_is_created_listed_and_removed_in_a_real_repo() {
        let tmp = std::env::temp_dir().join(format!("sunna-wt-{}", std::process::id()));
        let repo_dir = tmp.join("repo");
        std::fs::create_dir_all(&repo_dir).unwrap();
        sh(&repo_dir, &["init", "-q", "-b", "main"]);
        sh(&repo_dir, &["commit", "-q", "--allow-empty", "-m", "first"]);
        let folder = std::fs::canonicalize(&repo_dir).unwrap().to_string_lossy().into_owned();
        let repo = repo_of(&folder, &None).expect("a repo");
        assert_eq!((repo.name.as_str(), repo.linked), ("repo", false));
        let root = tmp.join("wts").to_string_lossy().into_owned();
        let prefs = Prefs { nest: true, ..Prefs::default() };
        let path = create("feature/x", &repo, &root, &prefs, &None).expect("created");
        assert_eq!(path, format!("{root}/repo/feature-x"));
        assert!(std::path::Path::new(&path).join(".git").exists());
        assert_eq!(create("", &repo, &root, &prefs, &None).unwrap_err(), i18n::t("worktree.type_a_branch_name"));
        let list = checkouts(&repo, &None);
        assert_eq!(list.len(), 2);
        assert!(list[0].main && list[1].branch.as_deref() == Some("feature/x"));
        let linked = repo_of(&path, &None).expect("linked repo");
        assert!(linked.linked && linked.name == "repo");
        assert!(git(&repo.main, &["worktree", "remove", &path], &None, 30).ok);
        assert_eq!(checkouts(&repo, &None).len(), 1);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
