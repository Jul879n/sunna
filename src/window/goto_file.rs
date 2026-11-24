//! Go to file (Cmd/Ctrl+P), native: fuzzy search over the files of the focused
//! pane's project. The list comes from ripgrep, else `git ls-files`, else `find`
//! (`crate::goto`), read in the background; Enter opens the file in the editor
//! (`file.open`), Esc closes. The tab/agent selector is the "Go to space or tab"
//! entry of the palette and Cmd/Ctrl+Alt+P.

use super::*;
use crate::goto::{self, MAX_FILES, Row};
use crate::i18n;
use gpui_base::input::{Input, InputEvent, InputState};

/// Overlay id: a second press of the key closes it.
pub(super) const ID: &str = "goto-file";
/// Rows shown at once; the list scrolls by keys.
const SHOWN: usize = 10;
const ROW_H: f32 = 32.0;
const SEARCH_ICON: &[u8] = include_bytes!("../../assets/icons/goto/search.svg");
const FILE_ICON: &[u8] = include_bytes!("../../assets/icons/goto/file-text.svg");

pub(super) struct GotoView {
    app: WeakEntity<AppWindow>,
    input: Entity<InputState>,
    focus: FocusHandle,
    query: String,
    at: usize,
    first: usize,
    /// None while the list is being read.
    files: Option<Vec<String>>,
    error: Option<String>,
    root: Option<String>,
    on: Option<String>,
    recent: Vec<String>,
    rows: Vec<Row>,
}

impl GotoView {
    /// `place` is the folder and machine the focused pane is in.
    fn new(app: WeakEntity<AppWindow>, place: (Option<String>, Option<String>), window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(i18n::t("rs.goto.placeholder")));
        input.update(cx, |s, _| s.set_editor_style(settings_ui::controls::input_style()));
        cx.subscribe_in(&input, window, |this, input, ev: &InputEvent, window, cx| match ev {
            InputEvent::Change => {
                this.query = input.read(cx).value().to_string();
                this.at = 0;
                this.first = 0;
                this.rank();
                cx.notify();
            }
            InputEvent::PressEnter { .. } => this.open(this.at, window, cx),
            _ => {}
        })
        .detach();
        input.update(cx, |s, cx| s.focus(window, cx));
        let (cwd, on) = place;
        let mut this = Self {
            app, input, focus: cx.focus_handle(), query: String::new(), at: 0, first: 0, files: None, error: None, root: None, on: on.clone(), recent: Vec::new(), rows: Vec::new(),
        };
        match cwd {
            None => {
                this.error = Some(i18n::t("rs.goto.no_folder"));
                this.files = Some(Vec::new());
            }
            Some(cwd) => {
                // Off the UI thread: listing a big project takes seconds.
                cx.spawn(async move |this, cx| {
                    let place = on.clone();
                    let listed = cx.background_executor().spawn(async move {
                        let (root, files) = goto::list(&cwd, place.as_deref())?;
                        let recent = goto::recent(place.as_deref(), &root);
                        Ok::<_, String>((root, files, recent))
                    }).await;
                    let _ = this.update(cx, |this, cx| {
                        match listed {
                            Ok((root, files, recent)) => {
                                this.root = Some(root);
                                this.files = Some(files);
                                this.recent = recent;
                            }
                            Err(e) => {
                                this.error = Some(e);
                                this.files = Some(Vec::new());
                            }
                        }
                        this.rank();
                        cx.notify();
                    });
                })
                .detach();
            }
        }
        this
    }

    /// Ranking 60000 paths is too slow for render: it runs when the query or the list
    /// changes, and render reuses it.
    fn rank(&mut self) {
        self.rows = goto::rank(self.files.as_deref().unwrap_or_default(), &self.query, &self.recent, goto::LIMIT);
    }

    fn close(&self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.app.update(cx, |app, cx| app.close_overlay(ID, window, cx));
    }

    fn open(&mut self, i: usize, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(row), Some(root)) = (self.rows.get(i), self.root.clone()) else { return };
        let path = row.path.clone();
        goto::remember(self.on.as_deref(), &root, &path);
        let args = json!({ "path": format!("{root}/{path}"), "on": self.on.clone().unwrap_or_default() });
        let _ = self.app.update(cx, |app, cx| {
            app.close_overlay(ID, window, cx);
            app.run("file.open", args, window, cx);
        });
    }

    /// `label` with its matched letters (`hits`, in characters) in the accent.
    fn text(label: &str, hits: &[usize]) -> StyledText {
        let starts: Vec<usize> = label.char_indices().map(|(b, _)| b).chain([label.len()]).collect();
        let lit = HighlightStyle { color: Some(c("accent")), font_weight: Some(FontWeight::SEMIBOLD), ..Default::default() };
        let marks: Vec<_> = hits.iter().filter(|&&i| i + 1 < starts.len()).map(|&i| (starts[i]..starts[i + 1], lit)).collect();
        StyledText::new(label.to_owned()).with_highlights(marks)
    }
}

impl Render for GotoView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let z = settings::z;
        self.at = self.at.min(self.rows.len().saturating_sub(1));
        if self.at < self.first {
            self.first = self.at;
        }
        if self.at >= self.first + SHOWN {
            self.first = self.at + 1 - SHOWN;
        }
        let empty = self.error.clone().unwrap_or_else(|| i18n::t(if self.files.is_some() { "rs.goto.empty" } else { "rs.goto.indexing" }));
        let list = if self.rows.is_empty() {
            div().px(z(16.0)).py(z(14.0)).text_size(z(13.0)).text_color(c("dim")).child(empty)
        } else {
            div().mb(z(6.0)).flex().flex_col().children(self.rows.iter().enumerate().skip(self.first).take(SHOWN).map(|(i, r)| {
                let on = i == self.at;
                div()
                    .id(SharedString::from(format!("gf-{i}")))
                    .h(z(ROW_H))
                    .flex_none()
                    .mx(z(6.0))
                    .px(z(10.0))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(z(10.0))
                    .rounded(z(7.0))
                    .cursor_pointer()
                    .text_size(z(12.8))
                    .when(on, |d| d.bg(c("selected")))
                    .hover(|s| s.bg(c("hover")))
                    .on_click(cx.listener(move |this, _, w, cx| this.open(i, w, cx)))
                    .child(svg().data(FILE_ICON).size(z(14.0)).text_color(c("dim")))
                    .child(div().flex_none().max_w(z(320.0)).truncate().text_color(c(if on { "foreground" } else { "muted" })).child(Self::text(&r.name, &r.name_hits)))
                    .child(div().flex_1().min_w_0().truncate().text_size(z(11.5)).text_color(c("dim")).child(Self::text(&r.dir, &r.dir_hits)))
            }))
        };
        div()
            .id("goto-file")
            .track_focus(&self.focus)
            .w_full()
            .min_h_0()
            .p(z(18.0))
            .flex()
            .flex_col()
            .gap(z(10.0))
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, w, cx| {
                let last = this.rows.len().saturating_sub(1);
                match e.keystroke.key.as_str() {
                    "escape" => return this.close(w, cx),
                    "down" => this.at = (this.at + 1).min(last),
                    "up" => this.at = this.at.saturating_sub(1),
                    "pagedown" => this.at = (this.at + SHOWN).min(last),
                    "pageup" => this.at = this.at.saturating_sub(SHOWN),
                    _ => return,
                }
                cx.stop_propagation();
                cx.notify();
            }))
            .child(
                div()
                    .h(z(36.0))
                    .flex_none()
                    .px(z(10.0))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(z(10.0))
                    .rounded(z(9.0))
                    .border_1()
                    .border_color(c("accent"))
                    .bg(c("background"))
                    .child(svg().data(SEARCH_ICON).size(z(16.0)).text_color(c("dim")))
                    .child(div().flex_1().h_full().text_size(z(12.8)).text_color(c("foreground")).child(Input::new(&self.input)))
                    .child(div().text_size(z(11.0)).text_color(c("dim")).child("Esc")),
            )
            .child(list)
            .when(self.files.as_ref().is_some_and(|f| f.len() >= MAX_FILES), |d| {
                d.child(div().px(z(6.0)).text_size(z(11.0)).text_color(c("dim")).child(i18n::t("rs.goto.truncated")))
            })
    }
}

impl AppWindow {
    /// The folder and machine of the focused pane, the way the Files, Changes and Search
    /// panels read them: its terminal's folder, else the space's.
    fn goto_place(&self) -> (Option<String>, Option<String>) {
        let space = self.model.active_space();
        let block = overlay_terminal_block(&self.overlays, self.window_terminal.as_deref()).or_else(|| terminal_block(&self.chrome, self.focused_pane.as_deref()));
        let (own, on) = match block.and_then(|b| b.terminal()) {
            Some(address) => (self.cwds.get(address).cloned(), address.split_once('/').map(|(machine, _)| machine.to_owned())),
            None => {
                let address = self.model.focused_pane().and_then(|p| self.model.address_of(&p));
                let shown = if self.shows_agent() { self.shown_agent() } else { address.clone() };
                (address.and_then(|a| self.cwds.get(&a).cloned()).or_else(|| shown.and_then(|a| self.cwds.get(&a).cloned())), space.and_then(|s| s.machine.clone()))
            }
        };
        let cwd = own.filter(|f| !f.is_empty()).or_else(|| space.map(|s| s.folder.clone()).filter(|f| !f.is_empty()));
        (cwd, on.filter(|m| !m.is_empty()))
    }

    /// `file.goto`: opens the selector, or closes it when it is open.
    pub(super) fn toggle_goto_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.overlays.iter().any(|o| o.id == ID) {
            return self.close_overlay(ID, window, cx);
        }
        let place = self.goto_place();
        let app = cx.weak_entity();
        let view = cx.new(|cx| GotoView::new(app, place, window, cx));
        let focus = cx.focus_handle();
        self.push_view_overlay(ID.into(), OverlayKind::Popup, size(640.0, 0.6), Some(40.0), view.into(), focus, cx);
    }
}
