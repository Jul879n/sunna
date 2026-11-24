//! Importing a color theme: Ghostty, Warp and VS Code (or Cursor) theme files
//! become Sunna's `themes/<name>.json`.

use crate::i18n;
use crate::settings::UiStyle;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// `themes/<name>.json`: what the app reads (see `settings::parse_theme_file`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ThemeDoc {
    pub background: String,
    pub foreground: String,
    pub selection: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    pub ansi: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub colors: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "style_is_default")]
    pub style: UiStyle,
}

fn style_is_default(s: &UiStyle) -> bool {
    *s == UiStyle::default()
}

impl ThemeDoc {
    /// The file's text.
    pub fn to_file(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default() + "\n"
    }
}

/// Why a file can't become a theme: a locale key and its variables.
#[derive(Debug, PartialEq)]
pub struct Failure {
    pub key: &'static str,
    pub vars: Vec<(&'static str, String)>,
}

impl Failure {
    pub fn text(&self) -> String {
        let vars: Vec<(&str, &str)> = self.vars.iter().map(|(k, v)| (*k, v.as_str())).collect();
        i18n::tf(self.key, &vars)
    }
}

fn fail(key: &'static str) -> Failure {
    Failure { key, vars: Vec::new() }
}

/// `#rgb` / `#rrggbb` / `#rrggbbaa` (with or without `#`) as lowercase `#rrggbb[aa]`.
pub fn hex(v: &str) -> Option<String> {
    let mut h = v.trim().trim_start_matches('#').to_lowercase();
    if !h.is_ascii() {
        return None;
    }
    if h.len() == 3 || h.len() == 4 {
        h = h.chars().flat_map(|c| [c, c]).collect();
    }
    ((h.len() == 6 || h.len() == 8) && h.chars().all(|c| c.is_ascii_hexdigit())).then(|| format!("#{h}"))
}

/// Terminal keys take exactly 6 digits: an alpha channel is dropped.
fn term(v: Option<&str>) -> Option<String> {
    hex(v?).map(|h| h[..7].to_owned())
}

const ANSI_NAMES: [&str; 8] = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];

struct Parts<'a> {
    background: Option<&'a str>,
    foreground: Option<&'a str>,
    selection: Option<&'a str>,
    cursor: Option<&'a str>,
    ansi: Vec<Option<String>>,
    colors: Vec<(&'static str, Option<&'a str>)>,
}

fn finish(p: Parts) -> Result<ThemeDoc, Failure> {
    let mut ansi = Vec::new();
    for (i, v) in p.ansi.into_iter().enumerate() {
        ansi.push(v.ok_or_else(|| Failure { key: "settings.import_missing_ansi", vars: vec![("index", i.to_string())] })?);
    }
    let background = term(p.background).ok_or_else(|| fail("settings.import_no_background"))?;
    let doc = ThemeDoc {
        background,
        foreground: term(p.foreground).unwrap_or_else(|| ansi[7].clone()),
        selection: term(p.selection).unwrap_or_else(|| ansi[8].clone()),
        cursor: term(p.cursor),
        colors: p.colors.into_iter().filter_map(|(k, v)| Some((k.to_owned(), hex(v?)?))).collect(),
        ansi,
        style: UiStyle::default(),
    };
    Ok(doc)
}

pub fn from_ghostty(text: &str) -> Result<ThemeDoc, Failure> {
    let mut ansi: Vec<Option<String>> = vec![None; 16];
    let mut o: BTreeMap<String, String> = BTreeMap::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let (key, value) = (key.trim(), value.trim());
        if key == "palette" {
            if let Some((n, color)) = value.split_once('=')
                && let Ok(n) = n.trim().parse::<usize>()
                && n < 16
            {
                ansi[n] = term(color.split_whitespace().next());
            }
        } else {
            o.insert(key.to_owned(), value.to_owned());
        }
    }
    let get = |k: &str| o.get(k).map(String::as_str);
    finish(Parts { background: get("background"), foreground: get("foreground"), selection: get("selection-background"), cursor: get("cursor-color"), ansi, colors: vec![] })
}

/// A minimal YAML reader: nested `key: value` by indentation.
fn yaml(text: &str) -> Value {
    let mut root = serde_json::Map::new();
    let mut stack: Vec<(isize, Vec<String>)> = vec![(-1, vec![])];
    for raw in text.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = (raw.len() - raw.trim_start_matches(' ').len()) as isize;
        let Some((key, value)) = trimmed.split_once(':') else { continue };
        let key = key.trim();
        if key.is_empty() || !key.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '-')) {
            continue;
        }
        while stack.len() > 1 && stack.last().is_some_and(|(i, _)| *i >= indent) {
            stack.pop();
        }
        let path = stack.last().map(|(_, p)| p.clone()).unwrap_or_default();
        let mut value = value.trim().to_owned();
        if let Some(at) = value.find(" #").or_else(|| value.find("\t#")) {
            value.truncate(at);
        }
        let mut value = value.trim().to_owned();
        for q in ['\'', '"'] {
            if value.len() >= 2 && value.starts_with(q) && value.ends_with(q) {
                value = value[1..value.len() - 1].to_owned();
            }
        }
        let mut node = &mut root;
        for k in &path {
            node = node.get_mut(k).and_then(Value::as_object_mut).expect("a parent exists");
        }
        if value.is_empty() {
            node.insert(key.to_owned(), Value::Object(Default::default()));
            let mut next = path;
            next.push(key.to_owned());
            stack.push((indent, next));
        } else {
            node.insert(key.to_owned(), Value::String(value));
        }
    }
    Value::Object(root)
}

pub fn from_warp(text: &str) -> Result<ThemeDoc, Failure> {
    let y = yaml(text);
    let s = |v: &Value| v.as_str().map(str::to_owned);
    let own = |k: &str| s(&y[k]).or_else(|| s(&y[k]["color"]));
    let mut ansi = vec![None; 16];
    for (i, n) in ANSI_NAMES.iter().enumerate() {
        ansi[i] = term(y["terminal_colors"]["normal"][n].as_str());
        ansi[i + 8] = term(y["terminal_colors"]["bright"][n].as_str());
    }
    let (background, accent) = (own("background"), own("accent"));
    finish(Parts {
        background: background.as_deref(),
        foreground: y["foreground"].as_str(),
        selection: None,
        cursor: y["cursor"].as_str(),
        ansi,
        colors: vec![("accent", accent.as_deref())],
    })
}

/// JSON with comments and trailing commas (VS Code's flavor) as plain JSON.
fn relax_json(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '"' {
                j += if chars[j] == '\\' { 2 } else { 1 };
            }
            out.extend(&chars[i..(j + 1).min(chars.len())]);
            i = j;
        } else if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            out.push('\n');
            continue;
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            let end = (i + 2..chars.len().saturating_sub(1)).find(|&k| chars[k] == '*' && chars[k + 1] == '/');
            i = end.map_or(chars.len(), |e| e + 1);
        } else {
            out.push(c);
        }
        i += 1;
    }
    let out = out.trim_start_matches('\u{feff}');
    fancy_regex::Regex::new(r",(\s*[}\]])").expect("static pattern").replace_all(out, "$1").into_owned()
}

pub fn from_vscode(text: &str) -> Result<ThemeDoc, Failure> {
    let doc: Value = serde_json::from_str(&relax_json(text)).map_err(|e| Failure { key: "settings.import_failed", vars: vec![("error", e.to_string())] })?;
    let c = &doc["colors"];
    let get = |k: &str| c[k].as_str();
    let cap = |s: &str| format!("{}{}", s[..1].to_uppercase(), &s[1..]);
    let mut ansi = Vec::new();
    for n in ANSI_NAMES {
        ansi.push(term(get(&format!("terminal.ansi{}", cap(n)))));
    }
    for n in ANSI_NAMES {
        ansi.push(term(get(&format!("terminal.ansiBright{}", cap(n)))));
    }
    finish(Parts {
        background: get("terminal.background").or(get("editor.background")),
        foreground: get("terminal.foreground").or(get("editor.foreground")),
        selection: get("terminal.selectionBackground").or(get("editor.selectionBackground")),
        cursor: get("terminalCursor.foreground"),
        ansi,
        colors: vec![("accent", get("focusBorder").or(get("button.background"))), ("chrome", get("sideBar.background")), ("border", get("panel.border").or(get("editorGroup.border")))],
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Format {
    Ghostty,
    Warp,
    VsCode,
}

/// The importer for a file name: `.json` VS Code, `.yaml`/`.yml` Warp, anything else Ghostty.
pub fn detect(file_name: &str) -> Format {
    let n = file_name.to_lowercase();
    if n.ends_with(".json") {
        Format::VsCode
    } else if n.ends_with(".yaml") || n.ends_with(".yml") {
        Format::Warp
    } else {
        Format::Ghostty
    }
}

pub fn parse(file_name: &str, text: &str) -> Result<ThemeDoc, Failure> {
    match detect(file_name) {
        Format::Ghostty => from_ghostty(text),
        Format::Warp => from_warp(text),
        Format::VsCode => from_vscode(text),
    }
}

/// A file name as a theme name: lowercase words joined by `-`.
pub fn slug(name: &str) -> String {
    let words: String = name.trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join("-");
    let kept: String = words.chars().filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-').collect();
    kept.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-")
}

/// A theme name that doesn't exist yet: `base`, `base-2`, `base-3`...
pub fn free_name(base: &str, taken: &[String]) -> String {
    let base = if base.is_empty() { "theme" } else { base };
    let mut n = base.to_owned();
    let mut i = 2;
    while taken.contains(&n) {
        n = format!("{base}-{i}");
        i += 1;
    }
    n
}

/// A place theme files live: its label and what it holds (name, path).
#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    pub id: String,
    pub label: String,
    pub files: Vec<(String, PathBuf)>,
}

fn names(dir: &Path, dirs: bool) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.metadata().is_ok_and(|m| m.is_dir() == dirs) || e.path().is_dir() == dirs)
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| !n.starts_with('.'))
        .collect();
    out.sort();
    out
}

fn files_in(dir: &Path, keep: impl Fn(&str) -> bool) -> Vec<(String, PathBuf)> {
    names(dir, false).into_iter().filter(|n| keep(n)).map(|n| (n.clone(), dir.join(n))).collect()
}

/// Where themes of other terminals are usually kept, plus the folders picked by hand.
pub fn scan(home: &Path, extra: &[PathBuf]) -> Vec<Source> {
    let mut found = Vec::new();
    let mut add = |id: String, label: String, files: Vec<(String, PathBuf)>| {
        if !files.is_empty() {
            found.push(Source { id, label, files });
        }
    };
    let mut ghostty = vec![home.join(".config/ghostty/themes")];
    if cfg!(target_os = "macos") {
        ghostty.push(PathBuf::from("/Applications/Ghostty.app/Contents/Resources/ghostty/themes"));
    }
    if cfg!(target_os = "linux") {
        ghostty.push(PathBuf::from("/usr/share/ghostty/themes"));
    }
    for dir in ghostty {
        add(format!("ghostty:{}", dir.display()), format!("Ghostty · {}", dir.display()), files_in(&dir, |_| true));
    }
    let yaml = |n: &str| n.to_lowercase().ends_with(".yaml") || n.to_lowercase().ends_with(".yml");
    let warp = home.join(".warp/themes");
    let mut warp_files = files_in(&warp, yaml);
    for sub in names(&warp, true) {
        warp_files.extend(files_in(&warp.join(sub), yaml));
    }
    add("warp".into(), format!("Warp · {}", warp.display()), warp_files);
    for (label, root) in [("VS Code", home.join(".vscode/extensions")), ("Cursor", home.join(".cursor/extensions"))] {
        let mut files = Vec::new();
        for ext in names(&root, true) {
            files.extend(files_in(&root.join(ext).join("themes"), |n| n.to_lowercase().ends_with(".json")));
        }
        add(label.to_owned(), format!("{label} · {}", root.display()), files);
    }
    for dir in extra {
        add(format!("folder:{}", dir.display()), dir.display().to_string(), files_in(dir, |_| true));
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const DRACULA: &str = "palette = 0=#21222c\npalette = 1=#ff5555\npalette = 2=#50fa7b\npalette = 3=#f1fa8c\npalette = 4=#bd93f9\npalette = 5=#ff79c6\npalette = 6=#8be9fd\npalette = 7=#f8f8f2\npalette = 8=#6272a4\npalette = 9=#ff6e6e\npalette = 10=#69ff94\npalette = 11=#ffffa5\npalette = 12=#d6acff\npalette = 13=#ff92df\npalette = 14=#a4ffff\npalette = 15=#ffffff\nbackground = #282a36\nforeground = #f8f8f2\ncursor-color = #f8f8f2\ncursor-text = #282a36\nselection-background = #44475a\nselection-foreground = #ffffff\n";

    #[test]
    fn ghostty_the_dracula_file() {
        let t = from_ghostty(DRACULA).unwrap();
        assert_eq!(t.background, "#282a36");
        assert_eq!(t.ansi[4], "#bd93f9");
        assert_eq!(t.selection, "#44475a");
        assert_eq!(t.cursor.as_deref(), Some("#f8f8f2"));
        assert_eq!(t.ansi.len(), 16);
        assert_eq!(serde_json::from_str::<ThemeDoc>(&t.to_file()).unwrap(), t);
    }

    #[test]
    fn warp_normal_and_bright_make_16_ansi_and_accent_goes_to_colors() {
        let block = |base: u32| ANSI_NAMES.iter().enumerate().map(|(i, c)| format!("    {c}: '#{0:02x}{0:02x}{0:02x}'", base + i as u32)).collect::<Vec<_>>().join("\n");
        let text = format!("# a comment\naccent: \"#ff8800\"\nbackground: '#101010'\nforeground: '#eeeeee'\nterminal_colors:\n  normal:\n{}\n  bright:\n{}\n", block(16), block(32));
        let t = from_warp(&text).unwrap();
        assert_eq!(t.ansi.len(), 16);
        assert_eq!(t.ansi[0], "#101010");
        assert_eq!(t.ansi[8], "#202020");
        assert_eq!(t.colors["accent"], "#ff8800");
        assert_eq!(t.selection, t.ansi[8]);
    }

    fn vscode(omit_ansi: bool) -> String {
        let names = ["Black", "Red", "Green", "Yellow", "Blue", "Magenta", "Cyan", "White"];
        let ansi = if omit_ansi { String::new() } else { names.iter().enumerate().map(|(i, n)| format!("\"terminal.ansi{n}\": \"#0{i}0000\", \"terminal.ansiBright{n}\": \"#1{i}0000\",")).collect::<Vec<_>>().join("\n") };
        format!("{{\n  // a theme\n  \"name\": \"x\", /* block */\n  \"colors\": {{\n    \"editor.background\": \"#1e1e1e\", \"editor.foreground\": \"#ccc\",\n    \"focusBorder\": \"#007acc88\", \"sideBar.background\": \"#252526\",\n    {ansi}\n  }},\n}}")
    }

    #[test]
    fn vscode_comments_and_trailing_commas() {
        let t = from_vscode(&vscode(false)).unwrap();
        assert_eq!(t.background, "#1e1e1e");
        assert_eq!(t.foreground, "#cccccc");
        assert_eq!(t.ansi.len(), 16);
        assert_eq!(t.ansi[9], "#110000");
        assert_eq!(t.colors["accent"], "#007acc88");
        assert_eq!(t.colors["chrome"], "#252526");
    }

    #[test]
    fn vscode_without_terminal_ansi_asks_for_the_missing_color() {
        let e = from_vscode(&vscode(true)).unwrap_err();
        assert_eq!(e.key, "settings.import_missing_ansi");
        assert_eq!(e.vars, vec![("index", "0".to_owned())]);
        assert_eq!(from_ghostty("palette = 0=#000000\n").unwrap_err().key, "settings.import_missing_ansi");
    }

    #[test]
    fn a_theme_without_background_says_so() {
        let text = DRACULA.replace("background = #282a36\n", "");
        assert_eq!(from_ghostty(&text).unwrap_err().key, "settings.import_no_background");
    }

    #[test]
    fn helpers() {
        assert_eq!(detect("a.JSON"), Format::VsCode);
        assert_eq!(detect("a.yml"), Format::Warp);
        assert_eq!(detect("Dracula"), Format::Ghostty);
        assert_eq!(slug("Tokyo Night Day!"), "tokyo-night-day");
        assert_eq!(slug("  --A  b-- "), "a-b");
        assert_eq!(hex("#abc").as_deref(), Some("#aabbcc"));
        assert_eq!(hex("FFAA0080").as_deref(), Some("#ffaa0080"));
        assert_eq!(hex("#12345"), None);
        assert_eq!(free_name("x", &["x".into(), "x-2".into()]), "x-3");
        assert_eq!(free_name("", &[]), "theme");
    }

    #[test]
    fn the_file_keeps_the_order_the_app_writes() {
        let t = from_ghostty(DRACULA).unwrap();
        let text = t.to_file();
        let at = |k: &str| text.find(&format!("\"{k}\"")).unwrap();
        assert!(at("background") < at("foreground") && at("foreground") < at("selection") && at("selection") < at("cursor") && at("cursor") < at("ansi"));
        assert!(!text.contains("style"));
    }
}
