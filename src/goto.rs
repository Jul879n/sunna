//! Go to file, the pure part: listing a project's files, fuzzy matching and
//! ranking. The overlay that shows it is `window/goto_file.rs`.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

/// The most files listed; the dialog says so when it is reached.
pub const MAX_FILES: usize = 60000;
/// The most rows ranked and kept for the list.
pub const LIMIT: usize = 200;
/// Folders that are never worth listing.
const SKIP: [&str; 3] = [".git", "node_modules", "target"];
const RECENT_KEPT: usize = 20;
const LIST_LIMIT: Duration = Duration::from_secs(30);
const FIND: &str = "find . \\( -name .git -o -name node_modules -o -name target \\) -prune -o -type f -print 2>/dev/null | head -n 60000";

/// True for a path inside a folder that is never worth listing.
pub fn skipped(path: &str) -> bool {
    path.split('/').any(|part| SKIP.contains(&part))
}

/// One path per line to a clean, capped list (no "./", no ignored folders, no blanks).
pub fn parse_list(stdout: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in stdout.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let path = line.strip_prefix("./").unwrap_or(line);
        if path.is_empty() || skipped(path) {
            continue;
        }
        out.push(path.to_owned());
        if out.len() >= MAX_FILES {
            break;
        }
    }
    out
}

fn lower(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

fn separator(c: char) -> bool {
    c.is_whitespace() || matches!(c, ':' | '.' | '-' | '_' | '/')
}

/// Fuzzy score of `q` (lowercase) in `text`, with the matched positions (in characters);
/// None: no match. The letters of `q` in order: word starts and runs score higher.
pub fn fuzzy(q: &str, text: &str) -> Option<(f64, Vec<usize>)> {
    let mut wanted = q.chars().peekable();
    wanted.peek()?;
    let (mut score, mut at, mut run) = (0.0_f64, 0usize, 0usize);
    let mut hits: Vec<usize> = Vec::new();
    let mut prev: Option<char> = None;
    let mut len = 0usize;
    for (i, ch) in text.chars().enumerate() {
        len = i + 1;
        let Some(&want) = wanted.peek() else { continue };
        if lower(ch) == want {
            let start = prev.is_none_or(separator);
            run = if i == at && !hits.is_empty() { run + 1 } else { 0 };
            score += 1.0 + if start { 6.0 } else { 0.0 } + run as f64 * 3.0 - (i - at).min(6) as f64 * 0.3;
            hits.push(i);
            at = i + 1;
            wanted.next();
        }
        prev = Some(ch);
    }
    if wanted.peek().is_some() {
        return None;
    }
    Some((score - len as f64 * 0.02, hits))
}

/// "src/a/b.rs" to ("src/a/", "b.rs").
pub fn split(path: &str) -> (&str, &str) {
    path.split_at(path.rfind('/').map_or(0, |i| i + 1))
}

/// One row of the list.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub path: String,
    pub dir: String,
    pub name: String,
    pub name_hits: Vec<usize>,
    pub dir_hits: Vec<usize>,
    pub score: f64,
}

fn row(path: &str, name_hits: Vec<usize>, dir_hits: Vec<usize>, score: f64) -> Row {
    let (dir, name) = split(path);
    Row { path: path.to_owned(), dir: dir.to_owned(), name: name.to_owned(), name_hits, dir_hits, score }
}

fn starts_with_lower(text: &str, q: &str) -> bool {
    let mut t = text.chars().map(lower);
    q.chars().all(|c| t.next() == Some(c))
}

/// The files matching `query`, best first: a match in the file name beats one spread over
/// the folders. An empty query lists the `recent` files first, then the rest in order.
pub fn rank(files: &[String], query: &str, recent: &[String], limit: usize) -> Vec<Row> {
    let q: String = query.trim().chars().filter(|c| !c.is_whitespace()).flat_map(char::to_lowercase).collect();
    if q.is_empty() {
        let known: HashSet<&str> = files.iter().map(String::as_str).collect();
        let first: Vec<&str> = recent.iter().map(String::as_str).filter(|p| known.contains(p)).collect();
        let seen: HashSet<&str> = first.iter().copied().collect();
        return first.into_iter().chain(files.iter().map(String::as_str).filter(|p| !seen.contains(p))).take(limit).map(|p| row(p, vec![], vec![], 0.0)).collect();
    }
    let mut out = Vec::new();
    for path in files {
        let (dir, name) = split(path);
        if let Some((score, hits)) = fuzzy(&q, name) {
            let prefix = if starts_with_lower(name, &q) { 12.0 } else { 0.0 };
            out.push(row(path, hits, vec![], score + 30.0 + prefix));
        } else if let Some((score, hits)) = fuzzy(&q, path) {
            let cut = dir.chars().count();
            out.push(row(path, hits.iter().filter(|&&i| i >= cut).map(|i| i - cut).collect(), hits.iter().copied().filter(|&i| i < cut).collect(), score));
        }
    }
    out.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| a.path.chars().count().cmp(&b.path.chars().count())));
    out.truncate(limit);
    out
}

fn missing(code: i32, stderr: &str) -> bool {
    let e = stderr.to_lowercase();
    code == 127 || e.contains("not found") || e.contains("no such file") || e.contains("enoent")
}

/// Runs a program here, or on machine `on`: (exit code, stdout, stderr).
fn exec(command: &str, args: &[&str], cwd: &str, on: Option<&str>) -> Result<(i32, String, String), String> {
    let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
    match on {
        Some(m) => crate::mods::run_remote(m, command, &args, Some(cwd.to_owned())).map(|(c, o, e)| (c, String::from_utf8_lossy(&o).into_owned(), String::from_utf8_lossy(&e).into_owned())),
        None => crate::mods::run_blocking(command, &args, Some(cwd.to_owned()), LIST_LIMIT),
    }
}

/// The project root (the git top, else the folder) and its files, relative to it, sorted.
/// Blocking: call it off the UI thread.
pub fn list(cwd: &str, on: Option<&str>) -> Result<(String, Vec<String>), String> {
    let mut root = cwd.to_owned();
    if let Ok((0, out, _)) = exec("git", &["rev-parse", "--show-toplevel"], cwd, on)
        && !out.trim().is_empty()
    {
        root = out.trim().to_owned();
    }
    let attempts: [(&str, &[&str]); 3] = [
        ("rg", &["--files", "--hidden", "-g", "!.git", "-g", "!node_modules", "-g", "!target"]),
        ("git", &["ls-files", "-co", "--exclude-standard"]),
        ("sh", &["-c", FIND]),
    ];
    for (command, args) in attempts {
        let Ok((code, out, err)) = exec(command, args, &root, on) else { continue };
        if missing(code, &err) || (code != 0 && out.is_empty()) {
            continue;
        }
        let mut files = parse_list(&out);
        files.sort();
        return Ok((root, files));
    }
    Err(crate::i18n::t("rs.goto.no_engine"))
}

fn recent_file() -> PathBuf {
    crate::mods::app_dir().join("data").join("goto-file").join("recent.json")
}

fn recent_key(on: Option<&str>, root: &str) -> String {
    format!("{}:{root}", on.unwrap_or(""))
}

/// The files opened last in this project, newest first.
pub fn recent(on: Option<&str>, root: &str) -> Vec<String> {
    let all: serde_json::Value = std::fs::read_to_string(recent_file()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    all.get(recent_key(on, root)).and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|p| p.as_str().map(str::to_owned)).collect()).unwrap_or_default()
}

/// Puts `path` first among the project's recent files.
pub fn remember(on: Option<&str>, root: &str, path: &str) {
    let file = recent_file();
    let mut all: serde_json::Value = std::fs::read_to_string(&file).ok().and_then(|s| serde_json::from_str(&s).ok()).filter(serde_json::Value::is_object).unwrap_or_else(|| serde_json::json!({}));
    let mut paths = recent(on, root);
    paths.retain(|p| p != path);
    paths.insert(0, path.to_owned());
    paths.truncate(RECENT_KEPT);
    all[recent_key(on, root)] = serde_json::json!(paths);
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = crate::persist::write_atomic(&file, all.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files() -> Vec<String> {
        ["src/main.rs", "src/window/editor.rs", "README.md", "docs/editor-notes.md", "web/app/main.tsx"].map(String::from).to_vec()
    }

    fn paths(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(|r| r.path.as_str()).collect()
    }

    #[test]
    fn the_file_list_drops_blanks_dot_slash_and_the_folders_that_are_never_listed() {
        let out = parse_list("./src/a.rs\n\nnode_modules/x/i.js\nsrc/target/o\n.git/config\r\nweb/.git/HEAD\nREADME.md\r\n");
        assert_eq!(out, ["src/a.rs", "README.md"]);
    }

    #[test]
    fn the_list_stops_at_the_cap() {
        let many: String = (0..MAX_FILES + 5).map(|i| format!("f{i}\n")).collect();
        assert_eq!(parse_list(&many).len(), MAX_FILES);
    }

    #[test]
    fn a_name_match_beats_a_match_spread_over_folders_and_typing_the_name_finds_the_file() {
        assert_eq!(paths(&rank(&files(), "editor", &[], LIMIT)), ["src/window/editor.rs", "docs/editor-notes.md"]);
        assert_eq!(rank(&files(), "main", &[], LIMIT)[0].path, "src/main.rs");
        let hit = &rank(&files(), "readme", &[], LIMIT)[0];
        assert_eq!((hit.path.as_str(), hit.name.as_str(), hit.dir.as_str()), ("README.md", "README.md", ""));
        assert_eq!(hit.name_hits, [0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn a_query_that_crosses_folders_matches_the_path_and_marks_the_folder_letters() {
        let rows = rank(&files(), "winedit", &[], LIMIT);
        assert_eq!(rows[0].path, "src/window/editor.rs");
        assert!(!rows[0].dir_hits.is_empty());
        assert!(rank(&files(), "zzz", &[], LIMIT).is_empty());
    }

    #[test]
    fn an_empty_query_shows_the_recent_files_first_ignoring_ones_that_no_longer_exist() {
        let recent = ["web/app/main.tsx", "gone.rs", "README.md"].map(String::from);
        let out = rank(&files(), "  ", &recent, LIMIT);
        assert_eq!(&paths(&out)[..2], ["web/app/main.tsx", "README.md"]);
        assert_eq!(out.len(), files().len());
        assert_eq!(rank(&files(), "", &[], 2).len(), 2);
    }

    #[test]
    fn the_rows_are_capped_and_a_name_prefix_ranks_first() {
        let many: Vec<String> = (0..500).map(|i| format!("dir/file{i}.rs")).collect();
        assert_eq!(rank(&many, "file", &[], LIMIT).len(), LIMIT);
        let names = ["a/xmain.rs", "b/main.rs"].map(String::from);
        assert_eq!(rank(&names, "main", &[], LIMIT)[0].path, "b/main.rs");
    }

    #[test]
    fn fuzzy_scores_runs_and_word_starts_and_reports_the_letters() {
        let (run, hits) = fuzzy("abc", "abc.rs").unwrap();
        let (spread, _) = fuzzy("abc", "axbxc.rs").unwrap();
        assert!(run > spread);
        assert_eq!(hits, [0, 1, 2]);
        assert!(fuzzy("abc", "acb").is_none());
        assert!(fuzzy("", "abc").is_none());
    }

    #[test]
    fn it_ignores_case_and_spaces_in_the_query() {
        assert_eq!(rank(&files(), " Read Me ", &[], LIMIT)[0].path, "README.md");
    }

    #[test]
    fn multibyte_names_keep_their_letter_positions() {
        let rows = rank(&["año/ñandú.txt".to_owned()], "ñand", &[], LIMIT);
        assert_eq!(rows[0].name_hits, [0, 1, 2, 3]);
    }

    #[test]
    fn a_missing_program_is_told_by_its_exit_code_or_its_words() {
        assert!(missing(127, ""));
        assert!(missing(1, "sh: rg: command not found"));
        assert!(!missing(1, "fatal: not a git repository"));
        assert!(!missing(0, ""));
    }
}
