//! Files in the native editor: reading them, what "unsaved" means, and a save
//! that never overwrites what changed on disk since it was read. Pure data and
//! blocking file work; the window's editor panes (`window/editor.rs`) drive it.

use std::io::Read;
use std::path::Path;

/// Why a file can't be shown as text.
#[derive(Debug, Clone, PartialEq)]
pub enum LoadError {
    Missing,
    /// Bigger than the 1 MiB mods can read too (its size in bytes, 0 when unknown).
    TooLarge(u64),
    /// Has NUL bytes: an image, an archive, a program.
    Binary,
    /// Text in another encoding.
    NotUtf8,
    Io(String),
}

impl LoadError {
    /// The sentence the pane shows.
    pub fn message(&self, path: &str) -> String {
        match self {
            LoadError::Missing => crate::i18n::tf("rs.editor.missing", &[("path", path)]),
            LoadError::TooLarge(_) => crate::i18n::t("rs.editor.too_large"),
            LoadError::Binary => crate::i18n::t("rs.editor.binary"),
            LoadError::NotUtf8 => crate::i18n::t("rs.editor.not_utf8"),
            LoadError::Io(e) => crate::i18n::tf("rs.editor.cannot_read", &[("error", e)]),
        }
    }
}

/// UTF-8 text from file bytes, or why not.
pub fn decode(bytes: Vec<u8>) -> Result<String, LoadError> {
    if bytes.len() as u64 > crate::mod_files::LIMIT {
        return Err(LoadError::TooLarge(bytes.len() as u64));
    }
    if bytes.contains(&0) {
        return Err(LoadError::Binary);
    }
    String::from_utf8(bytes).map_err(|_| LoadError::NotUtf8)
}

/// A local file's text. Reads at most the limit plus one byte, so a huge file never loads.
pub fn read_local(path: &Path) -> Result<String, LoadError> {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(LoadError::Missing),
        Err(e) => return Err(LoadError::Io(e.to_string())),
    };
    if file.metadata().is_ok_and(|m| m.is_dir()) {
        return Err(LoadError::Io(crate::i18n::t("rs.editor.is_folder")));
    }
    let mut bytes = Vec::new();
    file.take(crate::mod_files::LIMIT + 1).read_to_end(&mut bytes).map_err(|e| LoadError::Io(e.to_string()))?;
    decode(bytes)
}

/// A file on another machine (read only: the local API can't write there).
pub fn read_remote(machine: &str, path: &str) -> Result<String, LoadError> {
    match crate::mods::read_remote(machine, path) {
        Ok(Some(text)) if text.contains('\0') => Err(LoadError::Binary),
        Ok(Some(text)) => Ok(text),
        Ok(None) => Err(LoadError::Missing),
        Err(e) => Err(LoadError::Io(e)),
    }
}

/// The language the highlighter knows a file by (an extension or name syntect resolves).
pub fn language_for(path: &str) -> String {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path).to_lowercase();
    let ext = if name.contains('.') { name.rsplit('.').next().unwrap_or("") } else { "" };
    match ext {
        "ts" | "tsx" | "mts" | "cts" | "jsx" | "mjs" | "cjs" => "js",
        "toml" | "lock" => "yaml",
        "jsonc" | "json5" => "json",
        "svelte" | "vue" => "html",
        "markdown" | "mdx" => "md",
        "zsh" | "bash" | "env" => "sh",
        "" => match name.as_str() {
            "makefile" | "justfile" => "makefile",
            "dockerfile" => "sh",
            ".zshrc" | ".bashrc" | ".profile" | ".zprofile" => "sh",
            _ => "txt",
        },
        other => other,
    }
    .to_owned()
}

pub fn is_markdown(path: &str) -> bool {
    matches!(language_for(path).as_str(), "md")
}

/// The file's name, for a tab.
pub fn file_name(path: &str) -> &str {
    path.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().filter(|n| !n.is_empty()).unwrap_or(path)
}

/// What the disk says now compared with what was read.
#[derive(Debug, Clone, PartialEq)]
pub enum Disk {
    Same,
    Changed(String),
    Gone,
}

/// One open file's bookkeeping: what the disk had when it was read or last
/// written (the base a save is checked against) and what the editor showed then.
#[derive(Debug, Clone, PartialEq)]
pub struct Doc {
    baseline: String,
    shown: String,
    crlf: bool,
}

impl Doc {
    /// A file just read: the doc, and the text the editor shows (Windows line
    /// ends read as `\n` and come back as they were on save).
    pub fn open(disk: String) -> (Doc, String) {
        let crlf = disk.contains("\r\n") && disk.matches('\n').count() == disk.matches("\r\n").count();
        let shown = if crlf { disk.replace("\r\n", "\n") } else { disk.clone() };
        (Doc { baseline: disk, shown: shown.clone(), crlf }, shown)
    }

    pub fn baseline(&self) -> &str {
        &self.baseline
    }

    /// The bytes to write for what the editor shows.
    pub fn disk_text(&self, view: &str) -> String {
        if self.crlf { view.replace('\n', "\r\n") } else { view.to_owned() }
    }

    pub fn dirty(&self, view: &str) -> bool {
        self.shown != view
    }

    /// The file was written with `view`.
    pub fn saved(&mut self, view: &str) {
        self.baseline = self.disk_text(view);
        self.shown = view.to_owned();
    }

    /// Takes the disk's text as the new base (the person reloaded or chose to overwrite).
    pub fn rebase(&mut self, disk: String) -> String {
        let (doc, shown) = Doc::open(disk);
        *self = doc;
        shown
    }

    pub fn compare(&self, disk: Option<&str>) -> Disk {
        match disk {
            None => Disk::Gone,
            Some(d) if d == self.baseline => Disk::Same,
            Some(d) => Disk::Changed(d.to_owned()),
        }
    }
}

/// How a save ended.
#[derive(Debug, Clone, PartialEq)]
pub enum Saved {
    Written,
    /// The disk changed since the file was read (its text now, None: deleted).
    Conflict(Option<String>),
    Failed(String),
}

fn disk_now(path: &Path) -> Result<Option<String>, String> {
    crate::mod_files::read(path)
}

/// Writes `text` unless the file differs from `baseline`. Blocking: run it off the UI thread.
pub fn save(path: &Path, text: &str, baseline: &str) -> Saved {
    match disk_now(path) {
        Ok(Some(d)) if d == baseline => {}
        Ok(other) => return Saved::Conflict(other),
        Err(e) => return Saved::Failed(e),
    }
    match crate::mod_files::write(path, text, Some(baseline)) {
        Ok(()) => Saved::Written,
        // It may have changed between the check and the write.
        Err(e) => match disk_now(path) {
            Ok(Some(d)) if d == baseline => Saved::Failed(e),
            Ok(other) => Saved::Conflict(other),
            Err(_) => Saved::Failed(e),
        },
    }
}

/// Writes `text` over whatever the disk has now (the person confirmed). A deleted file is made again.
pub fn overwrite(path: &Path, text: &str) -> Saved {
    let now = match disk_now(path) {
        Ok(now) => now,
        Err(e) => return Saved::Failed(e),
    };
    match crate::mod_files::write(path, text, now.as_deref()) {
        Ok(()) => Saved::Written,
        Err(e) => Saved::Failed(e),
    }
}

/// "Save and close" in flight: the editors still being written, and the command
/// (close a tab, quit...) that runs once every one is on disk.
#[derive(Debug, Clone, PartialEq)]
pub struct SaveThen {
    waiting: std::collections::BTreeSet<String>,
    pub command: String,
    pub args: serde_json::Value,
}

/// What one finished save means for a [`SaveThen`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Step {
    /// Other files are still being written (or this pane isn't part of it).
    Waiting,
    /// Every file is saved: run the command.
    Done,
    /// This file wasn't saved (conflict, error, typed again): nothing closes.
    Failed,
}

impl SaveThen {
    pub fn new(panes: impl IntoIterator<Item = String>, command: &str, args: serde_json::Value) -> SaveThen {
        SaveThen { waiting: panes.into_iter().collect(), command: command.to_owned(), args }
    }

    pub fn waits_on(&self, pane: &str) -> bool {
        self.waiting.contains(pane)
    }

    /// `clean`: the pane has no unsaved changes after its save finished.
    pub fn settled(&mut self, pane: &str, clean: bool) -> Step {
        if !self.waiting.contains(pane) {
            return Step::Waiting;
        }
        if !clean {
            return Step::Failed;
        }
        self.waiting.remove(pane);
        if self.waiting.is_empty() { Step::Done } else { Step::Waiting }
    }
}

/// The text on disk now, for the poll that notices outside changes.
pub fn peek(path: &Path) -> Option<String> {
    disk_now(path).ok().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = crate::test_tmp().join(format!("sunna-editor-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn decodes_text_and_refuses_binary_big_and_foreign_encodings() {
        assert_eq!(decode(b"fn main() {}\n".to_vec()), Ok("fn main() {}\n".into()));
        assert_eq!(decode(vec![0x89, b'P', b'N', b'G', 0, 1]), Err(LoadError::Binary));
        assert_eq!(decode(vec![0xff, 0xfe, b'a']), Err(LoadError::NotUtf8));
        let big = vec![b'a'; crate::mod_files::LIMIT as usize + 1];
        assert!(matches!(decode(big), Err(LoadError::TooLarge(_))));
        assert_eq!(decode(vec![b'a'; crate::mod_files::LIMIT as usize]).map(|t| t.len()), Ok(crate::mod_files::LIMIT as usize));
    }

    #[test]
    fn reads_local_files_without_loading_huge_ones() {
        let dir = scratch("read");
        let path = dir.join("a.rs");
        std::fs::write(&path, "let x = 1;\n").unwrap();
        assert_eq!(read_local(&path), Ok("let x = 1;\n".into()));
        assert_eq!(read_local(&dir.join("none.rs")), Err(LoadError::Missing));
        assert!(matches!(read_local(&dir), Err(LoadError::Io(_))));
        let big = dir.join("big.txt");
        std::fs::write(&big, vec![b'x'; crate::mod_files::LIMIT as usize + 10]).unwrap();
        assert!(matches!(read_local(&big), Err(LoadError::TooLarge(_))));
        let bin = dir.join("a.bin");
        std::fs::write(&bin, [1u8, 0, 2]).unwrap();
        assert_eq!(read_local(&bin), Err(LoadError::Binary));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn languages_follow_the_name() {
        assert_eq!(language_for("/a/b/main.rs"), "rs");
        assert_eq!(language_for("/a/App.TSX"), "js");
        assert_eq!(language_for("Cargo.toml"), "yaml");
        assert_eq!(language_for("C:\\x\\notes.md"), "md");
        assert_eq!(language_for("/x/Makefile"), "makefile");
        assert_eq!(language_for("/x/LICENSE"), "txt");
        assert!(is_markdown("/x/README.md") && is_markdown("/x/a.markdown") && !is_markdown("/x/a.rs"));
        assert_eq!(file_name("/a/b/c.txt"), "c.txt");
        assert_eq!(file_name("C:\\a\\c.txt"), "c.txt");
    }

    #[test]
    fn unsaved_means_different_from_what_was_read_and_undo_makes_it_clean_again() {
        let (mut doc, shown) = Doc::open("one\ntwo\n".into());
        assert!(!doc.dirty(&shown));
        assert!(doc.dirty("one\ntwo!\n"));
        assert!(!doc.dirty("one\ntwo\n"));
        doc.saved("one\ntwo!\n");
        assert!(!doc.dirty("one\ntwo!\n") && doc.dirty("one\ntwo\n"));
        assert_eq!(doc.baseline(), "one\ntwo!\n");
    }

    #[test]
    fn windows_line_ends_come_back_on_save_but_mixed_ones_stay() {
        let (doc, shown) = Doc::open("a\r\nb\r\n".into());
        assert_eq!(shown, "a\nb\n");
        assert_eq!(doc.disk_text("a\nb\nc\n"), "a\r\nb\r\nc\r\n");
        assert!(!doc.dirty("a\nb\n"));
        let (mixed, shown) = Doc::open("a\r\nb\n".into());
        assert_eq!(shown, "a\r\nb\n");
        assert_eq!(mixed.disk_text("x\n"), "x\n");
    }

    #[test]
    fn compares_the_disk_with_what_was_read() {
        let (doc, _) = Doc::open("hello".into());
        assert_eq!(doc.compare(Some("hello")), Disk::Same);
        assert_eq!(doc.compare(Some("hello!")), Disk::Changed("hello!".into()));
        assert_eq!(doc.compare(None), Disk::Gone);
    }

    #[test]
    fn saves_and_refuses_to_overwrite_outside_changes() {
        let dir = scratch("save");
        let path = dir.join("notes.txt");
        std::fs::write(&path, "v1\n").unwrap();
        let (mut doc, _) = Doc::open("v1\n".into());
        // A normal save writes and the next one is based on it.
        assert_eq!(save(&path, "v2\n", doc.baseline()), Saved::Written);
        doc.saved("v2\n");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "v2\n");
        // Someone else edits the file: the save is refused and the disk keeps their text.
        std::fs::write(&path, "theirs\n").unwrap();
        assert_eq!(save(&path, "v3\n", doc.baseline()), Saved::Conflict(Some("theirs\n".into())));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "theirs\n");
        // Overwriting needs the person's say, then it lands.
        assert_eq!(overwrite(&path, "v3\n"), Saved::Written);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "v3\n");
        // Deleted outside: a conflict, and overwriting makes the file again.
        std::fs::remove_file(&path).unwrap();
        assert_eq!(save(&path, "v4\n", "v3\n"), Saved::Conflict(None));
        assert!(!path.exists());
        assert_eq!(overwrite(&path, "v4\n"), Saved::Written);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "v4\n");
        assert_eq!(peek(&path), Some("v4\n".into()));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rebase_takes_the_disk_as_the_new_base() {
        let (mut doc, _) = Doc::open("a\n".into());
        let shown = doc.rebase("b\r\nc\r\n".into());
        assert_eq!(shown, "b\nc\n");
        assert_eq!(doc.baseline(), "b\r\nc\r\n");
        assert!(!doc.dirty("b\nc\n"));
    }

    #[test]
    fn save_and_close_waits_for_every_file_and_stops_at_the_first_failure() {
        let mut s = SaveThen::new(["a".to_owned(), "b".to_owned()], "tab.close", serde_json::json!({ "id": "t" }));
        assert!(s.waits_on("a") && !s.waits_on("z"));
        assert_eq!(s.settled("z", true), Step::Waiting);
        assert_eq!(s.settled("a", true), Step::Waiting);
        assert_eq!(s.settled("b", true), Step::Done);

        let mut s = SaveThen::new(["a".to_owned(), "b".to_owned()], "app.quit", serde_json::json!({}));
        assert_eq!(s.settled("b", false), Step::Failed);
        assert!(s.waits_on("a") && s.waits_on("b"));
    }

    #[test]
    fn a_conflict_leaves_the_file_unsaved_so_nothing_closes() {
        let dir = scratch("save-close-conflict");
        let file = dir.join("a.txt");
        std::fs::write(&file, "one\n").unwrap();
        std::fs::write(&file, "changed elsewhere\n").unwrap();
        // Saved from the text read ("one"): the disk moved on, so the save is refused.
        let outcome = save(&file, "mine\n", "one\n");
        assert!(matches!(outcome, Saved::Conflict(Some(_))), "{outcome:?}");
        let mut s = SaveThen::new(["a".to_owned()], "tab.close", serde_json::json!({}));
        assert_eq!(s.settled("a", outcome == Saved::Written), Step::Failed);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "changed elsewhere\n");
    }
}
