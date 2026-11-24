//! What Settings > Mods does to disk and the network: reading what is
//! installed, installing mods, themes and layouts from GitHub (`owner/repo[@ref]`),
//! a git URL or a folder (pinned to the commit that was read), turning mods
//! off, and the GitHub catalog (topics herdr-mod, herdr-theme, herdr-layout).
//! Everything here blocks: the page runs it on a background thread. Nothing
//! here reaches for the app's home on its own: it is always passed in.

use crate::{mods, update};
use fancy_regex::Regex;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::timefmt;

/// The mods that ship in defaults/mods (a test keeps this list equal to the folder).
pub const SHIPPED_MODS: &[&str] = &[
    "activity", "add-project", "agents", "awake-menu", "bots", "changes", "files", "folders", "help", "machine-add", "menu", "new-menu", "pane", "peek",
    "picker", "prompt", "search", "spaces", "tabs", "terminals-bar", "terminals-pane", "toast", "welcome", "win98", "workspace-chrome", "workspace-status", "worktree",
];

/// Why something failed: a locale key, and what the program said.
#[derive(Debug, Clone, PartialEq)]
pub struct Fail {
    pub key: String,
    pub body: Option<String>,
}

impl Fail {
    fn new(key: &str) -> Self {
        Self { key: key.into(), body: None }
    }
    fn with(key: &str, body: impl Into<String>) -> Self {
        Self { key: key.into(), body: Some(body.into()).filter(|b| !b.trim().is_empty()) }
    }
    /// "Title: body", as the page shows it.
    pub fn text(&self) -> String {
        let title = crate::i18n::t(&self.key);
        match &self.body {
            Some(b) => format!("{title}: {b}"),
            None => title,
        }
    }
}

pub fn slug(name: &str) -> String {
    let lower = name.to_lowercase();
    let mut out = String::new();
    let mut word = false;
    for ch in lower.trim().chars() {
        if ch.is_whitespace() {
            word = true;
            continue;
        }
        if ch == '-' || ch.is_ascii_digit() || ch.is_ascii_lowercase() {
            if word && !out.is_empty() {
                out.push('-');
            }
            word = false;
            out.push(ch);
        }
    }
    let mut collapsed = String::new();
    for ch in out.chars() {
        if ch == '-' && collapsed.ends_with('-') {
            continue;
        }
        collapsed.push(ch);
    }
    collapsed.trim_matches('-').to_owned()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Github,
    Git,
    Folder,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    pub kind: SourceKind,
    pub url: String,
    pub path: String,
    pub git_ref: Option<String>,
    pub label: String,
    pub name: String,
}

fn word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `owner/repo[@ref]`, a git URL, or a folder.
pub fn parse_source(input: &str) -> Result<Source, Fail> {
    let s = input.trim();
    let (left, git_ref) = match s.split_once('@') {
        Some((l, r)) => (l, Some(r)),
        None => (s, None),
    };
    let part = |p: &str| !p.is_empty() && p.chars().all(|c| word(c) || c == '.' || c == '-');
    if let Some((owner, repo)) = left.split_once('/')
        && part(owner)
        && part(repo)
        && git_ref.is_none_or(|r| !r.is_empty() && r.chars().all(|c| word(c) || matches!(c, '.' | '/' | '-')))
    {
        return Ok(Source {
            kind: SourceKind::Github,
            url: format!("https://github.com/{owner}/{repo}.git"),
            path: String::new(),
            git_ref: git_ref.map(str::to_owned),
            label: format!("{owner}/{repo}"),
            name: slug(repo),
        });
    }
    if ["http://", "https://", "ssh://", "git@"].iter().any(|p| s.starts_with(p)) || s.ends_with(".git") {
        let base = s.strip_suffix(".git").unwrap_or(s).split(['/', ':']).rfind(|p| !p.is_empty()).unwrap_or("mod").to_owned();
        return Ok(Source { kind: SourceKind::Git, url: s.to_owned(), path: String::new(), git_ref: None, label: s.to_owned(), name: slug(&base) });
    }
    let drive = s.len() > 2 && s.as_bytes()[0].is_ascii_alphabetic() && s.as_bytes()[1] == b':' && matches!(s.as_bytes()[2], b'/' | b'\\');
    if s.starts_with('/') || s.starts_with("~/") || drive {
        let last = s.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or_default();
        return Ok(Source { kind: SourceKind::Folder, url: String::new(), path: s.to_owned(), git_ref: None, label: s.to_owned(), name: slug(last) });
    }
    Err(Fail::new("settings.mods_bad_source"))
}

pub struct Fetched {
    pub dir: PathBuf,
    pub commit: Option<String>,
    pub staging: Option<PathBuf>,
}

fn run(command: &str, args: &[&str], limit: u64) -> Result<(i32, String, String), String> {
    mods::run_blocking(command, &args.iter().map(|a| (*a).to_owned()).collect::<Vec<_>>(), None, Duration::from_secs(limit))
}

fn curl() -> String {
    update::system("curl").to_string_lossy().into_owned()
}

/// Brings the source's files to a folder: a shallow clone, or the folder itself.
pub fn fetch_source(home: &Path, src: &Source) -> Result<Fetched, Fail> {
    if src.kind == SourceKind::Folder {
        let dir = match src.path.strip_prefix("~/") {
            Some(rest) => mods::home().join(rest),
            None => PathBuf::from(&src.path),
        };
        if !dir.exists() {
            return Err(Fail::new("settings.mods_nothing_found"));
        }
        return Ok(Fetched { dir, commit: None, staging: None });
    }
    let parent = home.join("mod-store").join("staging");
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
    let staging = parent.join(format!("{}-{stamp}", if src.name.is_empty() { "src" } else { &src.name }));
    std::fs::create_dir_all(&parent).map_err(|e| Fail::with("settings.mods_clone_failed", e.to_string()))?;
    let target = staging.to_string_lossy().into_owned();
    let mut args = vec!["clone", "--depth", "1"];
    if let Some(r) = &src.git_ref {
        args.extend(["--branch", r]);
    }
    args.extend([src.url.as_str(), target.as_str()]);
    match run("git", &args, 120) {
        Ok((0, _, _)) => {}
        Ok((_, _, err)) => return Err(Fail::with("settings.mods_clone_failed", err)),
        Err(e) => return Err(Fail::with("settings.mods_clone_failed", e)),
    }
    let commit = run("git", &["-C", &target, "rev-parse", "HEAD"], 30).ok().map(|(_, o, _)| o.trim().to_owned()).filter(|c| !c.is_empty());
    Ok(Fetched { dir: staging.clone(), commit, staging: Some(staging) })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Mod,
    Theme,
    Layout,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Caps {
    pub programs: bool,
    pub network: bool,
    pub files: bool,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub kind: ItemKind,
    pub name: String,
    pub path: PathBuf,
    pub exists: bool,
    pub checked: bool,
    pub caps: Caps,
}

/// Names of the folders inside `dir` (none if it doesn't exist), hidden ones left out.
pub fn list_dirs(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| !n.starts_with('.'))
        .collect();
    out.sort();
    out
}

fn list_files(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir).into_iter().flatten().flatten().filter(|e| e.path().is_file()).filter_map(|e| e.file_name().into_string().ok()).collect();
    out.sort();
    out
}

pub fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

pub fn is_theme(j: &Value) -> bool {
    ["background", "foreground", "selection"].iter().all(|k| j[k].is_string()) && j["ansi"].as_array().is_some_and(|a| a.len() == 16)
}

/// What a source holds. Nothing installable is an error.
pub fn scan_source(dir: &Path, hint: &str) -> Result<Vec<Item>, Fail> {
    let item = |kind, name: String, path: PathBuf| Item { kind, name, path, exists: false, checked: false, caps: Caps::default() };
    let mut items = Vec::new();
    if dir.join("main.js").exists() {
        let base = if hint.is_empty() { dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default() } else { hint.to_owned() };
        items.push(item(ItemKind::Mod, slug(&base), dir.to_owned()));
    }
    for name in list_dirs(&dir.join("mods")) {
        if dir.join("mods").join(&name).join("main.js").exists() {
            items.push(item(ItemKind::Mod, name.clone(), dir.join("mods").join(name)));
        }
    }
    for f in list_files(&dir.join("themes")) {
        if let Some(name) = f.strip_suffix(".json")
            && read_json(&dir.join("themes").join(&f)).is_some_and(|j| is_theme(&j))
        {
            items.push(item(ItemKind::Theme, name.to_owned(), dir.join("themes").join(&f)));
        }
    }
    for f in list_files(&dir.join("layouts")) {
        if let Some(name) = f.strip_suffix(".json") {
            items.push(item(ItemKind::Layout, name.to_owned(), dir.join("layouts").join(&f)));
        }
    }
    if items.is_empty() {
        return Err(Fail::new("settings.mods_nothing_found"));
    }
    Ok(items)
}

pub fn destination(home: &Path, item: &Item) -> PathBuf {
    match item.kind {
        ItemKind::Mod => home.join("mods").join(&item.name),
        ItemKind::Theme => home.join("themes").join(format!("{}.json", item.name)),
        ItemKind::Layout => home.join("layouts").join(format!("{}.json", item.name)),
    }
}

/// Marks the items whose destination is taken (`replace`: a mod name being updated is allowed).
pub fn with_existing(home: &Path, items: Vec<Item>, replace: Option<&str>) -> Vec<Item> {
    items
        .into_iter()
        .map(|mut i| {
            i.exists = !(i.kind == ItemKind::Mod && Some(i.name.as_str()) == replace) && destination(home, &i).exists();
            i.checked = !i.exists;
            i
        })
        .collect()
}

/// What a mod's code can do, read from its source: it's trusted code, so say so before installing.
pub fn scan_capabilities(dir: &Path) -> Caps {
    fn files(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                files(&p, out);
            } else if p.extension().is_some_and(|x| x == "js") {
                out.push(p);
            }
        }
    }
    let re = |s: &str| Regex::new(s).expect("a fixed pattern");
    let (programs, network, writes) = (re(r"\b(exec|hc)\("), re(r#"\bfetch\(|from\s+"(net|websocket)""#), re(r#"\bwriteTextFile\(|from\s+"fs/promises""#));
    let mut found = Vec::new();
    files(dir, &mut found);
    let mut caps = Caps::default();
    for f in found {
        let Ok(src) = std::fs::read_to_string(f) else { continue };
        caps.programs |= programs.is_match(&src).unwrap_or(false);
        caps.network |= network.is_match(&src).unwrap_or(false);
        caps.files |= writes.is_match(&src).unwrap_or(false);
    }
    caps
}

/// A copy of a folder; a clone's `.git` is not part of what is installed.
pub fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        if e.file_name() == ".git" {
            continue;
        }
        let (a, b) = (e.path(), to.join(e.file_name()));
        if a.is_dir() {
            copy_dir(&a, &b)?;
        } else {
            std::fs::copy(&a, &b)?;
        }
    }
    Ok(())
}

/// Moves a folder; never writes over `to`.
pub fn move_dir(from: &Path, to: &Path) -> Result<(), Fail> {
    if to.exists() {
        return Err(Fail::new("settings.mods_name_taken"));
    }
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Fail::with("settings.it_didn_t_work", e.to_string()))?;
    }
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    copy_dir(from, to).and_then(|()| std::fs::remove_dir_all(from)).map_err(|e| Fail::with("settings.it_didn_t_work", e.to_string()))
}

pub struct Origin {
    pub source: String,
    pub git_ref: Option<String>,
    pub commit: Option<String>,
}

/// Copies the chosen items; never writes over what exists, except the mod being updated.
pub fn install_items(home: &Path, items: &[Item], meta: &Origin, replace: Option<&str>) -> Result<usize, Fail> {
    let io = |e: std::io::Error| Fail::with("settings.it_didn_t_work", e.to_string());
    for item in items {
        let dest = destination(home, item);
        if item.kind == ItemKind::Mod {
            if Some(item.name.as_str()) == replace && dest.exists() {
                std::fs::remove_dir_all(&dest).map_err(io)?;
            } else if dest.exists() {
                return Err(Fail::new("settings.mods_exists"));
            }
            copy_dir(&item.path, &dest).map_err(io)?;
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
            let record = json!({ "source": meta.source, "ref": meta.git_ref, "commit": meta.commit, "installed_at": timefmt::iso(now) });
            std::fs::write(dest.join(".herdr-install.json"), serde_json::to_string_pretty(&record).unwrap_or_default() + "\n").map_err(io)?;
        } else {
            if dest.exists() {
                return Err(Fail::new("settings.mods_exists"));
            }
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(io)?;
            }
            std::fs::copy(&item.path, &dest).map_err(io)?;
        }
    }
    Ok(items.len())
}

/// The newest commit of an installed mod's source.
pub fn latest_commit(source: &str, git_ref: Option<&str>) -> Option<String> {
    let src = parse_source(&match git_ref {
        Some(r) => format!("{source}@{r}"),
        None => source.to_owned(),
    })
    .ok()?;
    if src.kind == SourceKind::Folder {
        return None;
    }
    let (code, out, _) = run("git", &["ls-remote", &src.url, src.git_ref.as_deref().unwrap_or("HEAD")], 30).ok()?;
    (code == 0).then(|| out.split_whitespace().next().map(str::to_owned)).flatten()
}

// What is installed ------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Installed {
    pub name: String,
    pub disabled: bool,
    pub dir: PathBuf,
    /// Relative to the app's home, where its assets are.
    pub rel: String,
    pub meta: Value,
    pub install: Option<Value>,
    pub caps: Caps,
    pub shipped: bool,
}

pub fn list_installed(home: &Path) -> Vec<Installed> {
    let mut list = Vec::new();
    for (root, disabled, rel_root) in [(home.join("mods"), false, "mods"), (home.join("mod-store").join("disabled"), true, "mod-store/disabled")] {
        for name in list_dirs(&root) {
            let dir = root.join(&name);
            list.push(Installed {
                disabled,
                rel: format!("{rel_root}/{name}"),
                meta: read_json(&dir.join("mod.json")).unwrap_or_else(|| json!({})),
                install: read_json(&dir.join(".herdr-install.json")),
                caps: scan_capabilities(&dir),
                shipped: !disabled && SHIPPED_MODS.contains(&name.as_str()),
                dir,
                name,
            });
        }
    }
    list
}

impl Installed {
    pub fn title(&self) -> String {
        self.meta["name"].as_str().unwrap_or(&self.name).to_owned()
    }
    pub fn commit(&self) -> Option<&str> {
        self.install.as_ref()?["commit"].as_str().filter(|c| !c.is_empty())
    }
    /// `source` or `source@ref`, as it was typed to install.
    pub fn source_input(&self) -> Option<String> {
        let i = self.install.as_ref()?;
        let source = i["source"].as_str()?;
        Some(match i["ref"].as_str().filter(|r| !r.is_empty()) {
            Some(r) => format!("{source}@{r}"),
            None => source.to_owned(),
        })
    }
}

// The GitHub catalog -----------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Entry {
    pub full_name: String,
    #[serde(default, deserialize_with = "text_or_empty")]
    pub description: String,
    #[serde(default)]
    pub stargazers_count: u64,
    #[serde(default, deserialize_with = "text_or_empty")]
    pub pushed_at: String,
    #[serde(default, deserialize_with = "text_or_empty")]
    pub html_url: String,
}

fn text_or_empty<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(d)?.unwrap_or_default())
}

const TTL: Duration = Duration::from_secs(600);

static CATALOG: Mutex<Option<HashMap<String, (Instant, Vec<Entry>)>>> = Mutex::new(None);

/// GitHub's answer to `path` as JSON: the API through curl, else the `gh` CLI. The flag: rate limited.
fn github(path: &str) -> (Option<Value>, bool) {
    let url = format!("https://api.github.com/{path}");
    let mut limited = false;
    if let Ok((0, out, _)) = run(&curl(), &["-sSL", "-H", "Accept: application/vnd.github+json", "-H", "User-Agent: sunna", "-w", "\n%{http_code}", &url], 20)
        && let Some((body, status)) = out.rsplit_once('\n')
    {
        match status.trim() {
            "200" => return (serde_json::from_str(body).ok(), false),
            "403" | "429" => limited = true,
            _ => {}
        }
    }
    if let Ok((0, out, _)) = run("gh", &["api", path], 30)
        && let Ok(v) = serde_json::from_str(&out)
    {
        return (Some(v), false);
    }
    (None, limited)
}

/// Repositories with the GitHub topic `kind`, most starred first.
pub fn catalog(kind: &str) -> Result<Vec<Entry>, Fail> {
    if let Some((at, list)) = CATALOG.lock().ok().and_then(|c| c.as_ref()?.get(kind).cloned())
        && at.elapsed() < TTL
    {
        return Ok(list);
    }
    let (data, limited) = github(&format!("search/repositories?q=topic:{kind}&sort=stars&per_page=50"));
    let Some(data) = data else { return Err(Fail::new(if limited { "settings.mods_rate_limited" } else { "settings.mods_catalog_failed" })) };
    let list: Vec<Entry> = data["items"].as_array().into_iter().flatten().filter_map(|r| serde_json::from_value(r.clone()).ok()).collect();
    if let Ok(mut c) = CATALOG.lock() {
        c.get_or_insert_with(HashMap::new).insert(kind.to_owned(), (Instant::now(), list.clone()));
    }
    Ok(list)
}

/// Whether the path of a repo's file is something `scan_source` installs.
pub fn installable_path(p: &str) -> bool {
    let parts: Vec<&str> = p.split('/').collect();
    match parts.as_slice() {
        ["main.js"] => true,
        ["mods", _, "main.js"] => true,
        ["themes", f] | ["layouts", f] => f.ends_with(".json") && f.len() > 5,
        _ => false,
    }
}

/// Whether a GitHub repo holds something installable; None when GitHub couldn't say.
pub fn compatible(full_name: &str) -> Option<bool> {
    let (data, _) = github(&format!("repos/{full_name}/git/trees/HEAD?recursive=1"));
    let tree = data?["tree"].as_array()?.clone();
    Some(tree.iter().any(|n| n["path"].as_str().is_some_and(installable_path)))
}

/// Whole days since `iso`.
pub fn age_days(iso: &str, now: i64) -> Option<i64> {
    Some(((now - timefmt::parse_iso(iso)?) / 86_400).max(0))
}

/// Downloads the first of `urls` that exists to `path`.
pub fn download(path: &Path, urls: &[String]) -> bool {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let target = path.to_string_lossy().into_owned();
    urls.iter().any(|u| matches!(run(&curl(), &["-fsSL", "-o", &target, u], 20), Ok((0, _, _))) && path.exists())
}

#[cfg(test)]
mod tests {
    use super::{
        Caps, Entry, ItemKind, Origin, SHIPPED_MODS, SourceKind, age_days, copy_dir, fetch_source, install_items, installable_path, list_dirs, list_installed, move_dir, parse_source, read_json, scan_capabilities, scan_source, slug,
        with_existing,
    };
    use serde_json::json;
    use std::path::{Path, PathBuf};

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sunna-modstore-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn put(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    const THEME: &str = r##"{"background":"#000000","foreground":"#ffffff","selection":"#111111","ansi":["#000000","#000000","#000000","#000000","#000000","#000000","#000000","#000000","#000000","#000000","#000000","#000000","#000000","#000000","#000000","#000000"]}"##;

    #[test]
    fn slugs_match_the_theme_importer() {
        assert_eq!(slug("  My Cool Theme!  "), "my-cool-theme");
        assert_eq!(slug("a--b"), "a-b");
        assert_eq!(slug("-x-"), "x");
        assert_eq!(slug("herdr_mod.v2"), "herdrmodv2");
    }

    #[test]
    fn parse_source_tells_github_git_and_folders_apart() {
        let s = parse_source("acme/tools@v2").unwrap();
        assert_eq!((s.kind, s.url.as_str(), s.git_ref.as_deref(), s.label.as_str()), (SourceKind::Github, "https://github.com/acme/tools.git", Some("v2"), "acme/tools"));
        assert_eq!(parse_source("https://example.com/a/b.git").unwrap().kind, SourceKind::Git);
        assert_eq!(parse_source("https://example.com/a/b.git").unwrap().name, "b");
        assert_eq!(parse_source("git@github.com:a/b.git").unwrap().kind, SourceKind::Git);
        let f = parse_source("/tmp/mod-src").unwrap();
        assert_eq!((f.kind, f.path.as_str(), f.name.as_str()), (SourceKind::Folder, "/tmp/mod-src", "mod-src"));
        assert_eq!(parse_source("~/mods/x").unwrap().kind, SourceKind::Folder);
        assert_eq!(parse_source("C:\\mods\\x").unwrap().kind, SourceKind::Folder);
        assert_eq!(parse_source("acme/tools@").unwrap_err().key, "settings.mods_bad_source");
        assert_eq!(parse_source("just words").unwrap_err().key, "settings.mods_bad_source");
    }

    #[test]
    fn scan_source_finds_mods_themes_and_layouts_and_nothing_is_an_error() {
        let dir = scratch("scan");
        put(&dir.join("mods/a/main.js"), "x");
        put(&dir.join("themes/t.json"), THEME);
        put(&dir.join("themes/bad.json"), r##"{"background":"#000000"}"##);
        put(&dir.join("layouts/l.json"), "{}");
        let items = scan_source(&dir, "repo").unwrap();
        assert_eq!(items.iter().map(|i| (i.kind, i.name.as_str())).collect::<Vec<_>>(), [(ItemKind::Mod, "a"), (ItemKind::Theme, "t"), (ItemKind::Layout, "l")]);
        let own = scratch("scan-own");
        put(&own.join("main.js"), "x");
        assert_eq!(scan_source(&own, "My Mod").unwrap()[0].name, "my-mod");
        let empty = scratch("scan-empty");
        put(&empty.join("readme.md"), "hi");
        assert_eq!(scan_source(&empty, "").unwrap_err().key, "settings.mods_nothing_found");
    }

    #[test]
    fn an_existing_destination_is_flagged_and_never_overwritten_and_an_install_records_its_source() {
        let (src, home) = (scratch("inst-src"), scratch("inst-home"));
        put(&src.join("mods/a/main.js"), "new");
        put(&src.join("mods/b/main.js"), "b");
        put(&src.join("mods/b/.git/HEAD"), "history");
        put(&home.join("mods/a/main.js"), "mine");
        let items = with_existing(&home, scan_source(&src, "").unwrap(), None);
        assert_eq!(items.iter().map(|i| (i.name.as_str(), i.exists, i.checked)).collect::<Vec<_>>(), [("a", true, false), ("b", false, true)]);
        let meta = Origin { source: "acme/x".into(), git_ref: None, commit: Some("abc123".into()) };
        assert_eq!(install_items(&home, &items[..1], &meta, None).unwrap_err().key, "settings.mods_exists");
        assert_eq!(std::fs::read_to_string(home.join("mods/a/main.js")).unwrap(), "mine");
        assert_eq!(install_items(&home, &items[1..], &meta, None).unwrap(), 1);
        assert_eq!(std::fs::read_to_string(home.join("mods/b/main.js")).unwrap(), "b");
        assert!(!home.join("mods/b/.git").exists());
        let record = read_json(&home.join("mods/b/.herdr-install.json")).unwrap();
        assert_eq!((record["source"].as_str(), record["commit"].as_str()), (Some("acme/x"), Some("abc123")));
        // Updating replaces the mod that is being updated.
        let update = with_existing(&home, scan_source(&src, "").unwrap(), Some("a"));
        assert!(!update[0].exists);
        install_items(&home, &update[..1], &meta, Some("a")).unwrap();
        assert_eq!(std::fs::read_to_string(home.join("mods/a/main.js")).unwrap(), "new");
    }

    #[test]
    fn a_folder_install_works_end_to_end_with_a_local_folder() {
        let (src, home) = (scratch("e2e-src"), scratch("e2e-home"));
        put(&src.join("main.js"), "export default class A {}");
        put(&src.join("mod.json"), r#"{"name":"Prueba","place":"none"}"#);
        let source = parse_source(&src.to_string_lossy()).unwrap();
        let got = fetch_source(&home, &source).unwrap();
        assert!(got.staging.is_none() && got.commit.is_none());
        let items = with_existing(&home, scan_source(&got.dir, &source.name).unwrap(), None);
        install_items(&home, &items, &Origin { source: source.path.clone(), git_ref: None, commit: None }, None).unwrap();
        let list = list_installed(&home);
        assert_eq!((list.len(), list[0].title(), list[0].shipped, list[0].disabled), (1, "Prueba".to_owned(), false, false));
        // Off, then on again, then the name is taken.
        let name = list[0].name.clone();
        move_dir(&list[0].dir, &home.join("mod-store/disabled").join(&name)).unwrap();
        assert!(list_installed(&home)[0].disabled);
        copy_dir(&src, &home.join("mods").join(&name)).unwrap();
        assert_eq!(move_dir(&home.join("mod-store/disabled").join(&name), &home.join("mods").join(&name)).unwrap_err().key, "settings.mods_name_taken");
    }

    #[test]
    fn capabilities_are_read_from_the_code() {
        let dir = scratch("caps");
        put(&dir.join("a/main.js"), r#"const r = await exec("git", ["status"]);"#);
        put(&dir.join("b/main.js"), r#"fetch("https://x")"#);
        put(&dir.join("c/main.js"), r#"import * as fs from "fs/promises";"#);
        put(&dir.join("d/main.js"), "export default class D {}");
        assert_eq!(scan_capabilities(&dir.join("a")), Caps { programs: true, network: false, files: false });
        assert_eq!(scan_capabilities(&dir.join("b")), Caps { programs: false, network: true, files: false });
        assert!(scan_capabilities(&dir.join("c")).files);
        assert_eq!(scan_capabilities(&dir.join("d")), Caps::default());
    }

    #[test]
    fn installable_paths_are_the_ones_scan_source_reads() {
        for ok in ["main.js", "mods/x/main.js", "themes/t.json", "layouts/l.json"] {
            assert!(installable_path(ok), "{ok}");
        }
        for no in ["README.md", "src/lib.rs", "mods/x/y/main.js", "themes/.json", "themes/a/b.json"] {
            assert!(!installable_path(no), "{no}");
        }
    }

    #[test]
    fn catalog_entries_tolerate_nulls_and_extra_fields() {
        let e: Entry = serde_json::from_value(json!({ "full_name": "a/b", "description": null, "stargazers_count": 3, "pushed_at": "2026-01-01T00:00:00Z", "html_url": "https://github.com/a/b", "extra": 1 })).unwrap();
        assert_eq!((e.description.as_str(), e.stargazers_count), ("", 3));
        assert_eq!(age_days(&e.pushed_at, 1_767_225_600 + 3 * 86_400 + 5), Some(3));
        assert_eq!(age_days("garbage", 0), None);
    }

    #[test]
    fn shipped_mods_equal_the_folders_of_defaults_mods() {
        let mut real = list_dirs(&Path::new(env!("CARGO_MANIFEST_DIR")).join("defaults/mods"));
        real.sort();
        let mut listed: Vec<String> = SHIPPED_MODS.iter().map(|s| (*s).to_owned()).collect();
        listed.sort();
        assert_eq!(listed, real);
    }

    #[test]
    fn an_installed_mod_remembers_how_to_update() {
        let home = scratch("src-input");
        put(&home.join("mods/u/main.js"), "x");
        put(&home.join("mods/u/.herdr-install.json"), r#"{"source":"a/u","ref":"v1","commit":"aaa1111"}"#);
        let m = &list_installed(&home)[0];
        assert_eq!((m.commit(), m.source_input().as_deref()), (Some("aaa1111"), Some("a/u@v1")));
    }
}
