//! The Changes panel (`{"native": "changes"}`, once the `changes` mod): git status of the
//! focused pane's repository, drawn as Orca's "Changes" panel: the branch and what it is ahead
//! or behind, fetch / pull / push, a commit message, the STAGED and CHANGES lists (stage, unstage,
//! discard a file or all), the commits on the branch, and the diff of whatever is picked.
//! Reading git (status, numstat, merge-base, log, diff) and turning its text into rows are plain
//! functions, tested at the end; the view runs them on a background thread and draws the result.
//! It works on another machine through the core's exec, as Files does.

use super::files::{Place, READ, exec};
use super::native::{fade, icon, keyed, r, ring, step_cursor, tip, tone, z};
use super::*;
use crate::i18n;
use gpui::{UniformListScrollHandle, uniform_list};
use gpui_base::input::{Input, InputEvent, InputState};

macro_rules! change_icon {
    ($name:literal) => {
        include_bytes!(concat!("../../assets/icons/changes/", $name, ".svg")).as_slice()
    };
}
const UNDO: &[u8] = change_icon!("undo");
const PLUS: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/plus.svg");
const MINUS: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/minus.svg");
const FILE: &[u8] = include_bytes!("../../assets/icons/files/file.svg");
const BRANCH: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/git-branch.svg");

const ROW_H: f32 = 32.0;
const DIFF_ROW_H: f32 = 20.0;
const FILES_MAX: usize = 1000;
const DIFF_LINES: usize = 1500;
const COMMITS: &str = "50";

// Plain data ----------------------------------------------------------------------------

/// One changed file as a list shows it (`status`: A new, D deleted, U conflict, M changed).
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Change {
    pub path: String,
    pub status: char,
    pub fresh: bool,
    pub add: u32,
    pub del: u32,
    pub binary: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Commit {
    pub hash: String,
    pub subject: String,
    pub author: String,
    pub ago: String,
}

/// What one refresh read from the repository.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Snapshot {
    pub root: String,
    pub branch: Option<String>,
    pub upstream: bool,
    pub ahead: u32,
    pub behind: u32,
    pub has_head: bool,
    pub staged: Vec<Change>,
    pub changes: Vec<Change>,
    pub add: u32,
    pub del: u32,
    /// Compared against a base (Settings > Git): its name, and the commit the diffs are taken from.
    pub base_ref: Option<String>,
    pub diff_base: String,
    pub limited: bool,
    pub commits: Vec<Commit>,
}

/// Why a refresh gave nothing.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Failure {
    /// The folder is no repository: not an error to show in red.
    NotRepo(String),
    Git(String),
}

/// Which list a picked row belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Section {
    Staged,
    Changes,
    Commit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Pick {
    section: Section,
    key: String,
}

// Pure rules ------------------------------------------------------------------------------

/// Counts per path from `git diff --numstat -z`; `-` counts mark a binary file.
pub(super) fn parse_numstat(out: &str, into: &mut HashMap<String, (u32, u32, bool)>) {
    for entry in out.split('\0') {
        let mut parts = entry.split('\t');
        let (Some(a), Some(d)) = (parts.next(), parts.next()) else { continue };
        let path: Vec<&str> = parts.collect();
        if path.is_empty() {
            continue;
        }
        let old = into.entry(path.join("\t")).or_insert((0, 0, false));
        *old = (old.0 + a.parse().unwrap_or(0), old.1 + d.parse().unwrap_or(0), a == "-");
    }
}

/// `(X, Y, path)` of each `git status --porcelain=v1 -z` entry.
pub(super) fn parse_status(out: &str) -> Vec<(char, char, String)> {
    out.split('\0')
        .filter(|e| e.chars().count() > 3)
        .filter_map(|e| {
            let mut chars = e.chars();
            let (x, y) = (chars.next()?, chars.next()?);
            Some((x, y, e.get(3..)?.to_owned()))
        })
        .collect()
}

fn is_conflict(x: char, y: char) -> bool {
    x == 'U' || y == 'U' || (x == 'A' && y == 'A') || (x == 'D' && y == 'D')
}

fn letter(code: char) -> char {
    match code {
        'D' => 'D',
        'A' => 'A',
        _ => 'M',
    }
}

/// The staged list and the changes list from status entries. A file changed in the index and
/// again in the tree is in both; a conflict is only in the changes.
pub(super) fn split_status(entries: &[(char, char, String)], stats: &HashMap<String, (u32, u32, bool)>, staged_stats: &HashMap<String, (u32, u32, bool)>) -> (Vec<Change>, Vec<Change>) {
    let (mut staged, mut changes) = (Vec::new(), Vec::new());
    let stat = |m: &HashMap<String, (u32, u32, bool)>, p: &str| m.get(p).copied().unwrap_or((0, 0, false));
    for (x, y, path) in entries.iter().take(FILES_MAX) {
        let fresh = *x == '?';
        if is_conflict(*x, *y) {
            let (add, del, binary) = stat(stats, path);
            changes.push(Change { path: path.clone(), status: 'U', fresh: false, add, del, binary });
            continue;
        }
        if !matches!(x, ' ' | '?' | '!') {
            let (add, del, binary) = stat(staged_stats, path);
            staged.push(Change { path: path.clone(), status: letter(*x), fresh: false, add, del, binary });
        }
        if fresh || !matches!(y, ' ' | '!') {
            let (add, del, binary) = if fresh { (0, 0, false) } else { stat(stats, path) };
            changes.push(Change { path: path.clone(), status: if fresh { 'A' } else { letter(*y) }, fresh, add, del, binary });
        }
    }
    (staged, changes)
}

/// Against a base the list is what differs from it, committed or not; new files come from status.
pub(super) fn base_files(name_status: &str, entries: &[(char, char, String)], stats: &HashMap<String, (u32, u32, bool)>) -> Vec<Change> {
    let parts: Vec<&str> = name_status.split('\0').filter(|p| !p.is_empty()).collect();
    let mut files: Vec<Change> = parts
        .chunks_exact(2)
        .map(|c| {
            let (add, del, binary) = stats.get(c[1]).copied().unwrap_or((0, 0, false));
            let status = match c[0].chars().next() {
                Some('D') => 'D',
                Some('A') => 'A',
                Some('U') => 'U',
                _ => 'M',
            };
            Change { path: c[1].to_owned(), status, fresh: false, add, del, binary }
        })
        .collect();
    for (x, y, path) in entries {
        if *x == '?' && *y == '?' && !files.iter().any(|f| &f.path == path) {
            files.push(Change { path: path.clone(), status: 'A', fresh: true, add: 0, del: 0, binary: false });
        }
    }
    files.truncate(FILES_MAX);
    files
}

/// `git log` lines `hash \x1f subject \x1f author \x1f ago`.
pub(super) fn parse_commits(out: &str) -> Vec<Commit> {
    out.lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let mut f = l.split('\x1f');
            let mut next = || f.next().unwrap_or_default().to_owned();
            Commit { hash: next(), subject: next(), author: next(), ago: next() }
        })
        .collect()
}

/// `git rev-list --left-right --count @{u}...HEAD` is `behind ahead`.
pub(super) fn parse_counts(out: &str) -> (u32, u32) {
    let mut n = out.split_whitespace().map(|s| s.parse().unwrap_or(0));
    let behind = n.next().unwrap_or(0);
    (behind, n.next().unwrap_or(0))
}

/// Text draws a tab as nothing: pad each to the next 4-column stop.
pub(super) fn untab(text: &str) -> String {
    let mut out = String::new();
    let mut col = 0;
    for ch in text.chars() {
        if ch == '\t' {
            let pad = 4 - col % 4;
            out.extend(std::iter::repeat_n(' ', pad));
            col += pad;
        } else {
            out.push(ch);
            col += 1;
        }
    }
    out
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct DiffRow {
    /// `@` a hunk header, `+`, `-`, ` ` a line, `m` a file header or a message.
    pub kind: char,
    pub number: Option<u32>,
    pub text: String,
}

/// Numbers each diff line once, so a frame only draws the rows on screen.
pub(super) fn diff_rows(lines: &[String]) -> Vec<DiffRow> {
    let (mut old, mut new) = (0u32, 0u32);
    lines
        .iter()
        .map(|line| {
            if let Some(rest) = line.strip_prefix("@@ -") {
                let num = |s: &str| s.chars().take_while(char::is_ascii_digit).collect::<String>().parse::<u32>().unwrap_or(0);
                old = num(rest);
                new = rest.split_once(" +").map_or(0, |(_, n)| num(n));
                return DiffRow { kind: '@', number: None, text: line.clone() };
            }
            let body = |l: &str| {
                let t = untab(l.get(1..).unwrap_or_default());
                if t.is_empty() { " ".to_owned() } else { t }
            };
            match line.chars().next() {
                Some('-') => {
                    old += 1;
                    DiffRow { kind: '-', number: Some(old - 1), text: body(line) }
                }
                Some('+') => {
                    new += 1;
                    DiffRow { kind: '+', number: Some(new - 1), text: body(line) }
                }
                Some(' ') => {
                    old += 1;
                    new += 1;
                    DiffRow { kind: ' ', number: Some(new - 1), text: body(line) }
                }
                _ => DiffRow { kind: 'm', number: None, text: line.clone() },
            }
        })
        .collect()
}

/// A patch as rows: git's own header lines go, the file headers stay.
pub(super) fn patch_lines(text: &str) -> Vec<String> {
    const SKIP: [&str; 9] = ["index ", "--- ", "+++ ", "new file mode", "deleted file mode", "old mode", "new mode", "similarity ", "rename "];
    text.trim_end_matches('\n').split('\n').filter(|l| !SKIP.iter().any(|p| l.starts_with(p)) && !l.starts_with("Binary files")).map(str::to_owned).collect()
}

/// The width the diff needs, for the horizontal scroll (7 px a character, the gutter, some air).
pub(super) fn diff_width(rows: &[DiffRow]) -> f32 {
    rows.iter().map(|r| (r.text.chars().count() as f32 + 1.0) * 7.0 + 50.0).fold(300.0, f32::max)
}

/// The first `@@` on, or `None` when git printed no hunk (a binary file, a mode change).
fn hunks(text: &str) -> Option<Vec<String>> {
    let all: Vec<&str> = text.trim_end_matches('\n').split('\n').collect();
    let first = all.iter().position(|l| l.starts_with("@@"))?;
    Some(all[first..].iter().map(|l| (*l).to_owned()).collect())
}

fn cut_line(line: &str) -> String {
    if line.chars().count() > 4000 {
        format!("{}{}", line.chars().take(4000).collect::<String>(), i18n::t("changes.line_truncated"))
    } else {
        line.to_owned()
    }
}

/// How a status letter reads next to a file: the order of Settings > Git's groups.
pub(super) fn group_order(setting: Option<&str>) -> Option<[char; 4]> {
    match setting {
        Some("status") => Some(['U', 'M', 'A', 'D']),
        Some("new_first") => Some(['U', 'A', 'M', 'D']),
        _ => None,
    }
}

/// One line of the list: all 32 px, so it can be a uniform list.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum ListRow {
    /// A section title with its count, and what its button does to all of it.
    Head { section: Section, count: usize },
    /// A group of the changes (Settings > Git).
    Group { status: char, count: usize },
    File { section: Section, change: Change },
    Commit(Commit),
    Note(String),
}

pub(super) fn list_rows(snap: &Snapshot, groups: Option<[char; 4]>, empty_note: &str) -> Vec<ListRow> {
    let mut rows = Vec::new();
    if !snap.staged.is_empty() {
        rows.push(ListRow::Head { section: Section::Staged, count: snap.staged.len() });
        rows.extend(snap.staged.iter().cloned().map(|change| ListRow::File { section: Section::Staged, change }));
    }
    rows.push(ListRow::Head { section: Section::Changes, count: snap.changes.len() });
    if snap.changes.is_empty() {
        rows.push(ListRow::Note(empty_note.to_owned()));
    }
    match groups {
        None => rows.extend(snap.changes.iter().cloned().map(|change| ListRow::File { section: Section::Changes, change })),
        Some(order) => {
            for status in order {
                let group: Vec<&Change> = snap.changes.iter().filter(|f| f.status == status).collect();
                if group.is_empty() {
                    continue;
                }
                rows.push(ListRow::Group { status, count: group.len() });
                rows.extend(group.into_iter().cloned().map(|change| ListRow::File { section: Section::Changes, change }));
            }
        }
    }
    if !snap.commits.is_empty() {
        rows.push(ListRow::Head { section: Section::Commit, count: snap.commits.len() });
        rows.extend(snap.commits.iter().cloned().map(ListRow::Commit));
    }
    rows
}

/// Whether committing has something to commit.
pub(super) fn can_commit(message: &str, snap: Option<&Snapshot>, busy: bool) -> bool {
    !message.trim().is_empty() && !busy && snap.is_some_and(|s| !s.staged.is_empty() || !s.changes.is_empty()) && snap.is_some_and(|s| s.base_ref.is_none())
}

/// What `commit` runs: the staged files as they are, or everything when nothing is staged.
pub(super) fn commit_steps(snap: &Snapshot, message: &str) -> Vec<Vec<String>> {
    let mut steps = Vec::new();
    if snap.staged.is_empty() {
        steps.push(vec!["add".to_owned(), "-A".to_owned()]);
    }
    steps.push(vec!["commit".to_owned(), "-m".to_owned(), message.trim().to_owned()]);
    steps
}

/// The git command that undoes one file's changes in the tree.
pub(super) fn discard_args(change: &Change) -> Vec<String> {
    let args: &[&str] = if change.fresh { &["clean", "-f", "--"] } else { &["restore", "--"] };
    args.iter().map(|s| (*s).to_owned()).chain([change.path.clone()]).collect()
}

/// The git command that takes one file out of the index.
pub(super) fn unstage_args(has_head: bool, path: &str) -> Vec<String> {
    let args: &[&str] = if has_head { &["restore", "--staged", "--"] } else { &["rm", "--cached", "-q", "--"] };
    args.iter().map(|s| (*s).to_owned()).chain([path.to_owned()]).collect()
}

/// Git's last lines, which say what went wrong.
fn tail(text: &str) -> String {
    let lines: Vec<&str> = text.trim().lines().collect();
    lines[lines.len().saturating_sub(5)..].join("\n")
}

/// What `git rev-parse --show-toplevel` failing means. Only git's own "not a repository" gets the short
/// text: a permission or PATH failure says what it is.
pub(super) fn classify_rev_parse(stderr: &str) -> Failure {
    let why = stderr.trim().lines().next().unwrap_or_default();
    if why.is_empty() || why.to_lowercase().contains("not a git repository") { Failure::NotRepo(i18n::t("changes.not_a_git_repository")) } else { Failure::Git(why.to_owned()) }
}

/// Push sets the upstream on origin when the branch has none.
pub(super) fn push_args(upstream: bool, branch: Option<&str>) -> Vec<String> {
    let args: Vec<&str> = if upstream { vec!["push"] } else { vec!["push", "-u", "origin", branch.unwrap_or("HEAD")] };
    args.into_iter().map(str::to_owned).collect()
}

// Reading git (background thread) -------------------------------------------------------------

struct Git<'a> {
    root: &'a str,
    on: Option<&'a str>,
}

impl Git<'_> {
    fn run(&self, args: &[&str]) -> Result<(i32, String, String), String> {
        let all: Vec<&str> = READ.iter().copied().chain(args.iter().copied()).collect();
        exec(self.on, "git", &all, self.root, 30)
    }

    /// The output, or git's own complaint as the error.
    fn ok(&self, args: &[&str]) -> Result<String, String> {
        match self.run(args)? {
            (0, out, _) => Ok(out),
            (_, out, err) => Err(if err.trim().is_empty() { out } else { err }),
        }
    }

    fn succeeds(&self, args: &[&str]) -> bool {
        matches!(self.run(args), Ok((0, _, _)))
    }
}

/// Settings > Git: what the list compares the working tree with. "head" (the default): what is
/// uncommitted; "default" or "upstream": the branch's whole work since it left the repository's
/// default branch or its upstream. Names the base and gives its merge-base with HEAD.
fn merge_base(git: &Git, mode: &str) -> Option<(String, String)> {
    let found = if mode == "upstream" {
        git.succeeds(&["rev-parse", "--verify", "--quiet", "@{u}"]).then(|| "@{u}".to_owned())
    } else {
        let head = git.ok(&["symbolic-ref", "--short", "refs/remotes/origin/HEAD"]).ok().map(|s| s.trim().to_owned());
        head.into_iter().chain(["origin/main", "origin/master", "main", "master"].map(str::to_owned)).find(|n| git.succeeds(&["rev-parse", "--verify", "--quiet", n]))
    }?;
    let base = git.ok(&["merge-base", "HEAD", &found]).ok()?;
    Some((if found == "@{u}" { "upstream".to_owned() } else { found }, base.trim().to_owned()))
}

pub(super) fn load_status(cwd: &str, on: Option<&str>, mode: &str) -> Result<Snapshot, Failure> {
    let (code, out, err) = exec(on, "git", &["rev-parse", "--show-toplevel"], cwd, 30).map_err(Failure::Git)?;
    if code != 0 {
        // Only git's own "not a repository" gets the short text: a permission or PATH failure says what it is.
        return Err(classify_rev_parse(&err));
    }
    let root = out.trim().to_owned();
    let git = Git { root: &root, on };
    let g = |e: String| Failure::Git(e);
    let entries = parse_status(&git.ok(&["status", "--porcelain=v1", "-z", "--no-renames", "--untracked-files=all"]).map_err(g)?);
    let has_head = git.succeeds(&["rev-parse", "--verify", "HEAD"]);
    let branch = git.ok(&["branch", "--show-current"]).ok().map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
    let counts = git.ok(&["rev-list", "--left-right", "--count", "@{u}...HEAD"]).ok();
    let base = if has_head && mode != "head" { merge_base(&git, mode) } else { None };
    let against = base.as_ref().map_or_else(|| "HEAD".to_owned(), |b| b.1.clone());

    let (mut stats, mut staged_stats) = (HashMap::new(), HashMap::new());
    let numstat = ["diff", "--numstat", "-z", "--no-renames"];
    if has_head {
        parse_numstat(&git.ok(&["diff", &against, "--numstat", "-z", "--no-renames"]).map_err(Failure::Git)?, &mut stats);
    } else {
        parse_numstat(&git.ok(&["diff", "--cached", "--numstat", "-z", "--no-renames"]).map_err(Failure::Git)?, &mut stats);
        parse_numstat(&git.ok(&numstat).map_err(Failure::Git)?, &mut stats);
    }
    let (staged, changes) = if let Some(b) = &base {
        let named = git.ok(&["diff", "--name-status", "-z", "--no-renames", &b.1]).map_err(Failure::Git)?;
        (Vec::new(), base_files(&named, &entries, &stats))
    } else {
        parse_numstat(&git.ok(&["diff", "--cached", "--numstat", "-z", "--no-renames"]).map_err(Failure::Git)?, &mut staged_stats);
        let mut work = HashMap::new();
        parse_numstat(&git.ok(&numstat).map_err(Failure::Git)?, &mut work);
        split_status(&entries, &work, &staged_stats)
    };
    let (add, del) = changes.iter().chain(&staged).fold((0, 0), |(a, d), f| (a + f.add, d + f.del));
    let (behind, ahead) = counts.as_deref().map_or((0, 0), parse_counts);

    // The commits on the branch: since it left the default branch, else the latest ones.
    let range = merge_base(&git, "default").map(|b| format!("{}..HEAD", b.1));
    let mut log = vec!["log", "-n", COMMITS, "--format=%h%x1f%s%x1f%an%x1f%ar"];
    if let Some(r) = &range {
        log.push(r);
    }
    let commits = if has_head { git.ok(&log).map(|o| parse_commits(&o)).unwrap_or_default() } else { Vec::new() };
    Ok(Snapshot {
        root,
        branch,
        upstream: counts.is_some(),
        ahead,
        behind,
        has_head,
        staged,
        changes,
        add,
        del,
        base_ref: base.map(|b| b.0),
        diff_base: against,
        limited: entries.len() > FILES_MAX,
        commits,
    })
}

/// What picking a row shows: the lines of its diff.
fn load_diff(snap: &Snapshot, on: Option<&str>, pick: &Pick) -> Result<Vec<String>, String> {
    let git = Git { root: &snap.root, on };
    let path = pick.key.as_str();
    if pick.section == Section::Commit {
        let out = git.ok(&["show", "--format=", "--patch", "--no-ext-diff", "--no-textconv", "--no-color", path])?;
        return Ok(patch_lines(&out).into_iter().take(DIFF_LINES).collect());
    }
    let list = if pick.section == Section::Staged { &snap.staged } else { &snap.changes };
    let Some(file) = list.iter().find(|f| f.path == path) else { return Ok(Vec::new()) };
    if pick.section == Section::Changes && (file.fresh || (!snap.has_head && file.status == 'A')) {
        let full = format!("{}/{path}", snap.root);
        let text = match on {
            Some(machine) => mods::read_remote(machine, &full),
            None => crate::mod_files::read(std::path::Path::new(&full)),
        };
        return Ok(match text {
            Ok(Some(t)) if t.contains('\0') => vec![i18n::t("changes.binary_file_no_text")],
            Ok(Some(t)) => {
                let body: String = t.chars().take(400_000).collect();
                let lines: Vec<&str> = body.trim_end_matches('\n').split('\n').collect();
                let n = t.trim_end_matches('\n').split('\n').count();
                std::iter::once(format!("@@ -0,0 +1,{n} @@")).chain(lines.into_iter().take(DIFF_LINES).map(|l| format!("+{l}"))).collect()
            }
            _ => vec![i18n::t("changes.cannot_preview_this_file_binary")],
        });
    }
    let mut args = vec!["diff"];
    if pick.section == Section::Staged {
        args.push("--cached");
    } else if snap.has_head {
        args.push(&snap.diff_base);
    }
    args.extend(["--no-ext-diff", "--no-textconv", "--no-color", "--", path]);
    let out = git.ok(&args)?;
    Ok(match hunks(&out) {
        Some(l) => l.into_iter().take(DIFF_LINES).collect(),
        None => vec![i18n::t(if file.binary { "changes.binary_file_changed" } else { "changes.no_text_diff_available" })],
    })
}

// The view ----------------------------------------------------------------------------------------

pub(super) struct ChangesView {
    place: Place,
    raw: Place,
    generation: u64,
    input: Entity<InputState>,
    branch_input: Entity<InputState>,
    typing: bool,
    message: String,
    snap: Option<Snapshot>,
    error: Option<Failure>,
    loading: bool,
    again: bool,
    /// A write (commit, stage, fetch, ...) runs: the list waits for it.
    busy: Option<String>,
    note: Option<String>,
    rows: Vec<ListRow>,
    open: Option<Pick>,
    /// The person closed the diff: it stays closed until they pick a file.
    closed: bool,
    diff: Vec<DiffRow>,
    diff_width: f32,
    branches: Option<Vec<String>>,
    branch_filter: String,
    armed: Option<Pick>,
    /// The row under the pointer: its actions show (a style that changes layout on hover would
    /// differ between a frame's prepaint and paint).
    hover: Option<Pick>,
    base_mode: String,
    clear_input: bool,
    clear_branch_input: bool,
    seen: Instant,
    loaded: Option<Instant>,
    watched: Option<String>,
    list_scroll: UniformListScrollHandle,
    focus: FocusHandle,
    /// The row the keyboard is on, and whether the list has focus (the ring shows only then).
    cursor: usize,
    list_focused: bool,
    diff_scroll: UniformListScrollHandle,
}

fn pref(key: &str) -> Option<String> {
    mods::shared(key).and_then(|v| v.as_str().map(str::to_owned))
}

impl ChangesView {
    pub(super) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let make = |text: String, window: &mut Window, cx: &mut Context<Self>| {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder(text));
            input.update(cx, |s, _| s.set_editor_style(super::settings_ui::controls::input_style()));
            input
        };
        let input = make(i18n::t("changes.commit_message"), window, cx);
        let branch_input = make(i18n::t("changes.filter_branches"), window, cx);
        cx.subscribe_in(&input, window, |this, input, ev: &InputEvent, _window, cx| match ev {
            InputEvent::Change => {
                this.message = input.read(cx).value().to_string();
                cx.notify();
            }
            InputEvent::PressEnter { .. } => this.commit(cx),
            InputEvent::Focus => {
                this.typing = true;
                cx.notify();
            }
            InputEvent::Blur => {
                this.typing = false;
                cx.notify();
            }
            _ => {}
        })
        .detach();
        cx.subscribe_in(&branch_input, window, |this, input, ev: &InputEvent, _window, cx| {
            if matches!(ev, InputEvent::Change) {
                this.branch_filter = input.read(cx).value().to_lowercase();
                cx.notify();
            }
        })
        .detach();
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_secs(10)).await;
            if this.update(cx, |this, cx| this.refresh_if_seen(cx)).is_err() {
                break;
            }
        })
        .detach();
        Self {
            place: Place::default(), raw: Place::default(), generation: 0, input, branch_input, typing: false, message: String::new(), snap: None, error: None, loading: false, again: false, busy: None, note: None,
            rows: Vec::new(), open: None, closed: false, diff: Vec::new(), diff_width: 300.0, branches: None, branch_filter: String::new(), armed: None, hover: None, base_mode: "head".into(), clear_input: false, clear_branch_input: false, seen: Instant::now(),
            loaded: None, watched: None, list_scroll: UniformListScrollHandle::new(), focus: cx.focus_handle().tab_stop(true), cursor: 0, list_focused: false, diff_scroll: UniformListScrollHandle::new(),
        }
    }

    /// A new place (another pane's folder, another space) starts over.
    pub(super) fn set_place(&mut self, place: Place, cx: &mut Context<Self>) {
        if place == self.raw {
            return;
        }
        self.raw = place.clone();
        let place = place.normalized();
        if place == self.place {
            return;
        }
        self.place = place;
        self.generation += 1;
        self.snap = None;
        self.error = None;
        self.rows.clear();
        self.open = None;
        self.closed = false;
        self.diff.clear();
        self.note = None;
        self.armed = None;
        self.branches = None;
        self.watched = None;
        self.loading = false;
        self.loaded = None;
        self.message.clear();
        self.clear_input = true;
        self.refresh(cx);
        cx.notify();
    }

    fn refresh_if_seen(&mut self, cx: &mut Context<Self>) {
        if self.seen.elapsed() < Duration::from_secs(15) {
            self.refresh(cx);
        }
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let Some(cwd) = self.place.cwd.clone() else { return };
        if self.busy.is_some() {
            return;
        }
        if self.loading {
            self.again = true;
            return;
        }
        self.loading = true;
        let (generation, on) = (self.generation, self.place.on.clone());
        self.base_mode = pref("changes.base").unwrap_or_else(|| "head".into());
        let mode = self.base_mode.clone();
        cx.spawn(async move |this, cx| {
            let loaded = cx.background_executor().spawn(async move { load_status(&cwd, on.as_deref(), &mode) }).await;
            let _ = this.update(cx, |this, cx| {
                if this.generation != generation {
                    return;
                }
                this.loading = false;
                this.loaded = Some(Instant::now());
                match loaded {
                    Ok(snap) => {
                        this.error = None;
                        this.snap = Some(snap);
                        this.watch(cx);
                        this.after_load(cx);
                    }
                    Err(e) => {
                        this.snap = None;
                        this.rows.clear();
                        this.open = None;
                        this.diff.clear();
                        this.error = Some(e);
                    }
                }
                cx.notify();
                if std::mem::take(&mut this.again) {
                    this.refresh(cx);
                }
            });
        })
        .detach();
    }

    /// The list is rebuilt; the pick stays if its file is still there; else the first file's diff shows.
    fn after_load(&mut self, cx: &mut Context<Self>) {
        self.rebuild();
        let Some(snap) = &self.snap else { return };
        let exists = |p: &Pick| match p.section {
            Section::Staged => snap.staged.iter().any(|f| f.path == p.key),
            Section::Changes => snap.changes.iter().any(|f| f.path == p.key),
            Section::Commit => snap.commits.iter().any(|c| c.hash == p.key),
        };
        if self.open.as_ref().is_some_and(|p| !exists(p)) {
            self.open = None;
            self.diff.clear();
        }
        if self.open.is_none() && !self.closed {
            let first = snap.staged.first().map(|f| (Section::Staged, f)).or_else(|| snap.changes.first().map(|f| (Section::Changes, f)));
            self.open = first.map(|(section, f)| Pick { section, key: f.path.clone() });
        }
        if self.open.is_some() {
            self.load_open(cx);
        }
    }

    fn rebuild(&mut self) {
        let Some(snap) = &self.snap else { return };
        let empty = i18n::t("changes.no_uncommitted_changes");
        self.rows = list_rows(snap, group_order(pref("changes.groups").as_deref()), &empty);
    }

    fn load_open(&mut self, cx: &mut Context<Self>) {
        let (Some(snap), Some(pick)) = (self.snap.clone(), self.open.clone()) else { return };
        let (generation, on) = (self.generation, self.place.on.clone());
        cx.spawn(async move |this, cx| {
            let wanted = pick.clone();
            let lines = cx.background_executor().spawn(async move { load_diff(&snap, on.as_deref(), &wanted) }).await;
            let _ = this.update(cx, |this, cx| {
                if this.generation != generation || this.open.as_ref() != Some(&pick) {
                    return;
                }
                let lines = match lines {
                    Ok(l) => l.iter().map(|l| cut_line(l)).collect::<Vec<_>>(),
                    Err(_) => vec![i18n::t("changes.cannot_preview_this_diff_too")],
                };
                this.diff = diff_rows(&lines);
                this.diff_width = diff_width(&this.diff);
                cx.notify();
            });
        })
        .detach();
    }

    fn watch(&mut self, cx: &mut Context<Self>) {
        let Some(root) = self.snap.as_ref().map(|s| s.root.clone()).filter(|_| self.place.on.is_none()) else { return };
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

    /// Arrows move the cursor; Enter or Space shows the diff (or does the section's button on a title);
    /// S stages or unstages the file; Delete or Backspace discards it (press twice, as with the button).
    fn press(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        if let Some(to) = step_cursor(self.cursor, key, self.rows.len()) {
            self.cursor = to;
            self.armed = None;
            self.list_scroll.scroll_to_item(to, gpui::ScrollStrategy::Nearest);
            cx.notify();
            return true;
        }
        let Some(row) = self.rows.get(self.cursor).cloned() else { return false };
        let base = self.snap.as_ref().is_some_and(|s| s.base_ref.is_some());
        match (key, row) {
            ("enter" | "space", ListRow::File { section, change }) => self.pick(Pick { section, key: change.path }, cx),
            ("enter" | "space", ListRow::Commit(commit)) => self.pick(Pick { section: Section::Commit, key: commit.hash }, cx),
            ("enter" | "space", ListRow::Head { section: Section::Staged, .. }) => self.all(true, cx),
            ("enter" | "space", ListRow::Head { section: Section::Changes, count }) if !base && count > 0 => self.all(false, cx),
            ("s", ListRow::File { section: Section::Staged, change }) if !base => self.unstage(&change, cx),
            ("s", ListRow::File { section: Section::Changes, change }) if !base => self.stage(&change, cx),
            ("delete" | "backspace", ListRow::File { section: Section::Changes, change }) if !base => self.discard(&change, cx),
            _ => return false,
        }
        true
    }

    fn pick(&mut self, pick: Pick, cx: &mut Context<Self>) {
        self.armed = None;
        self.note = None;
        if self.open.as_ref() == Some(&pick) {
            self.open = None;
            self.closed = true;
            self.diff.clear();
        } else {
            self.open = Some(pick);
            self.closed = false;
            self.diff.clear();
            self.load_open(cx);
        }
        cx.notify();
    }

    /// Runs git commands one after another from the repository root, then reads it again.
    /// A failure shows git's own last lines. `secs` bounds each command (fetch, pull and push take longer).
    fn git_steps(&mut self, steps: Vec<Vec<String>>, secs: u64, clear_message: bool, cx: &mut Context<Self>) {
        let Some(root) = self.snap.as_ref().map(|s| s.root.clone()) else { return };
        if self.busy.is_some() {
            return;
        }
        let label = steps.last().map(|s| format!("git {}", s.join(" "))).unwrap_or_default();
        self.busy = Some(steps.last().and_then(|s| s.first().cloned()).unwrap_or_default());
        self.note = None;
        self.armed = None;
        let (generation, on) = (self.generation, self.place.on.clone());
        cx.spawn(async move |this, cx| {
            let failure = cx
                .background_executor()
                .spawn(async move {
                    for step in &steps {
                        let args: Vec<&str> = step.iter().map(String::as_str).collect();
                        match exec(on.as_deref(), "git", &args, &root, secs) {
                            Ok((0, _, _)) => {}
                            Ok((_, out, err)) => return Some(tail(if err.trim().is_empty() { &out } else { &err })),
                            Err(e) => return Some(tail(&e)),
                        }
                    }
                    None
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.generation != generation {
                    return;
                }
                this.busy = None;
                match failure {
                    Some(why) => this.note = Some(format!("{}\n{why}", i18n::tf("changes.git_failed", &[("command", &label)]))),
                    None if clear_message => {
                        this.message.clear();
                        this.clear_input = true;
                    }
                    None => {}
                }
                this.refresh(cx);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn commit(&mut self, cx: &mut Context<Self>) {
        let Some(snap) = self.snap.clone() else { return };
        if self.loading || !can_commit(&self.message, Some(&snap), self.busy.is_some()) {
            return;
        }
        let steps = commit_steps(&snap, &self.message);
        self.git_steps(steps, 30, true, cx);
    }

    fn open_branches(&mut self, cx: &mut Context<Self>) {
        let Some(snap) = &self.snap else { return };
        if self.branches.is_some() {
            self.branches = None;
            cx.notify();
            return;
        }
        self.branches = Some(Vec::new());
        self.branch_filter.clear();
        self.clear_branch_input = true;
        let (root, on, generation) = (snap.root.clone(), self.place.on.clone(), self.generation);
        cx.spawn(async move |this, cx| {
            let list = cx
                .background_executor()
                .spawn(async move { Git { root: &root, on: on.as_deref() }.ok(&["branch", "--format=%(refname:short)", "--sort=-committerdate"]).map(|o| o.lines().filter(|l| !l.is_empty()).map(str::to_owned).collect::<Vec<_>>()).unwrap_or_default() })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.generation == generation && this.branches.is_some() {
                    this.branches = Some(list);
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    fn stage(&mut self, change: &Change, cx: &mut Context<Self>) {
        self.git_steps(vec![vec!["add".into(), "--".into(), change.path.clone()]], 30, false, cx);
    }

    fn unstage(&mut self, change: &Change, cx: &mut Context<Self>) {
        let has_head = self.snap.as_ref().is_some_and(|s| s.has_head);
        self.git_steps(vec![unstage_args(has_head, &change.path)], 30, false, cx);
    }

    /// Discarding loses work: the first click arms the row's button, the second does it.
    fn discard(&mut self, change: &Change, cx: &mut Context<Self>) {
        let pick = Pick { section: Section::Changes, key: change.path.clone() };
        if self.armed.as_ref() != Some(&pick) {
            self.armed = Some(pick);
            cx.notify();
            return;
        }
        self.git_steps(vec![discard_args(change)], 30, false, cx);
    }

    fn all(&mut self, staged: bool, cx: &mut Context<Self>) {
        let args: Vec<String> = if staged { vec!["reset".into(), "-q".into()] } else { vec!["add".into(), "-A".into()] };
        self.git_steps(vec![args], 30, false, cx);
    }

    fn remote(&mut self, args: &[&str], cx: &mut Context<Self>) {
        self.git_steps(vec![args.iter().map(|s| (*s).to_owned()).collect()], 120, false, cx);
    }

    fn push(&mut self, cx: &mut Context<Self>) {
        let Some(snap) = &self.snap else { return };
        let args = push_args(snap.upstream, snap.branch.as_deref());
        self.git_steps(vec![args], 120, false, cx);
    }
}

// Drawing ---------------------------------------------------------------------------------------

fn status_color(status: char) -> Hsla {
    tone(match status {
        'A' => "ansi2",
        'D' | 'U' => "ansi1",
        _ => "blocked",
    })
}

impl ChangesView {
    fn text_button(&self, id: impl Into<ElementId>, label: String, on: bool, cx: &mut Context<Self>, act: impl Fn(&mut Self, &mut Context<Self>) + 'static) -> Stateful<Div> {
        let ring = c("accent");
        let base = div()
            .id(id)
            .h(z(ROW_H))
            .px(z(10.0))
            .flex()
            .flex_none()
            .items_center()
            .rounded(r(6.0))
            .border_1()
            .border_color(c("border"))
            .text_size(z(12.0))
            .text_color(c(if on { "foreground" } else { "muted" }));
        let el = if on {
            let app = cx.entity();
            keyed(base.cursor_pointer().hover(|s| s.bg(c("hover"))), ring, move |_, cx| app.update(cx, |this, cx| act(this, cx)))
        } else {
            base
        };
        el.child(label)
    }

    /// A 32 px square with an icon, for the actions on a row.
    fn icon_button(&self, id: impl Into<ElementId>, bytes: &'static [u8], label: String, cx: &mut Context<Self>, act: impl Fn(&mut Self, &mut Context<Self>) + 'static) -> Stateful<Div> {
        let ring = c("accent");
        let app = cx.entity();
        keyed(
            div()
                .id(id)
                .size(z(ROW_H))
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .rounded(r(6.0))
                .border_2()
                .border_color(Hsla { a: 0.0, ..ring })
                .cursor_pointer()
                .hover(|s| s.bg(c("raised")))
                .tooltip(tip(label))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()),
            ring,
            move |_, cx| app.update(cx, |this, cx| act(this, cx)),
        )
        .child(icon(bytes, 14.0, graphic("muted")))
    }

    fn head_row(&self, section: Section, count: usize, on_cursor: bool, cx: &mut Context<Self>) -> AnyElement {
        let title = match section {
            Section::Staged => i18n::t("rs.changes.staged"),
            Section::Changes => i18n::t("changes.changes"),
            Section::Commit => i18n::t("rs.changes.commits"),
        };
        let base = self.snap.as_ref().is_some_and(|s| s.base_ref.is_some());
        let idle = self.busy.is_none();
        let action = match section {
            Section::Staged => Some((i18n::t("rs.changes.unstage_all"), true)),
            Section::Changes if !base && count > 0 => Some((i18n::t("rs.changes.stage_all"), false)),
            _ => None,
        };
        ring(div().id(SharedString::from(format!("head-{section:?}"))), on_cursor)
            .h(z(ROW_H))
            .w_full()
            .flex_none()
            .flex()
            .items_center()
            .px(z(8.0))
            .gap(z(8.0))
            .rounded(r(7.0))
            .child(div().flex_1().min_w(px(0.0)).truncate().text_size(z(11.0)).font_weight(FontWeight::SEMIBOLD).text_color(tone("muted")).child(format!("{} {count}", title.to_uppercase())))
            .when_some(action, |d, (label, staged)| {
                d.child(
                    div()
                        .id(SharedString::from(format!("all-{staged}")))
                        .h(z(ROW_H))
                        .px(z(8.0))
                        .flex()
                        .items_center()
                        .rounded(r(6.0))
                        .text_size(z(12.0))
                        .text_color(if idle { c("foreground") } else { tone("muted") })
                        .when(idle, |d| d.cursor_pointer().hover(|s| s.bg(c("hover"))))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.all(staged, cx)))
                        .child(label),
                )
            })
            .into_any_element()
    }

    fn file_row(&self, section: Section, change: &Change, cursor_at: usize, on_cursor: bool, cx: &mut Context<Self>) -> AnyElement {
        let pick = Pick { section, key: change.path.clone() };
        let selected = self.open.as_ref() == Some(&pick);
        let armed = self.armed.as_ref() == Some(&pick);
        let base = self.snap.as_ref().is_some_and(|s| s.base_ref.is_some());
        let (dir, name) = match change.path.rsplit_once('/') {
            Some((d, n)) => (format!("{d}/"), n.to_owned()),
            None => (String::new(), change.path.clone()),
        };
        let id = SharedString::from(format!("change-{section:?}-{}", change.path));
        let (for_click, for_stage, for_discard) = (pick.clone(), change.clone(), change.clone());
        let hovered = self.hover.as_ref() == Some(&pick);
        let show_actions = !base && (hovered || armed || on_cursor);
        let for_hover = pick.clone();
        let actions = div().flex_none().flex().items_center().when(show_actions, |d| match section {
            Section::Staged => d.child(self.icon_button(SharedString::from(format!("unstage-{}", change.path)), MINUS, i18n::t("rs.changes.unstage"), cx, move |this, cx| this.unstage(&for_stage, cx))),
            _ => d
                .child(if armed {
                    div()
                        .id(SharedString::from(format!("discard-{}", for_discard.path)))
                        .h(z(ROW_H))
                        .px(z(8.0))
                        .flex()
                        .items_center()
                        .rounded(r(6.0))
                        .cursor_pointer()
                        .text_size(z(12.0))
                        .text_color(tone("ansi1"))
                        .hover(|s| s.bg(c("raised")))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(cx.listener({
                            let change = for_discard.clone();
                            move |this, _: &ClickEvent, _, cx| this.discard(&change, cx)
                        }))
                        .child(i18n::t("rs.changes.discard_confirm"))
                        .into_any_element()
                } else {
                    self.icon_button(SharedString::from(format!("discard-{}", for_discard.path)), UNDO, i18n::t("rs.changes.discard"), cx, {
                        let change = for_discard.clone();
                        move |this, cx| this.discard(&change, cx)
                    })
                    .into_any_element()
                })
                .when(!armed, |d| d.child(self.icon_button(SharedString::from(format!("stage-{}", change.path)), PLUS, i18n::t("rs.changes.stage"), cx, move |this, cx| this.stage(&for_stage, cx)))),
        });
        ring(div().id(id), on_cursor)
            .h(z(ROW_H))
            .w_full()
            .flex_none()
            .flex()
            .items_center()
            .pl(z(8.0))
            .pr(z(4.0))
            .gap(z(7.0))
            .rounded(r(7.0))
            .cursor_pointer()
            .text_size(z(13.0))
            .when(selected, |d| d.bg(c("selected")))
            .hover(|s| s.bg(c("hover")))
            .on_hover(cx.listener(move |this, over: &bool, _, cx| {
                if *over {
                    this.hover = Some(for_hover.clone());
                } else if this.hover.as_ref() == Some(&for_hover) {
                    this.hover = None;
                }
                cx.notify();
            }))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.cursor = cursor_at;
                this.focus.focus(window, cx);
                this.pick(for_click.clone(), cx)
            }))
            .child(icon(FILE, 14.0, graphic("muted")))
            .child(div().id(SharedString::from(format!("change-name-{}", change.path))).flex_none().max_w(relative(0.6)).truncate().text_color(c("foreground")).tooltip(tip(name.to_string())).child(name))
            .child(div().id(SharedString::from(format!("change-dir-{}", change.path))).flex_1().min_w(px(0.0)).truncate().text_size(z(12.0)).text_color(tone("muted")).tooltip(tip(dir.to_string())).child(dir))
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(z(6.0))
                    .when(show_actions, |d| d.hidden())
                    .when(change.add > 0, |d| d.child(div().text_size(z(12.0)).text_color(tone("ansi2")).child(format!("+{}", change.add))))
                    .when(change.del > 0, |d| d.child(div().text_size(z(12.0)).text_color(tone("ansi1")).child(format!("\u{2212}{}", change.del))))
                    .child(div().w(z(14.0)).text_size(z(12.0)).font_weight(FontWeight::SEMIBOLD).text_color(status_color(change.status)).child(change.status.to_string())),
            )
            .child(actions)
            .into_any_element()
    }

    fn commit_row(&self, commit: &Commit, cursor_at: usize, on_cursor: bool, cx: &mut Context<Self>) -> AnyElement {
        let pick = Pick { section: Section::Commit, key: commit.hash.clone() };
        let selected = self.open.as_ref() == Some(&pick);
        ring(div().id(SharedString::from(format!("commit-{}", commit.hash))), on_cursor)
            .h(z(ROW_H))
            .w_full()
            .flex_none()
            .flex()
            .items_center()
            .px(z(8.0))
            .gap(z(8.0))
            .rounded(r(7.0))
            .cursor_pointer()
            .text_size(z(13.0))
            .when(selected, |d| d.bg(c("selected")))
            .hover(|s| s.bg(c("hover")))
            .tooltip(tip(format!("{} \u{b7} {}", commit.author, commit.ago)))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.cursor = cursor_at;
                this.focus.focus(window, cx);
                this.pick(pick.clone(), cx)
            }))
            .child(div().flex_none().text_size(z(12.0)).text_color(tone("muted")).child(commit.hash.clone()))
            .child(div().flex_1().min_w(px(0.0)).truncate().child(commit.subject.clone()))
            .child(div().flex_none().text_size(z(12.0)).text_color(tone("muted")).child(commit.ago.clone()))
            .into_any_element()
    }

    fn list_row(&self, i: usize, cx: &mut Context<Self>) -> AnyElement {
        let on = self.list_focused && self.cursor == i;
        match self.rows.get(i).cloned() {
            Some(ListRow::Head { section, count }) => self.head_row(section, count, on, cx),
            Some(ListRow::Group { status, count }) => div()
                .h(z(ROW_H))
                .px(z(16.0))
                .flex()
                .items_center()
                .text_size(z(12.0))
                .text_color(tone("muted"))
                .child(format!("{} \u{b7} {count}", i18n::t(&format!("changes.group_{status}"))))
                .into_any_element(),
            Some(ListRow::File { section, change }) => self.file_row(section, &change, i, on, cx),
            Some(ListRow::Commit(commit)) => self.commit_row(&commit, i, on, cx),
            Some(ListRow::Note(text)) => div().h(z(ROW_H)).px(z(8.0)).flex().items_center().text_size(z(12.0)).text_color(tone("muted")).child(text).into_any_element(),
            None => div().into_any_element(),
        }
    }

    fn diff_row(&self, i: usize) -> AnyElement {
        let Some(row) = self.diff.get(i) else { return div().into_any_element() };
        let tint = |name: &str| Hsla { a: 0.14, ..c(name) };
        let base = div().w_full().h(z(DIFF_ROW_H)).flex_none().flex().items_center().whitespace_nowrap();
        match row.kind {
            '@' => base.px(z(12.0)).bg(c("raised")).text_color(tone("muted")).child(row.text.clone()).into_any_element(),
            'm' => base.px(z(12.0)).text_color(tone("muted")).child(row.text.clone()).into_any_element(),
            kind => base
                .gap(z(10.0))
                .when(kind == '+', |d| d.bg(tint("ansi2")))
                .when(kind == '-', |d| d.bg(tint("ansi1")))
                .child(div().w(z(36.0)).flex_none().text_right().text_color(tone("muted")).child(row.number.map(|n| n.to_string()).unwrap_or_default()))
                .child(div().flex_1().child(row.text.clone()))
                .into_any_element(),
        }
    }

    fn git_bar(&self, cx: &mut Context<Self>) -> Div {
        let ready = self.snap.is_some();
        let idle = ready && self.busy.is_none();
        let snap = self.snap.clone().unwrap_or_default();
        let branch = snap.branch.clone().unwrap_or_else(|| if ready { "HEAD".into() } else { "\u{2013}".into() });
        let counts = if snap.upstream { format!("\u{2191}{} \u{2193}{}", snap.ahead, snap.behind) } else if ready { i18n::t("changes.no_upstream") } else { String::new() };
        let spin = self.busy.clone();
        div()
            .flex_none()
            .px(z(10.0))
            .gap(z(6.0))
            .flex()
            .items_center()
            .child(
                div()
                    .id("git-branch")
                    .h(z(ROW_H))
                    .max_w(z(150.0))
                    .px(z(8.0))
                    .gap(z(6.0))
                    .flex()
                    .items_center()
                    .rounded(r(6.0))
                    .border_1()
                    .border_color(c(if self.branches.is_some() { "accent" } else { "border" }))
                    .text_size(z(12.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(c("hover")))
                    .tooltip(tip(i18n::t("changes.switch_branch")))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.open_branches(cx)))
                    .child(icon(BRANCH, 13.0, graphic("muted")))
                    .child(div().min_w(px(0.0)).truncate().child(branch)),
            )
            .child(div().flex_1().min_w(px(0.0)).truncate().text_size(z(12.0)).text_color(tone("muted")).child(counts))
            .when_some(spin, |d, op| d.child(div().flex_none().text_size(z(12.0)).text_color(tone("muted")).child(format!("{op}\u{2026}"))))
            .child(self.text_button("git-fetch", i18n::t("changes.fetch"), idle, cx, |this, cx| this.remote(&["fetch", "--prune"], cx)))
            .child(self.text_button("git-pull", i18n::t("changes.pull"), idle, cx, |this, cx| this.remote(&["pull", "--ff-only"], cx)))
            .child(self.text_button("git-push", i18n::t("changes.push"), idle, cx, |this, cx| this.push(cx)))
    }

    fn branch_list(&self, cx: &mut Context<Self>) -> Div {
        let current = self.snap.as_ref().and_then(|s| s.branch.clone());
        let list: Vec<String> = self.branches.clone().unwrap_or_default().into_iter().filter(|b| self.branch_filter.is_empty() || b.to_lowercase().contains(&self.branch_filter)).collect();
        div()
            .flex_none()
            .max_h(z(240.0))
            .flex()
            .flex_col()
            .p(z(8.0))
            .gap(z(6.0))
            .bg(c("surface"))
            .child(div().h(z(ROW_H)).px(z(8.0)).flex().items_center().rounded(r(6.0)).border_1().border_color(c("border")).text_size(z(12.0)).child(Input::new(&self.branch_input)))
            .child(div().id("git-branches").flex().flex_col().overflow_y_scroll().children(list.into_iter().enumerate().map(|(i, b)| {
                let name = b.clone();
                div()
                    .id(("git-b", i))
                    .h(z(ROW_H))
                    .px(z(8.0))
                    .flex()
                    .items_center()
                    .flex_none()
                    .rounded(r(6.0))
                    .text_size(z(12.0))
                    .cursor_pointer()
                    .when(Some(&b) == current.as_ref(), |d| d.text_color(c("accent")))
                    .hover(|s| s.bg(c("hover")))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.branches = None;
                        this.git_steps(vec![vec!["switch".into(), name.clone()]], 120, false, cx);
                    }))
                    .child(b)
            })))
    }
}

impl Render for ChangesView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.seen = Instant::now();
        self.list_focused = self.focus.is_focused(window);
        self.cursor = self.cursor.min(self.rows.len().saturating_sub(1));
        if std::mem::take(&mut self.clear_input) {
            self.input.update(cx, |s, cx| s.set_value("", window, cx));
        }
        if std::mem::take(&mut self.clear_branch_input) {
            self.branch_input.update(cx, |s, cx| s.set_value("", window, cx));
        }
        // Settings > Git changed (the base, the groups), or the list is old: read again.
        let base = pref("changes.base").unwrap_or_else(|| "head".into());
        if base != self.base_mode && self.snap.is_some() || self.loaded.is_some_and(|at| at.elapsed() > Duration::from_secs(10)) {
            self.refresh(cx);
        }
        let groups = pref("changes.groups");
        if self.snap.is_some() && self.rows.iter().any(|r| matches!(r, ListRow::Group { .. })) != group_order(groups.as_deref()).is_some() {
            self.rebuild();
        }
        let snap = self.snap.clone();
        let files = snap.as_ref().map_or(0, |s| s.changes.len() + s.staged.iter().filter(|f| !s.changes.iter().any(|c| c.path == f.path)).count());
        let enabled = can_commit(&self.message, snap.as_ref(), self.busy.is_some()) && !self.loading;
        let empty = match (&self.error, &self.place.cwd, &snap) {
            (Some(Failure::Git(e)), _, _) => Some((e.clone(), true)),
            (Some(Failure::NotRepo(e)), _, _) => Some((e.clone(), false)),
            (None, None, _) => Some((i18n::t("welcome.empty_changes"), false)),
            (None, Some(_), None) => Some((i18n::t("changes.loading_changes"), false)),
            _ => None,
        };
        let has_open = self.open.is_some() && snap.is_some();
        let count = self.rows.len();
        let diff_count = self.diff.len();
        let width = self.diff_width;
        let font = settings::get().font_family.clone();
        let committing = self.busy.as_deref() == Some("commit");
        div()
            .id("changes")
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
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(i18n::t("changes.changes")))
                    .child(div().px(z(5.0)).py(z(1.0)).rounded(r(4.0)).border_1().border_color(c("border")).text_size(z(11.0)).text_color(tone("muted")).child(i18n::tn("changes.n_files", files, &[("n", &files.to_string())])))
                    .when_some(snap.as_ref().and_then(|s| s.base_ref.clone()), |d, b| d.child(div().id("changes-base").max_w(z(110.0)).truncate().text_size(z(11.0)).text_color(tone("muted")).child(i18n::tf("changes.vs_base", &[("ref", &b)]))))
                    .child(div().flex_1())
                    .child(div().text_size(z(12.0)).text_color(tone("ansi2")).child(format!("+{}", snap.as_ref().map_or(0, |s| s.add))))
                    .child(div().text_size(z(12.0)).text_color(tone("ansi1")).child(format!("\u{2212}{}", snap.as_ref().map_or(0, |s| s.del)))),
            )
            .child(fade(8.0).my(z(0.0)))
            .child(self.git_bar(cx).py(z(6.0)))
            .when(self.branches.is_some(), |d| d.child(self.branch_list(cx)))
            .child(fade(8.0).my(z(0.0)))
            .child(
                div()
                    .flex_none()
                    .p(z(10.0))
                    .gap(z(8.0))
                    .flex()
                    .child(
                        div()
                            .id("commit-message")
                            .flex_1()
                            .min_w(px(0.0))
                            .h(z(ROW_H))
                            .px(z(10.0))
                            .flex()
                            .items_center()
                            .rounded(r(8.0))
                            .border_1()
                            .border_color(c(if self.typing { "accent" } else { "border" }))
                            .cursor_text()
                            .text_size(z(12.0))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(Input::new(&self.input)),
                    )
                    .child(
                        div()
                            .id("commit")
                            .h(z(ROW_H))
                            .px(z(14.0))
                            .flex()
                            .items_center()
                            .rounded(r(8.0))
                            .bg(c(if enabled { "solid" } else { "raised" }))
                            .text_color(c(if enabled { "solid_foreground" } else { "muted" }))
                            .text_size(z(12.0))
                            .when(enabled, |d| d.cursor_pointer())
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.commit(cx)))
                            .child(if committing { i18n::t("changes.committing") } else { i18n::t("changes.commit") }),
                    ),
            )
            .when_some(self.note.clone(), |d, note| d.child(div().px(z(12.0)).pb(z(8.0)).text_size(z(12.0)).text_color(tone("ansi1")).child(note)))
            .when_some(empty, |d, (text, error)| {
                d.child(div().px(z(14.0)).py(z(8.0)).flex().flex_col().items_start().gap(z(8.0)).text_size(z(12.0)).text_color(tone(if error { "ansi1" } else { "muted" })).child(text)
                    .when(error, |d| d.child(self.text_button("changes-retry", i18n::t("rs.setup.retry"), true, cx, |this, cx| this.refresh(cx)))))
            })
            .child(fade(8.0).my(z(0.0)))
            .when(snap.is_some(), |d| {
                d.child(
                    div().id("changes-list-focus").track_focus(&self.focus).flex_1().min_h(z(96.0)).p(z(6.0))
                    .on_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
                        if this.press(e.keystroke.key.as_str(), cx) {
                            cx.stop_propagation();
                        }
                    })).child(
                        uniform_list("changes-list", count, cx.processor(|this, range: std::ops::Range<usize>, _window, cx| range.map(|i| this.list_row(i, cx)).collect::<Vec<_>>()))
                            .track_scroll(&self.list_scroll)
                            .size_full(),
                    ),
                )
            })
            .when(snap.as_ref().is_some_and(|s| s.limited), |d| d.child(div().px(z(12.0)).py(z(4.0)).text_size(z(12.0)).text_color(tone("muted")).child(i18n::t("changes.showing_the_first_1_000_changed"))))
            .when(has_open, |d| {
                d.child(fade(8.0).my(z(0.0))).child(
                    div().flex_1().min_h(px(0.0)).relative().font_family(font).text_size(z(12.0)).child(
                        div().id("changes-diff-x").size_full().overflow_x_scroll().child(
                            div().min_w(px(width)).w_full().h_full().child(
                                uniform_list("changes-diff", diff_count, cx.processor(|this, range: std::ops::Range<usize>, _window, _cx| range.map(|i| this.diff_row(i)).collect::<Vec<_>>()))
                                    .track_scroll(&self.diff_scroll)
                                    .size_full(),
                            ),
                        ),
                    ),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Change, Commit, DiffRow, Failure, ListRow, Section, Snapshot, base_files, can_commit, classify_rev_parse, commit_steps, diff_rows, diff_width, discard_args, group_order, list_rows, load_status, merge_base,
        parse_commits, parse_counts, parse_numstat, parse_status, patch_lines, push_args, split_status, unstage_args, untab, Git,
    };
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    fn git(dir: &Path, args: &[&str]) {
        let out = std::process::Command::new("git").args(["-c", "user.email=t@t.t", "-c", "user.name=t", "-c", "commit.gpgsign=false"]).args(args).current_dir(dir).output().unwrap();
        assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    }

    /// A repository with two commits on main, under a folder of its own.
    fn repo(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sunna-changes-{name}-{}", std::process::id())).canonicalize().unwrap_or_else(|_| std::env::temp_dir().join(format!("sunna-changes-{name}-{}", std::process::id())));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q", "-b", "main"]);
        std::fs::write(dir.join("a.txt"), "uno\n").unwrap();
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "-qm", "primero"]);
        std::fs::write(dir.join("b.txt"), "dos\n").unwrap();
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "-qm", "segundo"]);
        dir.canonicalize().unwrap()
    }

    fn change(path: &str, status: char) -> Change {
        Change { path: path.into(), status, ..Change::default() }
    }

    #[test]
    fn tabs_in_a_diff_pad_to_the_next_4_column_stop() {
        let rows = diff_rows(&["@@ -1,2 +1,2 @@", "-\tif x {", "+\t\tif x {", " a\tb", "+    spaces"].map(String::from));
        assert_eq!(rows.iter().map(|r| r.text.as_str()).collect::<Vec<_>>(), ["@@ -1,2 +1,2 @@", "    if x {", "        if x {", "a   b", "    spaces"]);
        assert_eq!(untab("\tx"), "    x");
        assert_eq!(untab("ab\tx"), "ab  x");
    }

    #[test]
    fn diff_lines_are_numbered_from_their_hunk() {
        let rows = diff_rows(&["@@ -10,2 +20,3 @@", " ctx", "-old", "+new", "+more"].map(String::from));
        let numbers: Vec<_> = rows.iter().map(|r| (r.kind, r.number)).collect();
        assert_eq!(numbers, [('@', None), (' ', Some(20)), ('-', Some(11)), ('+', Some(21)), ('+', Some(22))]);
        assert!(diff_width(&rows) >= 300.0);
        assert_eq!(diff_rows(&["diff --git a/x b/x".to_owned()]), vec![DiffRow { kind: 'm', number: None, text: "diff --git a/x b/x".into() }]);
    }

    #[test]
    fn a_patch_loses_git_header_lines_and_keeps_file_headers() {
        let text = "diff --git a/x b/x\nindex 1..2 100644\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(patch_lines(text), ["diff --git a/x b/x", "@@ -1 +1 @@", "-a", "+b"]);
    }

    #[test]
    fn the_bar_reads_how_far_the_branch_is_from_its_upstream() {
        assert_eq!(parse_counts("2\t1\n"), (2, 1));
        assert_eq!(parse_counts(""), (0, 0));
    }

    #[test]
    fn push_without_an_upstream_sets_one_on_origin() {
        assert_eq!(push_args(false, Some("feature")), ["push", "-u", "origin", "feature"]);
        assert_eq!(push_args(false, None), ["push", "-u", "origin", "HEAD"]);
        assert_eq!(push_args(true, Some("feature")), ["push"]);
    }

    #[test]
    fn the_commit_list_reads_git_log_lines() {
        let commits = parse_commits("abc123\x1fprimero\x1fAna\x1f2 days ago\ndef456\x1fsegundo\x1fLuis\x1f1 day ago\n");
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[1], Commit { hash: "def456".into(), subject: "segundo".into(), author: "Luis".into(), ago: "1 day ago".into() });
    }

    #[test]
    fn only_gits_not_a_repository_gets_the_short_text() {
        assert_eq!(classify_rev_parse("fatal: cannot change to '/repo': Operation not permitted\n"), Failure::Git("fatal: cannot change to '/repo': Operation not permitted".into()));
        assert!(matches!(classify_rev_parse("fatal: not a git repository (or any of the parent directories): .git\n"), Failure::NotRepo(_)));
        assert!(matches!(classify_rev_parse(""), Failure::NotRepo(_)));
    }

    #[test]
    fn status_splits_into_staged_and_changes_and_a_conflict_is_only_in_changes() {
        let entries = parse_status("M  s.rs\0 M w.rs\0MM both.rs\0A  new.rs\0?? fresh.md\0 D gone.rs\0UU clash.rs\0");
        let mut stats = HashMap::new();
        parse_numstat("3\t1\tw.rs\0-\t-\tbin.png\0", &mut stats);
        let (staged, changes) = split_status(&entries, &stats, &HashMap::new());
        assert_eq!(staged.iter().map(|f| (f.path.as_str(), f.status)).collect::<Vec<_>>(), [("s.rs", 'M'), ("both.rs", 'M'), ("new.rs", 'A')]);
        assert_eq!(changes.iter().map(|f| (f.path.as_str(), f.status, f.fresh)).collect::<Vec<_>>(), [("w.rs", 'M', false), ("both.rs", 'M', false), ("fresh.md", 'A', true), ("gone.rs", 'D', false), ("clash.rs", 'U', false)]);
        assert_eq!((changes[0].add, changes[0].del), (3, 1));
        assert!(stats["bin.png"].2);
    }

    #[test]
    fn against_a_base_the_list_is_what_differs_and_new_files_come_from_status() {
        let entries = parse_status("?? fresh.md\0 M a.rs\0");
        let files = base_files("M\0a.rs\0A\0b.rs\0D\0c.rs\0", &entries, &HashMap::new());
        assert_eq!(files.iter().map(|f| (f.path.as_str(), f.status, f.fresh)).collect::<Vec<_>>(), [("a.rs", 'M', false), ("b.rs", 'A', false), ("c.rs", 'D', false), ("fresh.md", 'A', true)]);
    }

    #[test]
    fn the_groups_order_the_files_and_the_staged_ones_come_first() {
        let snap = Snapshot { staged: vec![change("s", 'M')], changes: vec![change("a", 'M'), change("b", 'A'), change("c", 'D'), change("d", 'M')], commits: vec![Commit { hash: "h".into(), ..Commit::default() }], ..Snapshot::default() };
        let names = |order| {
            list_rows(&snap, group_order(order), "none")
                .into_iter()
                .map(|r| match r {
                    ListRow::Head { section, count } => format!("{section:?}{count}"),
                    ListRow::Group { status, count } => format!("{status}{count}"),
                    ListRow::File { change, .. } => change.path,
                    ListRow::Commit(c) => c.hash,
                    ListRow::Note(n) => n,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(names(None), ["Staged1", "s", "Changes4", "a", "b", "c", "d", "Commit1", "h"]);
        assert_eq!(names(Some("status")), ["Staged1", "s", "Changes4", "M2", "a", "d", "A1", "b", "D1", "c", "Commit1", "h"]);
        assert_eq!(names(Some("new_first")), ["Staged1", "s", "Changes4", "A1", "b", "M2", "a", "d", "D1", "c", "Commit1", "h"]);
        let empty = list_rows(&Snapshot::default(), None, "none");
        assert_eq!(empty, vec![ListRow::Head { section: Section::Changes, count: 0 }, ListRow::Note("none".into())]);
    }

    #[test]
    fn commit_needs_a_message_something_to_commit_and_no_base() {
        let with = Snapshot { changes: vec![change("a", 'M')], ..Snapshot::default() };
        assert!(can_commit(" fix ", Some(&with), false));
        assert!(!can_commit("  ", Some(&with), false));
        assert!(!can_commit("fix", Some(&with), true));
        assert!(!can_commit("fix", Some(&Snapshot::default()), false));
        assert!(!can_commit("fix", None, false));
        assert!(!can_commit("fix", Some(&Snapshot { base_ref: Some("main".into()), ..with.clone() }), false));
    }

    #[test]
    fn commit_stages_everything_only_when_nothing_is_staged() {
        let none = Snapshot { changes: vec![change("a", 'M')], ..Snapshot::default() };
        assert_eq!(commit_steps(&none, " msg "), vec![vec!["add", "-A"], vec!["commit", "-m", "msg"]]);
        let some = Snapshot { staged: vec![change("a", 'M')], ..none };
        assert_eq!(commit_steps(&some, "msg"), vec![vec!["commit", "-m", "msg"]]);
    }

    #[test]
    fn discard_and_unstage_use_the_command_that_fits_the_file() {
        assert_eq!(discard_args(&Change { fresh: true, ..change("n.md", 'A') }), ["clean", "-f", "--", "n.md"]);
        assert_eq!(discard_args(&change("a.rs", 'M')), ["restore", "--", "a.rs"]);
        assert_eq!(unstage_args(true, "a.rs"), ["restore", "--staged", "--", "a.rs"]);
        assert_eq!(unstage_args(false, "a.rs"), ["rm", "--cached", "-q", "--", "a.rs"]);
    }

    #[test]
    fn a_real_repository_reads_staged_changed_new_files_and_its_commits() {
        let dir = repo("status");
        std::fs::write(dir.join("a.txt"), "uno\ndos\n").unwrap();
        std::fs::write(dir.join("b.txt"), "cambiado\n").unwrap();
        git(&dir, &["add", "b.txt"]);
        std::fs::write(dir.join("nuevo.md"), "x\n").unwrap();
        let snap = load_status(dir.to_str().unwrap(), None, "head").unwrap();
        assert_eq!(snap.root, dir.to_str().unwrap());
        assert_eq!(snap.branch.as_deref(), Some("main"));
        assert!(snap.has_head && !snap.upstream);
        assert_eq!(snap.staged.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), ["b.txt"]);
        assert_eq!(snap.changes.iter().map(|f| (f.path.as_str(), f.status)).collect::<Vec<_>>(), [("a.txt", 'M'), ("nuevo.md", 'A')]);
        assert_eq!(snap.changes[0].add, 1);
        // main is the default branch here: nothing "on the branch" is newer than it, so the latest commits show.
        assert_eq!(snap.commits.iter().map(|c| c.subject.as_str()).collect::<Vec<_>>(), Vec::<&str>::new());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_folder_that_is_no_repository_says_so_and_a_new_repository_has_no_head() {
        let plain = std::env::temp_dir().join(format!("sunna-changes-plain-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&plain);
        std::fs::create_dir_all(&plain).unwrap();
        assert!(matches!(load_status(plain.to_str().unwrap(), None, "head"), Err(Failure::NotRepo(_))));
        git(&plain, &["init", "-q", "-b", "main"]);
        std::fs::write(plain.join("x.txt"), "x\n").unwrap();
        let snap = load_status(plain.canonicalize().unwrap().to_str().unwrap(), None, "head").unwrap();
        assert!(!snap.has_head && snap.commits.is_empty());
        assert_eq!(snap.changes.len(), 1);
        let _ = std::fs::remove_dir_all(&plain);
    }

    #[test]
    fn a_feature_branch_lists_its_commits_and_a_compare_base_picks_the_merge_base() {
        let dir = repo("base");
        let sha = |rev: &str| String::from_utf8(std::process::Command::new("git").args(["rev-parse", rev]).current_dir(&dir).output().unwrap().stdout).unwrap().trim().to_owned();
        git(&dir, &["switch", "-qc", "feat"]);
        std::fs::write(dir.join("c.txt"), "tres\n").unwrap();
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "-qm", "tercero"]);
        let main_sha = sha("main");
        // origin/trunk is the repository's default branch, main its upstream's twin.
        git(&dir, &["remote", "add", "origin", "/nonexistent"]);
        git(&dir, &["update-ref", "refs/remotes/origin/trunk", &main_sha]);
        git(&dir, &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/trunk"]);
        git(&dir, &["branch", "--set-upstream-to=origin/trunk"]);
        let g = Git { root: dir.to_str().unwrap(), on: None };
        assert_eq!(merge_base(&g, "default"), Some(("origin/trunk".to_owned(), main_sha.clone())));
        assert_eq!(merge_base(&g, "upstream"), Some(("upstream".to_owned(), main_sha.clone())));
        let snap = load_status(dir.to_str().unwrap(), None, "default").unwrap();
        assert_eq!(snap.base_ref.as_deref(), Some("origin/trunk"));
        assert_eq!(snap.diff_base, main_sha);
        assert_eq!(snap.changes.iter().map(|f| (f.path.as_str(), f.status)).collect::<Vec<_>>(), [("c.txt", 'A')]);
        assert_eq!(snap.commits.iter().map(|c| c.subject.as_str()).collect::<Vec<_>>(), ["tercero"]);
        assert!(snap.upstream && snap.ahead == 1);
        let none = std::env::temp_dir().join(format!("sunna-changes-nobase-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&none);
        std::fs::create_dir_all(&none).unwrap();
        git(&none, &["init", "-q", "-b", "other"]);
        std::fs::write(none.join("x"), "x").unwrap();
        git(&none, &["add", "-A"]);
        git(&none, &["commit", "-qm", "x"]);
        assert_eq!(merge_base(&Git { root: none.to_str().unwrap(), on: None }, "default"), None);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&none);
    }
}

#[cfg(test)]
mod contrast {
    use crate::settings;
    use crate::window::native::{contrast, pull_readable};

    fn rgb(hex: &str, under: Option<[f32; 3]>) -> Option<[f32; 3]> {
        let h = hex.strip_prefix('#')?;
        let v = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok().map(|x| f32::from(x) / 255.0);
        let (r, g, b) = (v(0)?, v(2)?, v(4)?);
        let a = if h.len() == 8 { v(6)? } else { 1.0 };
        let base = under.unwrap_or([0.0; 3]);
        Some([r * a + base[0] * (1.0 - a), g * a + base[1] * (1.0 - a), b * a + base[2] * (1.0 - a)])
    }

    /// The text the panels draw (secondary text, status letters, +/-) is pulled to 4.5:1 on the panel's
    /// background and on the row's hover and selected fills (`native::tone`), in every shipped theme.
    #[test]
    fn panel_text_reaches_4_5_to_1_on_every_shipped_theme() {
        let mut bad = Vec::new();
        let mut moved = 0;
        for (name, theme, _, shipped) in settings::themes() {
            if !shipped {
                continue;
            }
            let get = |k: &str| theme.get(k).cloned().unwrap_or_default();
            let Some(bg) = rgb(&get("background"), None) else { continue };
            let fills = [bg, rgb(&get("hover"), Some(bg)).unwrap_or(bg), rgb(&get("selected"), Some(bg)).unwrap_or(bg)];
            let foreground = rgb(&get("foreground"), Some(bg)).unwrap_or(bg);
            for key in ["muted", "ansi1", "ansi2", "blocked"] {
                let Some(fg) = rgb(&get(key), Some(bg)) else { continue };
                let out = pull_readable(fg, &fills, foreground, 4.5);
                moved += usize::from(out != fg);
                // On every fill when some color can read on all of them; always on the page.
                let solvable = [[0.0; 3], [1.0; 3]].iter().any(|c| fills.iter().all(|f| contrast(*c, *f) >= 4.5));
                for fill in if solvable { &fills[..] } else { &fills[..1] } {
                    let r = contrast(out, *fill);
                    if r < 4.5 {
                        bad.push(format!("{name}: {key} = {r:.2}"));
                    }
                }
            }
        }
        assert!(bad.is_empty(), "below 4.5:1 after the pull:\n{}", bad.join("\n"));
        // The pull only moves what needs it: some shipped colors already read, some do not.
        assert!(moved > 0);
    }
}
