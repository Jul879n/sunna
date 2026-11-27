//! The Search panel (`{"native": "search"}`, once the `search` mod): text search in the focused
//! pane's folder (its repository's top when it is in one), results grouped by file, like Orca's.
//! ripgrep first, then `git grep` inside a repository, then `grep`. A hit opens in the editor on
//! its line (`file.open`). Parsing, the command lines and the grouping are plain functions
//! (tested at the end); the view only draws them.

use super::files::{Place, exec};
use super::native::{fade, icon, r, ring, step_cursor, tip, tone, z};
use super::*;
use crate::i18n;
use gpui::{UniformListScrollHandle, uniform_list};
use gpui_base::input::{Input, InputEvent, InputState};

const CLOSE: &[u8] = include_bytes!("../../assets/icons/pane/close.svg");
const ROW_H: f32 = 32.0;
pub(super) const MAX_HITS: usize = 500;
/// A line's text before the match is cut to this many characters (so the match stays in view).
const LEAD: usize = 32;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Hit {
    pub path: String,
    pub line: u64,
    pub text: String,
    pub start: Option<usize>,
    pub end: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Group {
    pub path: String,
    pub hits: Vec<Hit>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Grouped {
    pub groups: Vec<Group>,
    pub count: usize,
    pub truncated: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Options {
    pub case: bool,
    pub regex: bool,
    pub word: bool,
    pub glob: String,
}

// Pure rules ---------------------------------------------------------------------------------

fn clean_path(p: &str) -> String {
    p.strip_prefix("./").unwrap_or(p).to_owned()
}

/// `rg --json` output to hits (one per matching line).
pub(super) fn parse_rg(out: &str) -> Vec<Hit> {
    out.lines()
        .filter_map(|line| {
            let v: Value = serde_json::from_str(line).ok()?;
            if v["type"] != "match" {
                return None;
            }
            let d = &v["data"];
            let text = d["lines"]["text"].as_str().unwrap_or("").trim_end_matches(['\n', '\r']).to_owned();
            let sub = &d["submatches"][0];
            Some(Hit {
                path: clean_path(d["path"]["text"].as_str()?),
                line: d["line_number"].as_u64()?,
                text,
                start: sub["start"].as_u64().map(|n| n as usize),
                end: sub["end"].as_u64().map(|n| n as usize),
            })
        })
        .collect()
}

/// `path:line:text` (git grep, grep) to hits. The match is found again in the line, unless it is a regex.
pub(super) fn parse_grep(out: &str, query: &str, regex: bool) -> Vec<Hit> {
    out.lines()
        .filter_map(|row| {
            // The first `:<digits>:` ends the path (a path may hold a colon).
            let (path, line, text) = row.match_indices(':').find_map(|(i, _)| {
                let rest = &row[i + 1..];
                let digits = rest.find(|c: char| !c.is_ascii_digit())?;
                (digits > 0 && rest[digits..].starts_with(':')).then(|| (&row[..i], &rest[..digits], &rest[digits + 1..]))
            })?;
            let at = if regex || query.is_empty() { None } else { text.to_lowercase().find(&query.to_lowercase()).filter(|s| text.is_char_boundary(*s) && text.is_char_boundary(s + query.len().min(text.len() - s))) };
            Some(Hit { path: clean_path(path), line: line.parse().ok()?, text: text.to_owned(), start: at, end: at.map(|s| s + query.len()) })
        })
        .collect()
}

/// Hits grouped by file in the order they came, capped at `MAX_HITS`.
pub(super) fn group(hits: Vec<Hit>) -> Grouped {
    let truncated = hits.len() > MAX_HITS;
    let mut out = Grouped { truncated, ..Grouped::default() };
    let mut index: HashMap<String, usize> = HashMap::new();
    for hit in hits.into_iter().take(MAX_HITS) {
        let at = *index.entry(hit.path.clone()).or_insert_with(|| {
            out.groups.push(Group { path: hit.path.clone(), hits: Vec::new() });
            out.groups.len() - 1
        });
        out.groups[at].hits.push(hit);
        out.count += 1;
    }
    out
}

/// A line split around its match, leading blanks dropped; with no (valid) offsets the line is all "before".
pub(super) fn split_line(text: &str, start: Option<usize>, end: Option<usize>) -> (String, String, String) {
    let trimmed = text.trim_start();
    let cut = text.len() - trimmed.len();
    if let (Some(s), Some(e)) = (start, end) {
        let (s, e) = (s.saturating_sub(cut), e.saturating_sub(cut));
        if s <= e && e <= trimmed.len() && trimmed.is_char_boundary(s) && trimmed.is_char_boundary(e) {
            return (trimmed[..s].to_owned(), trimmed[s..e].to_owned(), trimmed[e..].to_owned());
        }
    }
    (trimmed.to_owned(), String::new(), String::new())
}

/// `before` with only its last `LEAD` characters, behind an ellipsis when it was longer.
pub(super) fn lead(before: &str) -> String {
    let n = before.chars().count();
    if n <= LEAD { before.to_owned() } else { format!("\u{2026}{}", before.chars().skip(n - LEAD).collect::<String>()) }
}

pub(super) fn rg_args(q: &str, o: &Options) -> Vec<String> {
    let mut a: Vec<String> = ["--json", "--max-count", "50", "--max-columns", "300", if o.case { "-s" } else { "-S" }].map(str::to_owned).into();
    if !o.regex {
        a.push("-F".into());
    }
    if o.word {
        a.push("-w".into());
    }
    if !o.glob.is_empty() {
        a.extend(["-g".into(), o.glob.clone()]);
    }
    a.extend(["--".into(), q.to_owned(), ".".into()]);
    a
}

fn grep_flags(o: &Options) -> Vec<String> {
    let mut f = Vec::new();
    if !o.case {
        f.push("-i".to_owned());
    }
    f.push(if o.regex { "-E" } else { "-F" }.to_owned());
    if o.word {
        f.push("-w".into());
    }
    f
}

pub(super) fn git_grep_args(q: &str, o: &Options) -> Vec<String> {
    let mut a: Vec<String> = ["grep", "-n", "-I", "--full-name"].map(str::to_owned).into();
    a.extend(grep_flags(o));
    a.extend(["-e".into(), q.to_owned()]);
    if !o.glob.is_empty() {
        a.extend(["--".into(), o.glob.clone()]);
    }
    a
}

pub(super) fn grep_args(q: &str, o: &Options) -> Vec<String> {
    let mut a: Vec<String> = ["-rnI", "--exclude-dir=.git", "--exclude-dir=node_modules", "--exclude-dir=target", "--exclude-dir=dist"].map(str::to_owned).into();
    a.extend(grep_flags(o));
    a.extend(["-e".into(), q.to_owned(), ".".into()]);
    a
}

/// A program that is not installed (as opposed to one that ran and failed).
fn missing(r: &Result<(i32, String, String), String>) -> bool {
    let has = |s: &str| {
        let s = s.to_lowercase();
        s.contains("not found") || s.contains("no such file") || s.contains("cannot run") || s.contains("cannot start")
    };
    match r {
        Ok((127, _, _)) => true,
        Ok((_, _, err)) => has(err),
        Err(e) => has(e),
    }
}

fn failure(err: &str, fallback: &str) -> String {
    err.lines().rev().find(|l| !l.trim().is_empty()).map_or_else(|| fallback.to_owned(), |l| l.trim().to_owned())
}

/// One search, from `cwd` (its repository's top when it has one): the root it searched and the hits.
fn run_search(q: &str, o: &Options, cwd: &str, on: Option<&str>) -> Result<(String, Vec<Hit>), String> {
    let (root, repo) = match exec(on, "git", &["rev-parse", "--show-toplevel"], cwd, 30) {
        Ok((0, top, _)) if !top.trim().is_empty() => (top.trim().to_owned(), true),
        _ => (cwd.to_owned(), false),
    };
    let call = |program: &str, args: Vec<String>| {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        exec(on, program, &refs, &root, 30)
    };
    let first = call("rg", rg_args(q, o));
    if !missing(&first) {
        let (code, out, err) = first?;
        return if code > 1 { Err(failure(&err, "rg failed")) } else { Ok((root, parse_rg(&out))) };
    }
    if repo {
        let (code, out, err) = call("git", git_grep_args(q, o))?;
        return if code > 1 { Err(failure(&err, "git grep failed")) } else { Ok((root, parse_grep(&out, q, o.regex))) };
    }
    if cfg!(windows) && on.is_none() {
        return Err(i18n::t("search.no_engine"));
    }
    let (code, out, err) = call("grep", grep_args(q, o))?;
    if code > 1 { Err(failure(&err, "grep failed")) } else { Ok((root, parse_grep(&out, q, o.regex))) }
}

#[derive(Clone, Debug, PartialEq)]
enum Row {
    File(usize),
    Hit(usize, usize),
}

fn rows_of(g: &Grouped) -> Vec<Row> {
    g.groups.iter().enumerate().flat_map(|(i, f)| std::iter::once(Row::File(i)).chain((0..f.hits.len()).map(move |j| Row::Hit(i, j)))).collect()
}

/// What `file.open` takes for a hit: the file under the searched root, its line, and its machine ("" is this computer).
fn open_args(root: &str, on: Option<&str>, hit: &Hit) -> Value {
    json!({ "path": format!("{root}/{}", hit.path), "line": hit.line, "on": on.unwrap_or_default() })
}

// The view -------------------------------------------------------------------------------------

pub(super) struct SearchView {
    app: WeakEntity<AppWindow>,
    place: Place,
    raw: Place,
    generation: u64,
    input: Entity<InputState>,
    glob_input: Entity<InputState>,
    typing: [bool; 2],
    options: Options,
    results: Option<Grouped>,
    rows: Vec<Row>,
    error: Option<String>,
    running: bool,
    root: Option<String>,
    /// The machine the results are from.
    on: Option<String>,
    scroll: UniformListScrollHandle,
    focus: FocusHandle,
    cursor: usize,
    list_focused: bool,
}

impl SearchView {
    pub(super) fn new(app: WeakEntity<AppWindow>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let make = |text: String, window: &mut Window, cx: &mut Context<Self>| {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder(text));
            input.update(cx, |s, _| s.set_editor_style(super::settings_ui::controls::input_style()));
            input
        };
        let input = make(i18n::t("search.placeholder"), window, cx);
        let glob_input = make(i18n::t("search.glob"), window, cx);
        for (n, state) in [&input, &glob_input].into_iter().enumerate() {
            cx.subscribe_in(state, window, move |this, _, ev: &InputEvent, _window, cx| match ev {
                InputEvent::PressEnter { .. } => this.search(cx),
                InputEvent::Focus => {
                    this.typing[n] = true;
                    cx.notify();
                }
                InputEvent::Blur => {
                    this.typing[n] = false;
                    cx.notify();
                }
                _ => {}
            })
            .detach();
        }
        Self {
            app, place: Place::default(), raw: Place::default(), generation: 0, input, glob_input, typing: [false; 2], options: Options::default(), results: None, rows: Vec::new(), error: None, running: false,
            root: None, on: None, scroll: UniformListScrollHandle::new(), focus: cx.focus_handle().tab_stop(true), cursor: 0, list_focused: false,
        }
    }

    /// Another pane's folder (or machine) clears the results: they were for the other place.
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
        self.results = None;
        self.rows.clear();
        self.error = None;
        self.running = false;
        self.cursor = 0;
        cx.notify();
    }

    fn search(&mut self, cx: &mut Context<Self>) {
        let query = self.input.read(cx).value().to_string();
        if query.is_empty() || self.running {
            return;
        }
        let Some(cwd) = self.place.cwd.clone() else {
            self.error = Some(i18n::t("search.no_folder"));
            cx.notify();
            return;
        };
        self.options.glob = self.glob_input.read(cx).value().trim().to_owned();
        let (options, generation, on) = (self.options.clone(), self.generation, self.place.on.clone());
        self.running = true;
        self.error = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let from = on.clone();
            let done = cx.background_executor().spawn(async move { run_search(&query, &options, &cwd, from.as_deref()) }).await;
            let _ = this.update(cx, |this, cx| {
                this.running = false;
                if this.generation == generation {
                    match done {
                        Ok((root, hits)) => {
                            let grouped = group(hits);
                            this.rows = rows_of(&grouped);
                            this.results = Some(grouped);
                            this.root = Some(root);
                            this.on = on;
                            this.error = None;
                        }
                        Err(e) => {
                            this.results = None;
                            this.rows.clear();
                            this.error = Some(e);
                        }
                    }
                    this.cursor = 0;
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn hit_at(&self, row: &Row) -> Option<&Hit> {
        let g = self.results.as_ref()?;
        match row {
            Row::File(i) => g.groups.get(*i)?.hits.first(),
            Row::Hit(i, j) => g.groups.get(*i)?.hits.get(*j),
        }
    }

    /// The editor opens the file on the hit's line, in its own tab.
    fn open(&self, row: &Row, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(root), Some(hit)) = (self.root.as_deref(), self.hit_at(row)) else { return };
        let args = open_args(root, self.on.as_deref(), hit);
        let _ = self.app.update(cx, |app, cx| app.run("file.open", args, window, cx));
    }

    fn press(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if let Some(to) = step_cursor(self.cursor, key, self.rows.len()) {
            self.cursor = to;
            self.scroll.scroll_to_item(to, gpui::ScrollStrategy::Nearest);
            cx.notify();
            return true;
        }
        match (key, self.rows.get(self.cursor).cloned()) {
            ("enter" | "space", Some(row)) => self.open(&row, window, cx),
            _ => return false,
        }
        true
    }

    fn toggle(&self, id: &'static str, label: &'static str, tooltip: String, on: bool, flip: fn(&mut Options), cx: &mut Context<Self>) -> Stateful<Div> {
        div()
            .id(id)
            .size(z(ROW_H))
            .min_w(z(ROW_H))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(r(6.0))
            .border_1()
            .border_color(c(if on { "accent" } else { "border" }))
            .bg(c(if on { "selected" } else { "background" }))
            .text_size(z(12.0))
            .text_color(if on { c("foreground") } else { tone("muted") })
            .cursor_pointer()
            .hover(|s| s.bg(c("hover")))
            .focusable()
            .tab_stop(true)
            .focus_visible(|s| s.border_color(c("foreground")))
            .tooltip(super::native::tip(tooltip))
            .on_key_down(cx.listener(move |this, e: &KeyDownEvent, _, cx| {
                if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                    flip(&mut this.options);
                    this.search(cx);
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                flip(&mut this.options);
                this.search(cx);
                cx.notify();
            }))
            .child(label)
    }

    fn field(&self, id: &'static str, state: &Entity<InputState>, typing: bool) -> Stateful<Div> {
        div()
            .id(id)
            .flex_1()
            .min_w(px(0.0))
            .h(z(ROW_H))
            .px(z(10.0))
            .flex()
            .items_center()
            .rounded(r(8.0))
            .border_1()
            .border_color(c(if typing { "accent" } else { "border" }))
            .cursor_text()
            .text_size(z(12.0))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(Input::new(state))
    }

    fn row(&self, i: usize, cx: &mut Context<Self>) -> AnyElement {
        let (Some(row), Some(g)) = (self.rows.get(i).cloned(), self.results.as_ref()) else { return div().into_any_element() };
        let on_cursor = self.list_focused && self.cursor == i;
        let click = row.clone();
        let base = |id: SharedString| {
            ring(div().id(id), on_cursor)
                .h(z(ROW_H))
                .w_full()
                .flex_none()
                .flex()
                .items_center()
                .px(z(8.0))
                .gap(z(8.0))
                .rounded(r(7.0))
                .cursor_pointer()
                .hover(|s| s.bg(c("hover")))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.cursor = i;
                    this.focus.focus(window, cx);
                    this.open(&click, window, cx);
                }))
        };
        match row {
            Row::File(f) => {
                let path = &g.groups[f].path;
                let (dir, name) = path.rsplit_once('/').map_or(("", path.as_str()), |(d, n)| (d, n));
                base(SharedString::from(format!("search-file-{path}")))
                    .text_size(z(12.0))
                    .child(div().id("hit-name").flex_none().max_w(relative(0.6)).truncate().font_weight(FontWeight::SEMIBOLD).text_color(c("foreground")).tooltip(tip(name.to_string())).child(name.to_owned()))
                    .child(div().id("hit-dir").flex_1().min_w(px(0.0)).truncate().text_color(tone("muted")).tooltip(tip(dir.to_string())).child(dir.to_owned()))
                    .into_any_element()
            }
            Row::Hit(f, h) => {
                let hit = &g.groups[f].hits[h];
                let (before, found, after) = split_line(&hit.text, hit.start, hit.end);
                base(SharedString::from(format!("search-hit-{}-{}", hit.path, hit.line)))
                    .pl(z(16.0))
                    .child(div().w(z(34.0)).flex_none().text_size(z(11.0)).text_color(tone("muted")).child(hit.line.to_string()))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .overflow_hidden()
                            .flex()
                            .whitespace_nowrap()
                            .text_size(z(12.0))
                            .child(div().flex_none().text_color(tone("muted")).child(lead(&before)))
                            .child(div().flex_none().font_weight(FontWeight::SEMIBOLD).text_color(tone("accent")).child(found))
                            .child(div().flex_none().text_color(tone("muted")).child(after)),
                    )
                    .into_any_element()
            }
        }
    }
}

impl Render for SearchView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.list_focused = self.focus.is_focused(window);
        self.cursor = self.cursor.min(self.rows.len().saturating_sub(1));
        let (count, files) = self.results.as_ref().map_or((0, 0), |g| (g.count, g.groups.len()));
        let note = match (&self.error, self.running, &self.results, &self.place.cwd) {
            (Some(e), _, _, _) => Some((e.clone(), true)),
            (None, true, _, _) => Some((i18n::t("search.searching"), false)),
            (None, false, Some(g), _) if g.count == 0 => Some((i18n::t("search.empty"), false)),
            (None, false, None, None) => Some((i18n::t("search.no_folder"), false)),
            (None, false, None, Some(_)) => Some((i18n::t("search.hint"), false)),
            _ => None,
        };
        let truncated = self.results.as_ref().is_some_and(|g| g.truncated);
        let total = self.rows.len();
        let (query_on, glob_on) = (self.typing[0], self.typing[1]);
        div()
            .id("search")
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
                    .pl(z(14.0))
                    .pr(z(5.0))
                    .gap(z(8.0))
                    .flex()
                    .items_center()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(i18n::t("tabs.search")))
                    .child(div().flex_1().min_w(px(0.0)).truncate().text_right().text_size(z(11.0)).text_color(tone("muted")).child(if self.results.as_ref().is_some_and(|g| g.count > 0) { format!("{} {}", i18n::tn("search.matches", count, &[("count", &count.to_string())]), i18n::tn("search.in_files", files, &[("files", &files.to_string())])) } else { String::new() }))
                    .child(
                        div()
                            .id("search-close")
                            .size(z(ROW_H))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(r(6.0))
                            .cursor_pointer()
                            .border_1()
                            .border_color(Hsla { a: 0.0, ..c("accent") })
                            .hover(|s| s.bg(c("hover")))
                            .focusable()
                            .tab_stop(true)
                            .focus_visible(|s| s.border_color(graphic("accent")))
                            .tooltip(super::native::tip(i18n::t("menu.close")))
                            .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                                    let _ = this.app.update(cx, |app, cx| app.run("layout.hide", json!({ "id": "panel" }), window, cx));
                                    cx.stop_propagation();
                                }
                            }))
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                let _ = this.app.update(cx, |app, cx| app.run("layout.hide", json!({ "id": "panel" }), window, cx));
                            }))
                            .child(icon(CLOSE, 13.0, tone("muted"))),
                    ),
            )
            .child(fade(8.0).my(z(0.0)))
            .child(
                div()
                    .flex_none()
                    .p(z(10.0))
                    .gap(z(8.0))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .gap(z(6.0))
                            .child(self.field("search-query", &self.input, query_on))
                            .child(self.toggle("search-case", "Aa", i18n::t("search.case"), self.options.case, |o| o.case = !o.case, cx))
                            .child(self.toggle("search-regex", ".*", i18n::t("search.regex"), self.options.regex, |o| o.regex = !o.regex, cx))
                            .child(self.toggle("search-word", "W", i18n::t("search.word"), self.options.word, |o| o.word = !o.word, cx)),
                    )
                    .child(div().flex().child(self.field("search-glob", &self.glob_input, glob_on))),
            )
            .child(fade(8.0).my(z(0.0)))
            .when_some(note, |d, (text, error)| d.child(div().px(z(14.0)).py(z(8.0)).text_size(z(12.0)).text_color(tone(if error { "ansi1" } else { "muted" })).child(text)))
            .child(
                div()
                    .id("search-list-focus")
                    .track_focus(&self.focus)
                    .flex_1()
                    .min_h(px(0.0))
                    .p(z(6.0))
                    .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                        if this.press(e.keystroke.key.as_str(), window, cx) {
                            cx.stop_propagation();
                        }
                    }))
                    .child(uniform_list("search-list", total, cx.processor(|this, range: std::ops::Range<usize>, _window, cx| range.map(|i| this.row(i, cx)).collect::<Vec<_>>())).track_scroll(&self.scroll).size_full()),
            )
            .when(truncated, |d| d.child(div().px(z(14.0)).py(z(8.0)).text_size(z(12.0)).text_color(tone("muted")).child(i18n::tf("search.truncated", &[("count", &MAX_HITS.to_string())]))))
    }
}

#[cfg(test)]
mod tests {
    use super::{Hit, MAX_HITS, Options, Row, git_grep_args, grep_args, group, lead, missing, open_args, parse_grep, parse_rg, rg_args, rows_of, split_line};

    fn hit(path: &str, line: u64) -> Hit {
        Hit { path: path.into(), line, text: "x".into(), start: None, end: None }
    }

    #[test]
    fn ripgreps_json_becomes_hits_without_the_dot_slash() {
        let out = concat!(
            r#"{"type":"begin","data":{"path":{"text":"./a.rs"}}}"#, "\n",
            r#"{"type":"match","data":{"path":{"text":"./src/a.rs"},"lines":{"text":"  let x = hola;\n"},"line_number":7,"submatches":[{"match":{"text":"hola"},"start":10,"end":14}]}}"#, "\n",
            "not json\n",
            r#"{"type":"end","data":{}}"#, "\n",
        );
        assert_eq!(parse_rg(out), vec![Hit { path: "src/a.rs".into(), line: 7, text: "  let x = hola;".into(), start: Some(10), end: Some(14) }]);
        assert!(parse_rg("").is_empty());
    }

    #[test]
    fn grep_rows_split_at_the_first_line_number_and_find_the_match() {
        let hits = parse_grep("./a:b.txt:12:tres Hola mundo\nsrc/m.rs:3:x:4:y\nbroken line\n", "hola", false);
        assert_eq!(hits.len(), 2);
        assert_eq!((hits[0].path.as_str(), hits[0].line, hits[0].start, hits[0].end), ("a:b.txt", 12, Some(5), Some(9)));
        assert_eq!((hits[1].path.as_str(), hits[1].line, hits[1].text.as_str()), ("src/m.rs", 3, "x:4:y"));
        // A regex has no offsets to find again.
        assert_eq!(parse_grep("a:1:foo", "fo+", true)[0].start, None);
    }

    #[test]
    fn hits_group_by_file_in_order_and_stop_at_the_cap() {
        let g = group(vec![hit("b", 1), hit("a", 2), hit("b", 3)]);
        assert_eq!(g.groups.iter().map(|f| (f.path.as_str(), f.hits.len())).collect::<Vec<_>>(), [("b", 2), ("a", 1)]);
        assert_eq!((g.count, g.truncated), (3, false));
        let many = group((0..MAX_HITS as u64 + 5).map(|i| hit(&format!("f{}", i % 7), i)).collect());
        assert_eq!((many.count, many.truncated), (MAX_HITS, true));
        assert_eq!(rows_of(&g), vec![Row::File(0), Row::Hit(0, 0), Row::Hit(0, 1), Row::File(1), Row::Hit(1, 0)]);
    }

    #[test]
    fn a_line_splits_around_its_match_after_dropping_the_indent() {
        assert_eq!(split_line("    foo(bar)", Some(8), Some(11)), ("foo(".into(), "bar".into(), ")".into()));
        // Offsets inside a multibyte character, or past the end, leave the whole line.
        assert_eq!(split_line("ñandú", Some(1), Some(3)), ("ñandú".into(), String::new(), String::new()));
        assert_eq!(split_line("ab", Some(1), Some(9)).1, "");
        assert_eq!(split_line("  ab", None, None), ("ab".into(), String::new(), String::new()));
    }

    #[test]
    fn a_long_lead_is_cut_to_keep_the_match_visible() {
        assert_eq!(lead("short"), "short");
        let long = "x".repeat(40);
        assert_eq!(lead(&long), format!("\u{2026}{}", "x".repeat(32)));
    }

    #[test]
    fn command_lines_follow_the_options() {
        let plain = Options::default();
        assert_eq!(rg_args("q", &plain), ["--json", "--max-count", "50", "--max-columns", "300", "-S", "-F", "--", "q", "."]);
        let all = Options { case: true, regex: true, word: true, glob: "*.rs".into() };
        assert_eq!(rg_args("q", &all), ["--json", "--max-count", "50", "--max-columns", "300", "-s", "-w", "-g", "*.rs", "--", "q", "."]);
        assert_eq!(git_grep_args("q", &plain), ["grep", "-n", "-I", "--full-name", "-i", "-F", "-e", "q"]);
        assert_eq!(git_grep_args("q", &all), ["grep", "-n", "-I", "--full-name", "-E", "-w", "-e", "q", "--", "*.rs"]);
        assert!(grep_args("q", &plain).ends_with(&["-i".into(), "-F".into(), "-e".into(), "q".into(), ".".into()]));
    }

    #[test]
    fn a_program_that_is_not_installed_is_told_from_one_that_failed() {
        assert!(missing(&Ok((127, String::new(), String::new()))));
        assert!(missing(&Err("cannot run rg: No such file or directory".into())));
        assert!(!missing(&Ok((2, String::new(), "regex parse error".into()))));
        assert!(!missing(&Ok((1, String::new(), String::new()))));
    }

    #[test]
    fn a_hit_opens_under_the_root_on_its_line_and_machine() {
        let h = Hit { line: 9, ..hit("src/a.rs", 9) };
        assert_eq!(open_args("/r", None, &h), serde_json::json!({ "path": "/r/src/a.rs", "line": 9, "on": "" }));
        assert_eq!(open_args("/r", Some("vps"), &h)["on"], "vps");
    }

    #[test]
    fn a_real_search_finds_the_text_in_a_temporary_folder() {
        let dir = std::env::temp_dir().join(format!("sunna-search-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.txt"), "uno\nHola mundo\n").unwrap();
        std::fs::write(dir.join("sub/b.txt"), "nada\nhola otra vez\n").unwrap();
        let root = dir.to_string_lossy().into_owned();
        let (_, hits) = super::run_search("hola", &Options::default(), &root, None).unwrap();
        let g = group(hits);
        assert_eq!(g.count, 2, "{g:?}");
        let (_, exact) = super::run_search("hola", &Options { case: true, ..Options::default() }, &root, None).unwrap();
        assert_eq!(exact.len(), 1);
        let (_, none) = super::run_search("zzz", &Options::default(), &root, None).unwrap();
        assert!(none.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
