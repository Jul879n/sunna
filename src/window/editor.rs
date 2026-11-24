//! Files as panes: a code editor (highlighting, line numbers, folding, find and
//! replace from gpui-base's `Editor`) that reads and writes the file itself.
//! A pane is a mod pane named `editor` with `props: { file, on? }`, so tabs,
//! splits and saved layouts need nothing new; this module draws it natively and
//! keeps its state (`editor.rs` is the pure part: reading, conflicts, dirty).

use super::*;
use crate::editor::{self, Disk, Doc, LoadError, Saved};
use gpui_base::input::{Editor, EditorState, InputBase, InputEvent, Position};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The mod name that means "a native file editor" in a layout or `tab.new`.
pub const NAME: &str = "editor";

/// How many files have unsaved changes (the quit handlers ask before leaving).
pub static UNSAVED: AtomicUsize = AtomicUsize::new(0);

const POLL: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, PartialEq)]
pub struct Look {
    size: f32,
    numbers: bool,
    folding: bool,
    wrap: bool,
}

/// Settings > Editor, as `shared()` keys.
fn look() -> Look {
    let flag = |k: &str, d: bool| mods::shared(k).and_then(|v| v.as_bool()).unwrap_or(d);
    Look {
        size: mods::shared("editor.font_size").and_then(|v| v.as_f64()).map_or(12.0, |s| s.clamp(8.0, 32.0)) as f32,
        numbers: flag("editor.line_numbers", true),
        folding: flag("editor.folding", true),
        wrap: flag("editor.wrap", false),
    }
}

enum Phase {
    Loading,
    Failed(String),
    Ready,
}

enum Note {
    /// The file changed (its text now) or was deleted (None) since it was read.
    Conflict(Option<String>),
    Error(String),
}

pub struct EditorPane {
    path: String,
    on: Option<String>,
    phase: Phase,
    state: Option<Entity<EditorState>>,
    doc: Option<Doc>,
    dirty: bool,
    preview: bool,
    note: Option<Note>,
    saving: bool,
    /// Bumped by each read, so an old one that finishes late is ignored.
    generation: u64,
    look: Look,
    _sub: Option<Subscription>,
}

impl EditorPane {
    fn new(path: String, on: Option<String>) -> EditorPane {
        EditorPane { path, on, phase: Phase::Loading, state: None, doc: None, dirty: false, preview: false, note: None, saving: false, generation: 0, look: look(), _sub: None }
    }

    fn readonly(&self) -> bool {
        self.on.is_some()
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

/// `props` of an editor pane: its file and the machine it is on.
fn file_props(props: &Value) -> Option<(String, Option<String>)> {
    let file = props.get("file").and_then(Value::as_str).filter(|f| !f.is_empty())?;
    let on = props.get("on").and_then(Value::as_str).filter(|m| !m.is_empty()).map(str::to_owned);
    Some((file.to_owned(), on))
}

fn colors() -> std::collections::HashMap<String, Hsla> {
    [("keyword", "ansi5"), ("string", "ansi2"), ("comment", "dim"), ("function", "ansi4"), ("type", "ansi3"), ("number", "ansi1"), ("constant", "ansi6")]
        .into_iter()
        .map(|(semantic, theme)| (semantic.to_owned(), c(theme)))
        .collect()
}

/// `path` as a file the person means, joined to `base` when it's relative.
fn resolve(path: &str, base: &str) -> String {
    let path = path.strip_prefix("file://").unwrap_or(path);
    let absolute = path.starts_with('/') || path.starts_with('~') || path.as_bytes().get(1) == Some(&b':') || path.starts_with("\\\\");
    if absolute || base.is_empty() {
        return path.to_owned();
    }
    format!("{}/{}", base.trim_end_matches('/'), path.trim_start_matches("./"))
}

impl AppWindow {
    fn editor_target(&self, a: &Value) -> Result<String, String> {
        let pane = Self::owned(a, "pane").or_else(|| self.model.focused_pane()).ok_or("no pane: focus an editor or pass pane")?;
        if self.editors.contains_key(&pane) { Ok(pane) } else { Err(format!("pane {pane:?} is not a file editor")) }
    }

    /// Every editor pane the model has, with its tab: (pane, tab, file, machine, line).
    fn editor_panes(&self) -> Vec<(String, String, String, Option<String>, Option<usize>)> {
        let mut out = Vec::new();
        for s in &self.model.spaces {
            for t in &s.tabs {
                for n in t.root.panes() {
                    if let Some(Content::Mod { name, props }) = n.content()
                        && name == NAME
                        && let Some((file, on)) = file_props(props)
                    {
                        out.push((n.id.clone(), t.id.clone(), file, on, props.get("line").and_then(Value::as_u64).map(|l| l as usize)));
                    }
                }
            }
        }
        out
    }

    /// Panes with unsaved changes, among `panes` (all of them when None).
    pub(super) fn unsaved_files(&self, panes: Option<&[String]>) -> Vec<String> {
        let mut out: Vec<String> = self.editors.iter()
            .filter(|(id, e)| e.dirty && panes.is_none_or(|p| p.contains(id)))
            .map(|(_, e)| editor::file_name(&e.path).to_owned())
            .collect();
        out.sort();
        out
    }

    /// True when `command` must wait: files in `panes` (every open one for None) have
    /// unsaved changes. A person is asked; a script gets an error naming them.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn ask_unsaved(&mut self, command: &str, mut args: Value, panes: Option<&[String]>, a: &Value, person: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if a.get("confirmed").and_then(Value::as_bool) == Some(true) {
            return false;
        }
        let files = self.unsaved_files(panes);
        if files.is_empty() {
            return false;
        }
        let files = files.join(", ");
        if !person {
            self.fail(crate::i18n::tf("rs.editor.unsaved_script", &[("command", command), ("files", &files)]));
            return true;
        }
        args["confirmed"] = json!(true);
        let dirty: Vec<String> = self.editors.iter().filter(|(id, e)| e.dirty && panes.is_none_or(|p| p.contains(id))).map(|(id, _)| id.clone()).collect();
        let save = json!({ "panes": dirty, "then": { "name": command, "args": args } });
        let mut ask = super::confirm::Ask::new(
            crate::i18n::tf("rs.editor.close_title", &[("files", &files)]),
            crate::i18n::tf("rs.editor.close_detail", &[("files", &files)]),
            command,
            args,
        );
        ask.width = 420.0;
        ask.confirm_label = Some(crate::i18n::t("rs.editor.discard"));
        ask.extra = Some((crate::i18n::t("rs.editor.save_close"), ("editor.save_close".to_owned(), save)));
        self.open_confirm(ask, window, cx);
        true
    }

    /// `editor.save_close {panes?, then}`: saves the files with unsaved changes (those of `panes`,
    /// all by default), then runs `then {name, args}` as confirmed. A file that can't be saved
    /// (the disk changed, a write error) stops it: nothing closes and the file is shown.
    fn save_then(&mut self, a: &Value, window: &mut Window, cx: &mut Context<Self>) {
        let Some(then) = a.get("then").and_then(|t| t.get("name")).and_then(Value::as_str).map(str::to_owned) else { return self.fail("editor.save_close needs then: {name, args}") };
        let mut args = a["then"].get("args").cloned().filter(Value::is_object).unwrap_or_else(|| json!({}));
        args["confirmed"] = json!(true);
        let wanted: Option<Vec<String>> = a.get("panes").and_then(Value::as_array).map(|p| p.iter().filter_map(Value::as_str).map(str::to_owned).collect());
        let mut dirty: Vec<String> = self.editors.iter().filter(|(id, e)| e.dirty && wanted.as_ref().is_none_or(|p| p.contains(id))).map(|(id, _)| id.clone()).collect();
        dirty.sort();
        if dirty.is_empty() {
            self.answer(Ok(json!({ "saved": 0 })));
            return self.run(&then, args, window, cx);
        }
        for pane in &dirty {
            if let Err(e) = self.save_editor(pane, false, window, cx) {
                return self.fail(e);
            }
        }
        let n = dirty.len();
        self.save_close = Some(editor::SaveThen::new(dirty, &then, args));
        self.answer(Ok(json!({ "saving": n })));
    }

    /// One save finished: runs what waits on it, or stops and shows the file that didn't save.
    fn settle_save_close(&mut self, pane: &str, window: &mut Window, cx: &mut Context<Self>) {
        let clean = self.editors.get(pane).is_some_and(|e| !e.dirty);
        let Some(pending) = self.save_close.as_mut().filter(|p| p.waits_on(pane)) else { return };
        match pending.settled(pane, clean) {
            editor::Step::Waiting => {}
            editor::Step::Done => {
                if let Some(p) = self.save_close.take() {
                    self.run(&p.command, p.args, window, cx);
                }
            }
            editor::Step::Failed => {
                self.save_close = None;
                let file = self.editors.get(pane).map(|e| editor::file_name(&e.path).to_owned()).unwrap_or_default();
                self.run("pane.focus", json!({ "pane": pane }), window, cx);
                self.command_error("editor.save_close", &crate::i18n::tf("rs.editor.save_close_failed", &[("file", &file)]), window, cx);
            }
        }
    }

    fn count_unsaved(&self) {
        UNSAVED.store(self.editors.values().filter(|e| e.dirty).count(), Ordering::Relaxed);
    }

    pub(super) fn dirty_panes(&self) -> HashSet<String> {
        self.editors.iter().filter(|(_, e)| e.dirty).map(|(id, _)| id.clone()).collect()
    }

    /// Creates the state of every editor pane that has none and drops the ones whose pane is gone.
    pub(super) fn sync_editors(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let wanted = self.editor_panes();
        self.editors.retain(|id, e| wanted.iter().any(|(pane, _, file, on, _)| pane == id && *file == e.path && *on == e.on));
        for (pane, _, file, on, line) in wanted {
            if self.editors.contains_key(&pane) {
                continue;
            }
            if let Some(line) = line {
                self.editor_goto.entry(pane.clone()).or_insert(line);
            }
            self.editors.insert(pane.clone(), EditorPane::new(file, on));
            self.read_editor(&pane, window, cx);
        }
        self.editor_goto.retain(|id, _| self.editors.contains_key(id));
        self.count_unsaved();
        if !self.editors.is_empty() && self.editor_poll.is_none() {
            self.editor_poll = Some(cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor().timer(POLL).await;
                    let Ok(files) = this.update(cx, |this, _| this.pollable()) else { return };
                    if files.is_empty() {
                        continue;
                    }
                    let seen = cx.background_executor().spawn(async move { files.into_iter().map(|(id, path, gen_)| (id, gen_, editor::peek(std::path::Path::new(&path)))).collect::<Vec<_>>() }).await;
                    let _ = this.update_in(cx, |this, window, cx| this.polled(seen, window, cx));
                }
            }));
        }
    }

    /// Local files worth checking against the disk: loaded and not being saved.
    fn pollable(&self) -> Vec<(String, String, u64)> {
        self.editors.iter().filter(|(_, e)| e.on.is_none() && !e.saving && matches!(e.phase, Phase::Ready)).map(|(id, e)| (id.clone(), e.path.clone(), e.generation)).collect()
    }

    /// What the disk said about each open file: a clean one follows it, an edited one asks.
    fn polled(&mut self, seen: Vec<(String, u64, Option<String>)>, window: &mut Window, cx: &mut Context<Self>) {
        for (id, gen_, disk) in seen {
            let Some(e) = self.editors.get_mut(&id) else { continue };
            if e.generation != gen_ || e.saving {
                continue;
            }
            let Some(doc) = e.doc.as_ref() else { continue };
            match doc.compare(disk.as_deref()) {
                Disk::Same => {
                    if matches!(e.note, Some(Note::Conflict(_))) {
                        e.note = None;
                        cx.notify();
                    }
                }
                Disk::Changed(text) if !e.dirty => self.apply_disk(&id, text, window, cx),
                Disk::Changed(text) => {
                    if !matches!(&e.note, Some(Note::Conflict(Some(t))) if *t == text) {
                        e.note = Some(Note::Conflict(Some(text)));
                        cx.notify();
                    }
                }
                Disk::Gone => {
                    if e.dirty && !matches!(e.note, Some(Note::Conflict(None))) {
                        e.note = Some(Note::Conflict(None));
                        cx.notify();
                    }
                }
            }
        }
    }

    /// Shows the disk's text as the file (nothing unsaved is lost: the caller checked).
    fn apply_disk(&mut self, id: &str, disk: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(e) = self.editors.get_mut(id) else { return };
        let (Some(doc), Some(state)) = (e.doc.as_mut(), e.state.clone()) else { return };
        let shown = doc.rebase(disk);
        e.dirty = false;
        e.note = None;
        state.update(cx, |s, cx| s.set_value(shown, window, cx));
        self.count_unsaved();
        self.publish(cx);
        cx.notify();
    }

    /// Reads the file in the background and shows it.
    fn read_editor(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(e) = self.editors.get_mut(id) else { return };
        e.generation += 1;
        let (gen_, path, on, id) = (e.generation, e.path.clone(), e.on.clone(), id.to_owned());
        cx.spawn_in(window, async move |this, cx| {
            let result = cx.background_executor().spawn(async move {
                match on {
                    Some(machine) => editor::read_remote(&machine, &path),
                    None => editor::read_local(std::path::Path::new(&path)),
                }
            }).await;
            let _ = this.update_in(cx, |this, window, cx| this.read_done(&id, gen_, result, window, cx));
        })
        .detach();
    }

    fn read_done(&mut self, id: &str, gen_: u64, result: Result<String, LoadError>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(e) = self.editors.get_mut(id) else { return };
        if e.generation != gen_ {
            return;
        }
        match result {
            Err(error) => {
                e.phase = Phase::Failed(error.message(&e.path));
                e.state = None;
                e.doc = None;
                e.dirty = false;
                e._sub = None;
            }
            Ok(disk) => {
                if let (Some(_), Some(_)) = (e.doc.as_ref(), e.state.as_ref()) {
                    // A reload of a file already shown.
                    return self.apply_disk(id, disk, window, cx);
                }
                let (doc, shown) = Doc::open(disk);
                let (path, readonly) = (e.path.clone(), e.readonly());
                let state = self.build_editor(&path, &shown, readonly, window, cx);
                let pane = id.to_owned();
                let sub = cx.subscribe_in(&state, window, move |this, _, ev: &InputEvent, _, cx| {
                    if matches!(ev, InputEvent::Change) {
                        this.editor_changed(&pane, cx);
                    }
                });
                let Some(e) = self.editors.get_mut(id) else { return };
                e.look = look();
                e.doc = Some(doc);
                e.state = Some(state.clone());
                e._sub = Some(sub);
                e.phase = Phase::Ready;
                if let Some(line) = self.editor_goto.remove(id) {
                    state.update(cx, |s, cx| s.set_cursor_position(Position::new(line.saturating_sub(1) as u32, 0), window, cx));
                }
                if self.focused_pane.as_deref() == Some(id) && self.window_active && !self.overlays.iter().any(|o| o.kind.modal_keys()) && self.model.active_tab().is_some_and(|t| t.root.find(id).is_some()) {
                    state.focus_handle(cx).focus(window, cx);
                }
            }
        }
        self.publish(cx);
        cx.notify();
    }

    fn build_editor(&self, path: &str, text: &str, readonly: bool, window: &mut Window, cx: &mut Context<Self>) -> Entity<EditorState> {
        let l = look();
        let state = cx.new(|cx| {
            let mut s = EditorState::new(window, cx).language(editor::language_for(path)).line_number(l.numbers).folding(l.folding).soft_wrap(l.wrap);
            s.set_readonly(readonly, cx);
            s.set_editor_style(gpui_shell::editor_style(colors()));
            s.set_highlighter_factory(gpui_shell::editor_highlighter_factory(), cx);
            s
        });
        state.update(cx, |s, cx| s.set_value(text.to_owned(), window, cx));
        state
    }

    /// Settings > Editor changed: shown files follow it.
    pub(super) fn apply_editor_look(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let now = look();
        for e in self.editors.values_mut() {
            if e.look == now {
                continue;
            }
            e.look = now;
            if let Some(state) = &e.state {
                state.update(cx, |s, cx| {
                    s.set_line_number(now.numbers, window, cx);
                    s.set_folding(now.folding, window, cx);
                    s.set_soft_wrap(now.wrap, window, cx);
                });
            }
        }
    }

    fn editor_changed(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(e) = self.editors.get_mut(id) else { return };
        let (Some(doc), Some(state)) = (e.doc.as_ref(), e.state.as_ref()) else { return };
        let dirty = doc.dirty(&state.read(cx).value());
        if dirty != e.dirty {
            e.dirty = dirty;
            if !dirty && matches!(e.note, Some(Note::Error(_))) {
                e.note = None;
            }
            self.count_unsaved();
            self.publish(cx);
        }
        cx.notify();
    }

    // ---- commands -----------------------------------------------------------

    /// `file.open`, `editor.save`, `editor.preview`, `editor.reload`, `editor.goto`.
    pub(super) fn editor_command(&mut self, name: &str, a: &Value, person: bool, window: &mut Window, cx: &mut Context<Self>) {
        if name == "file.open" {
            let Some(path) = Self::arg(a, "path") else { return self.fail("file.open needs path") };
            let line = a.get("line").and_then(Value::as_u64).map(|l| l as usize);
            let r = self.open_file(path, line, Self::owned(a, "on"), Self::owned(a, "space"), Self::focus(a).unwrap_or(true), window, cx);
            return self.answer(r);
        }
        if name == "editor.save_close" {
            return self.save_then(a, window, cx);
        }
        let pane = match self.editor_target(a) {
            Ok(p) => p,
            Err(e) => return self.fail(e),
        };
        match name {
            "editor.save" => {
                let force = a.get("force").and_then(Value::as_bool) == Some(true);
                match self.save_editor(&pane, force, window, cx) {
                    Ok(()) => self.answer(Ok(json!({ "pane": pane }))),
                    Err(e) => self.fail(e),
                }
            }
            "editor.preview" => {
                let Some(e) = self.editors.get_mut(&pane) else { return };
                if !editor::is_markdown(&e.path) {
                    return self.fail("only a markdown file has a preview");
                }
                e.preview = a.get("on").and_then(Value::as_bool).unwrap_or(!e.preview);
                if !e.preview
                    && let Some(s) = &e.state
                {
                    s.focus_handle(cx).focus(window, cx);
                }
                cx.notify();
            }
            "editor.reload" => {
                let Some(e) = self.editors.get(&pane) else { return };
                if e.dirty && a.get("confirmed").and_then(Value::as_bool) != Some(true) {
                    if !person {
                        return self.fail("editor.reload drops unsaved changes: pass confirmed: true");
                    }
                    let detail = crate::i18n::tf("rs.editor.discard_detail", &[("files", editor::file_name(&e.path))]);
                    let args = json!({ "pane": pane, "confirmed": true });
                    return self.confirm(&crate::i18n::t("rs.editor.reload_title"), &detail, "editor.reload", args, window, cx);
                }
                self.reload_editor(&pane, window, cx);
            }
            "editor.goto" => {
                let Some(line) = a.get("line").and_then(Value::as_u64) else { return self.fail("editor.goto needs line (1 is the first)") };
                self.goto_line(&pane, line as usize, window, cx);
            }
            _ => self.fail(format!("unknown editor command {name:?}")),
        }
    }

    fn goto_line(&mut self, pane: &str, line: usize, window: &mut Window, cx: &mut Context<Self>) {
        match self.editors.get(pane).and_then(|e| e.state.clone()) {
            Some(state) => {
                if let Some(e) = self.editors.get_mut(pane) {
                    e.preview = false;
                }
                state.update(cx, |s, cx| s.set_cursor_position(Position::new(line.saturating_sub(1) as u32, 0), window, cx));
                cx.notify();
            }
            None => {
                self.editor_goto.insert(pane.to_owned(), line);
            }
        }
    }

    fn reload_editor(&mut self, pane: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(e) = self.editors.get_mut(pane) {
            e.note = None;
            if e.doc.is_none() {
                e.phase = Phase::Loading;
            }
        }
        self.read_editor(pane, window, cx);
        cx.notify();
    }

    /// Opens a file in its own tab (or shows the tab that has it).
    #[allow(clippy::too_many_arguments)]
    fn open_file(&mut self, path: &str, line: Option<usize>, on: Option<String>, space: Option<String>, focus: bool, window: &mut Window, cx: &mut Context<Self>) -> Result<Value, String> {
        let space_id = space.or_else(|| self.model.active_space().map(|p| p.id.clone())).ok_or("no space yet: make one first (space.new)")?;
        let Some(sp) = self.model.space(&space_id) else { return Err(format!("no space {space_id:?}")) };
        // `on` empty: this computer; absent: the space's machine.
        let on = match on {
            Some(machine) => Some(machine).filter(|m| !m.is_empty()),
            None => sp.machine.clone(),
        };
        let path = resolve(path, &sp.folder);
        if path.chars().any(char::is_control) {
            return Err("the path has control characters".into());
        }
        let same = |tab: &Tab| tab.root.panes().into_iter().find_map(|n| match n.content() {
            Some(Content::Mod { name, props }) if name == NAME && file_props(props) == Some((path.clone(), on.clone())) => Some(n.id.clone()),
            _ => None,
        });
        let existing = self.model.space(&space_id).and_then(|s| s.tabs.iter().find_map(|t| same(t).map(|pane| (t.id.clone(), pane))));
        if let Some((tab, pane)) = existing {
            if focus {
                self.model.active = Some(space_id.clone());
                self.model.select_tab(&tab);
                self.model.focus_pane(&pane);
            }
            if let Some(line) = line {
                self.goto_line(&pane, line, window, cx);
            }
            return Ok(json!({ "space": space_id, "tab": tab, "pane": pane, "opened": false }));
        }
        let mut props = json!({ "file": path });
        if let Some(on) = &on {
            props["on"] = json!(on);
        }
        let made = self.new_mod_tab(Some(space_id), editor::file_name(&path), NAME.to_owned(), props, focus)?;
        if let (Some(line), Some(pane)) = (line, made.get("pane").and_then(Value::as_str)) {
            self.editor_goto.insert(pane.to_owned(), line);
        }
        Ok(made)
    }

    /// Opens the files dropped on the window; folders are left alone.
    pub(super) fn open_dropped(&mut self, paths: &[PathBuf], window: &mut Window, cx: &mut Context<Self>) {
        for p in paths.iter().filter(|p| p.is_file()) {
            let Some(path) = p.to_str() else { continue };
            // A file dropped from this computer is local whatever space is shown.
            let r = self.open_file(path, None, Some(String::new()), None, true, window, cx);
            if let Err(e) = r {
                self.command_error("file.open", &e, window, cx);
            }
        }
        self.flush_documents(false, window, cx);
        cx.notify();
    }

    fn save_editor(&mut self, pane: &str, force: bool, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        let Some(e) = self.editors.get_mut(pane) else { return Err(format!("no editor {pane:?}")) };
        if e.readonly() {
            return Err(crate::i18n::t("rs.editor.remote_read_only"));
        }
        let (Some(doc), Some(state)) = (e.doc.as_ref(), e.state.as_ref()) else { return Err(crate::i18n::t("rs.editor.not_loaded")) };
        if e.saving {
            return Ok(());
        }
        let view = state.read(cx).value().to_string();
        let text = doc.disk_text(&view);
        let (baseline, path) = (doc.baseline().to_owned(), e.path.clone());
        e.saving = true;
        let pane = pane.to_owned();
        cx.spawn_in(window, async move |this, cx| {
            let outcome = cx.background_executor().spawn(async move {
                let path = std::path::Path::new(&path);
                if force { editor::overwrite(path, &text) } else { editor::save(path, &text, &baseline) }
            }).await;
            let _ = this.update_in(cx, |this, window, cx| this.save_done(&pane, view, outcome, window, cx));
        })
        .detach();
        Ok(())
    }

    fn save_done(&mut self, pane: &str, view: String, outcome: Saved, window: &mut Window, cx: &mut Context<Self>) {
        let Some(e) = self.editors.get_mut(pane) else { return };
        e.saving = false;
        match outcome {
            Saved::Written => {
                if let Some(doc) = e.doc.as_mut() {
                    doc.saved(&view);
                }
                e.note = None;
                if let (Some(doc), Some(state)) = (e.doc.as_ref(), e.state.as_ref()) {
                    // Typing during the write keeps its mark.
                    e.dirty = doc.dirty(&state.read(cx).value());
                }
            }
            Saved::Conflict(disk) => e.note = Some(Note::Conflict(disk)),
            Saved::Failed(error) => e.note = Some(Note::Error(error)),
        }
        self.count_unsaved();
        self.publish(cx);
        cx.notify();
        self.settle_save_close(pane, window, cx);
    }

    // ---- drawing --------------------------------------------------------------

    pub(super) fn editor_focus(&self, pane: &str, cx: &App) -> Option<FocusHandle> {
        self.editors.get(pane).and_then(|e| e.state.as_ref()).map(|s| s.focus_handle(cx).clone())
    }

    /// The editor pane whose text holds the keyboard.
    pub(super) fn focused_editor(&self, window: &Window, cx: &App) -> Option<String> {
        self.editors.iter().find(|(_, e)| e.state.as_ref().is_some_and(|s| s.focus_handle(cx).is_focused(window))).map(|(id, _)| id.clone())
    }

    pub(super) fn render_editor(&self, id: &str, cx: &mut Context<Self>) -> AnyElement {
        let Some(e) = self.editors.get(id) else {
            return div().size_full().bg(c("background")).into_any_element();
        };
        let pane = id.to_owned();
        let button = |key: &str, label: String, primary: bool, on: Rc<dyn Fn(&mut AppWindow, &mut Window, &mut Context<AppWindow>)>, cx: &mut Context<Self>| {
            div()
                .id(SharedString::from(format!("{key}-{id}")))
                .h(px(24.0))
                .px(px(9.0))
                .flex()
                .items_center()
                .flex_none()
                .rounded(px(6.0))
                .border_1()
                .border_color(if primary { c("accent") } else { c("border") })
                .bg(if primary { c("accent") } else { c("raised") })
                .text_color(if primary { c("accent_foreground") } else { c("foreground") })
                .text_xs()
                .cursor_pointer()
                .hover(|s| s.opacity(0.85))
                .on_click(cx.listener(move |this, _, window, cx| on(this, window, cx)))
                .child(label)
        };
        let name = editor::file_name(&e.path).to_owned();
        let folder = e.path.strip_suffix(&name).unwrap_or("").trim_end_matches(['/', '\\']).to_owned();
        let md = editor::is_markdown(&e.path);
        let ready = matches!(e.phase, Phase::Ready);
        let mut bar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.0))
            .h(px(36.0))
            .px(px(12.0))
            .flex_none()
            .bg(c("surface"))
            .child(div().w(px(8.0)).h(px(8.0)).flex_none().rounded_full().when(e.dirty, |d| d.bg(c("accent"))))
            .child(div().flex_none().font_weight(FontWeight::SEMIBOLD).text_sm().child(name))
            .child(div().flex_1().min_w_0().overflow_hidden().whitespace_nowrap().text_xs().text_color(c("dim")).child(match &e.on {
                Some(machine) => format!("{machine}:{folder}"),
                None => folder,
            }));
        if e.readonly() {
            bar = bar.child(div().flex_none().text_xs().text_color(c("muted")).child(crate::i18n::t("rs.editor.read_only")));
        }
        if md && ready {
            let (p1, label) = (pane.clone(), if e.preview { crate::i18n::t("rs.editor.edit") } else { crate::i18n::t("rs.editor.preview") });
            bar = bar.child(button("preview", label, false, Rc::new(move |this, window, cx| this.run("editor.preview", json!({ "pane": p1 }), window, cx)), cx));
        }
        if ready && !e.readonly() {
            let p2 = pane.clone();
            bar = bar.child(button("save", crate::i18n::t("rs.editor.save"), e.dirty, Rc::new(move |this, window, cx| this.run("editor.save", json!({ "pane": p2 }), window, cx)), cx));
        }
        let banner = e.note.as_ref().map(|note| {
            let (message, actions) = match note {
                Note::Conflict(disk) => {
                    let (p1, p2) = (pane.clone(), pane.clone());
                    let message = crate::i18n::t(if disk.is_some() { "rs.editor.conflict_changed" } else { "rs.editor.conflict_gone" });
                    let overwrite = button("overwrite", crate::i18n::t("rs.editor.overwrite"), false, Rc::new(move |this, window, cx| this.run("editor.save", json!({ "pane": p1, "force": true }), window, cx)), cx);
                    let reload = button("reload", crate::i18n::t("rs.editor.reload"), false, Rc::new(move |this, window, cx| this.run("editor.reload", json!({ "pane": p2 }), window, cx)), cx);
                    (message, vec![overwrite.into_any_element(), reload.into_any_element()])
                }
                Note::Error(error) => (crate::i18n::tf("rs.editor.save_failed", &[("error", error)]), vec![]),
            };
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .flex_none()
                .mx(px(10.0))
                .mb(px(8.0))
                .px(px(10.0))
                .py(px(6.0))
                .rounded(px(6.0))
                .border_1()
                .border_color(c("blocked"))
                .bg(c("raised"))
                .text_xs()
                .child(div().flex_1().min_w_0().child(message))
                .children(actions)
        });
        let body: AnyElement = match (&e.phase, &e.state) {
            (Phase::Ready, Some(state)) if e.preview => {
                let source = state.read(cx).value();
                div()
                    .id(SharedString::from(format!("editor-md-{id}")))
                    .size_full()
                    .overflow_y_scroll()
                    .p(px(18.0))
                    .child(gpui_base::TextView::markdown(SharedString::from(format!("editor-md-text-{id}")), source).selectable(true))
                    .into_any_element()
            }
            (Phase::Ready, Some(state)) => {
                let focus = state.clone();
                let font = settings::get().font_family.clone();
                InputBase::new(SharedString::from(format!("editor-text-{id}")))
                    .flex()
                    .items_start()
                    .w_full()
                    .h_full()
                    .min_h_0()
                    .font_family(font)
                    .text_size(px(e.look.size * settings::get().zoom))
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| focus.update(cx, |s, cx| s.focus(window, cx)))
                    .child(Editor::new(state))
                    .into_any_element()
            }
            (Phase::Failed(message), _) => div().size_full().flex().items_center().justify_center().p(px(24.0)).text_sm().text_color(c("dim")).child(message.clone()).into_any_element(),
            _ => div().size_full().flex().items_center().justify_center().text_sm().text_color(c("dim")).child(crate::i18n::t("rs.editor.loading")).into_any_element(),
        };
        let click = pane.clone();
        div()
            .id(SharedString::from(format!("editor-{id}")))
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .overflow_hidden()
            .bg(c("background"))
            .text_color(c("foreground"))
            .key_context(format!("Mod mod = {NAME}").as_str())
            .on_any_mouse_down(cx.listener(move |this, _, _, cx| {
                if this.focused_pane.as_deref() != Some(click.as_str()) {
                    this.model.focus_pane(&click);
                    this.focused_pane = Some(click.clone());
                    this.publish(cx);
                    this.save();
                    cx.notify();
                }
            }))
            .child(bar)
            .children(banner)
            .child(div().flex_1().min_h_0().child(body))
            .into_any_element()
    }
}

/// What a window test can see of an editor pane.
#[cfg(test)]
pub(super) struct Probe {
    pub loaded: bool,
    pub failed: Option<String>,
    pub dirty: bool,
    pub text: Option<String>,
    pub note: &'static str,
    pub preview: bool,
}

#[cfg(test)]
impl AppWindow {
    pub(super) fn editor_probe(&self, pane: &str, cx: &App) -> Option<Probe> {
        let e = self.editors.get(pane)?;
        Some(Probe {
            loaded: matches!(e.phase, Phase::Ready),
            failed: match &e.phase {
                Phase::Failed(m) => Some(m.clone()),
                _ => None,
            },
            dirty: e.dirty,
            text: e.state.as_ref().map(|s| s.read(cx).value().to_string()),
            note: match e.note {
                None => "",
                Some(Note::Conflict(_)) => "conflict",
                Some(Note::Error(_)) => "error",
            },
            preview: e.preview,
        })
    }

    /// Types `text` at the cursor, as the person would.
    pub(super) fn editor_type(&mut self, pane: &str, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(state) = self.editors.get(pane).and_then(|e| e.state.clone()) {
            state.update(cx, |s, cx| s.replace(text.to_owned(), window, cx));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{file_props, resolve};
    use serde_json::{Value, json};

    #[test]
    fn props_name_the_file_and_machine() {
        assert_eq!(file_props(&json!({ "file": "/a/b.rs" })), Some(("/a/b.rs".into(), None)));
        assert_eq!(file_props(&json!({ "file": "/a/b.rs", "on": "vm" })), Some(("/a/b.rs".into(), Some("vm".into()))));
        assert_eq!(file_props(&json!({ "file": "/a/b.rs", "on": "" })), Some(("/a/b.rs".into(), None)));
        assert_eq!(file_props(&json!({ "file": "" })), None);
        assert_eq!(file_props(&Value::Null), None);
    }

    #[test]
    fn relative_paths_start_in_the_space_folder() {
        assert_eq!(resolve("src/a.rs", "/work/app"), "/work/app/src/a.rs");
        assert_eq!(resolve("./a.rs", "/work/app/"), "/work/app/a.rs");
        assert_eq!(resolve("/etc/hosts", "/work/app"), "/etc/hosts");
        assert_eq!(resolve("C:/x/y.txt", "/work/app"), "C:/x/y.txt");
        assert_eq!(resolve("file:///tmp/a.md", "/work"), "/tmp/a.md");
        assert_eq!(resolve("a.rs", ""), "a.rs");
    }
}
