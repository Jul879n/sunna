//! The Settings window, native: `NATIVE_SECTIONS` are its pages.
//! The values are the same ones as ever: settings.json keys and shared().

use super::*;
use crate::i18n;
use gpui_base::input::{InputEvent, InputState};
use gpui_base::{Scrollbar, ScrollbarMode};

mod agents;
mod alerts;
pub(super) mod controls;
mod editor;
mod fonts;
mod general;
mod git;
mod keyboard;
mod look;
mod live;
mod machines;
mod modlist;
mod modstore;
mod privacy;
pub(in crate::window) mod timefmt;
mod updates;
mod sidebar;
mod terminal;
mod theme_import;
mod theme_ui;
pub mod search;
mod store;

use controls::{Act, act};
use store::Object;

/// What the welcome dialog takes from these pages: the theme cards and a change to settings.json.
pub(super) use theme_ui::{card as theme_card, shown_themes};

/// Changes settings.json, keeping every other key; `false` when it could not be written.
pub(super) fn edit_settings(change: impl FnOnce(&mut Object)) -> bool {
    store::edit(change).is_ok()
}

/// settings.json as the person wrote it (empty when there is none).
pub(super) fn read_settings() -> Object {
    store::read().unwrap_or_default()
}

/// The Settings sections, in menu order. An unknown one opens the first.
pub(super) const NATIVE_SECTIONS: &[&str] = &["look", "general", "git", "editor", "alerts", "terminal", "keyboard", "agents", "machines", "updates", "privacy", "mods"];

/// The Settings window's overlay id.
pub(super) const NATIVE_ID: &str = "settings-native";

pub(super) fn is_native(section: &str) -> bool {
    NATIVE_SECTIONS.contains(&section)
}

/// Which text field is meant.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Field {
    Directory,
    Prefix,
    Template,
    OpenIn,
}

pub(super) struct TextField {
    id: Field,
    input: Entity<InputState>,
    /// What was typed and isn't saved yet.
    text: Option<String>,
    focused: bool,
}

pub(super) struct SettingsView {
    app: WeakEntity<AppWindow>,
    section: String,
    query: String,
    at: usize,
    search: Entity<InputState>,
    fields: Vec<TextField>,
    focus: FocusHandle,
    scroll: ScrollHandle,
    /// settings.json as read, and whether it can't be written (broken).
    person: Object,
    person_time: Option<SystemTime>,
    fonts: fonts::State,
    keys: keyboard::State,
    agents: agents::State,
    look: look::State,
    sidebar: sidebar::State,
    machines: machines::State,
    mods: modlist::State,
    /// Check now was pressed (Updates), and Show what we send (Privacy).
    asked: Option<std::time::Instant>,
    show_data: bool,
}

fn icon(section: &str) -> &'static [u8] {
    match section {
        "look" => include_bytes!("../../../assets/icons/settings/palette.svg"),
        "mods" => include_bytes!("../../../assets/icons/settings/puzzle.svg"),
        "general" => include_bytes!("../../../assets/icons/settings/settings.svg"),
        "terminal" | "agents" => include_bytes!("../../../assets/icons/settings/terminal.svg"),
        "editor" => include_bytes!("../../../assets/icons/settings/pencil.svg"),
        "git" => include_bytes!("../../../assets/icons/settings/layout.svg"),
        "keyboard" => include_bytes!("../../../assets/icons/settings/keyboard.svg"),
        "machines" => include_bytes!("../../../assets/icons/settings/server.svg"),
        "alerts" => include_bytes!("../../../assets/icons/settings/bell.svg"),
        "updates" => include_bytes!("../../../assets/icons/settings/refresh.svg"),
        _ => include_bytes!("../../../assets/icons/settings/shield.svg"),
    }
}

impl SettingsView {
    fn new(app: WeakEntity<AppWindow>, section: &str, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mk = |placeholder: String, cx: &mut Context<Self>, window: &mut Window| {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
            input.update(cx, |s, _| s.set_editor_style(controls::input_style()));
            input
        };
        let search = mk(i18n::t("settings.search_settings"), cx, window);
        cx.subscribe_in(&search, window, |this, input, ev: &InputEvent, window, cx| match ev {
            InputEvent::Change => {
                this.query = input.read(cx).value().trim().to_owned();
                this.at = 0;
                cx.notify();
            }
            InputEvent::PressEnter { .. } => {
                if let Some(hit) = search::rank(&this.query, 12).get(this.at).cloned() {
                    this.go(hit.section, window, cx);
                }
            }
            _ => {}
        })
        .detach();
        let mut fields = Vec::new();
        for (id, placeholder) in [
            (Field::Directory, general::DIRECTORY.to_owned()),
            (Field::Prefix, String::new()),
            (Field::Template, store::get_str("worktree.template")),
            (Field::OpenIn, i18n::t("settings.orca_open_in_placeholder")),
        ] {
            let input = mk(placeholder, cx, window);
            cx.subscribe_in(&input, window, move |this, input, ev: &InputEvent, window, cx| match ev {
                InputEvent::Change => {
                    // Only what the person types counts: set_value doesn't send Change.
                    let text = input.read(cx).value().to_string();
                    if let Some(f) = this.fields.iter_mut().find(|f| f.id == id) {
                        f.text = Some(text);
                    }
                }
                InputEvent::Focus => this.set_focused(id, true),
                InputEvent::Blur => {
                    this.set_focused(id, false);
                    this.commit(id, window, cx);
                }
                InputEvent::PressEnter { .. } => this.commit(id, window, cx),
                _ => {}
            })
            .detach();
            fields.push(TextField { id, input, text: None, focused: false });
        }
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let mut this = Self {
            app,
            section: section.to_owned(),
            query: String::new(),
            at: 0,
            search,
            fields,
            focus,
            scroll: ScrollHandle::new(),
            person: Object::new(),
            person_time: None,
            fonts: fonts::State::new(window, cx),
            keys: keyboard::State::default(),
            agents: agents::State::default(),
            look: look::State::default(),
            sidebar: sidebar::State::default(),
            machines: machines::State::new(window, cx),
            mods: modlist::State::new(window, cx),
            asked: None,
            show_data: false,
        };
        this.sync_person();
        this.start_live(window, cx);
        this.entered(section, window, cx);
        this
    }

    /// Tab and Shift+Tab walk the window's tab stops, staying inside this window:
    /// past the last control it comes back to the first.
    fn cycle_focus(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let step = |window: &mut Window, cx: &mut Context<Self>| if forward { window.focus_next(cx) } else { window.focus_prev(cx) };
        step(window, cx);
        let mut tries = 0;
        while !self.focus.contains_focused(window, cx) && tries < 400 {
            step(window, cx);
            tries += 1;
        }
    }

    fn set_focused(&mut self, id: Field, on: bool) {
        if let Some(f) = self.fields.iter_mut().find(|f| f.id == id) {
            f.focused = on;
        }
    }

    /// Re-reads settings.json when it changed (by the app, an agent or the person).
    fn sync_person(&mut self) {
        let time = std::fs::metadata(settings::path()).and_then(|m| m.modified()).ok();
        if self.person_time.is_some() && time == self.person_time {
            return;
        }
        self.person_time = time;
        self.person = store::read().unwrap_or_default();
    }

    /// A handler that runs on this view, which draws again afterwards.
    fn act(&self, cx: &mut Context<Self>, f: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static) -> Act {
        Self::act_on(&cx.entity().downgrade(), f)
    }

    /// The same from a handle taken earlier, for code that builds many handlers at once.
    fn act_on(view: &WeakEntity<Self>, f: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static) -> Act {
        let view = view.clone();
        act(move |w, cx| {
            view.update(cx, |this, cx| {
                f(this, w, cx);
                cx.notify();
            })
            .ok();
        })
    }

    /// A switch for a yes/no kept in shared().
    fn shared_switch(&self, cx: &mut Context<Self>, id: &'static str, key: &'static str) -> Stateful<Div> {
        let on = store::flag(key);
        controls::switch(id, on, self.act(cx, move |_, _, _| mods::set_shared(key.into(), json!(!on))))
    }

    /// A switch for a yes/no stored as a field of an object in shared().
    fn object_switch(&self, cx: &mut Context<Self>, id: &'static str, key: &'static str, field: &'static str) -> Stateful<Div> {
        let on = store::object_flag(key, field);
        controls::switch(id, on, self.act(cx, move |_, _, _| store::set_object_flag(key, field, !on)))
    }

    /// A choice kept in shared(): `choices` are (value, locale key).
    fn shared_pick(&self, cx: &mut Context<Self>, id: &'static str, key: &'static str, choices: &[(&'static str, &'static str)]) -> Div {
        let now = store::get_str(key);
        let list = choices.iter().map(|(v, l)| ((*v).to_owned(), i18n::t(l))).collect();
        let view = cx.entity().downgrade();
        controls::segmented(id, list, &now, move |value: String, _w, cx| {
            mods::set_shared(key.into(), json!(value));
            view.update(cx, |_, cx| cx.notify()).ok();
        })
    }

    /// Asks for a folder, then hands it to `then`.
    fn pick_folder(&self, prompt: String, window: &mut Window, cx: &mut Context<Self>, then: impl FnOnce(&mut Self, String, &mut Window, &mut Context<Self>) + 'static) {
        let paths = cx.prompt_for_paths(PathPromptOptions { files: false, directories: true, multiple: false, prompt: Some(prompt.into()) });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else { return };
            let Some(folder) = paths.into_iter().next() else { return };
            let _ = this.update_in(cx, |this, window, cx| {
                then(this, folder.to_string_lossy().into_owned(), window, cx);
                cx.notify();
            });
        })
        .detach();
    }

    /// Runs one of the app's commands (`app.keep_awake`, ...).
    fn run(&self, name: &'static str, args: Value, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.app.update(cx, |app, cx| app.run(name, args, window, cx));
    }

    fn toast(&self, failure: store::Failure, window: &mut Window, cx: &mut Context<Self>) {
        let mut n = json!({ "title": failure.title, "level": "error" });
        if let Some(body) = failure.body {
            n["body"] = json!(body);
        }
        self.notify(n, window, cx);
    }

    pub(super) fn notify(&self, notification: Value, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.app.update(cx, |app, cx| app.open_overlay(mods::new_overlay_id(), json!({ "kind": "toast", "notification": notification }), window, cx));
    }

    /// Changes settings.json the way an agent would: read, change, write back; says so when it can't.
    fn change(&mut self, window: &mut Window, cx: &mut Context<Self>, edit: impl FnOnce(&mut Object)) {
        match store::edit(edit) {
            Ok(next) => {
                self.person = next;
                self.person_time = std::fs::metadata(settings::path()).and_then(|m| m.modified()).ok();
            }
            Err(failure) => self.toast(failure, window, cx),
        }
        cx.notify();
    }

    /// Goes to a section: here when it's drawn here, else in the JS window.
    fn go(&mut self, section: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.query.clear();
        self.at = 0;
        self.search.update(cx, |s, cx| s.set_value("", window, cx));
        self.focus.focus(window, cx);
        self.fonts.open = None;
        self.keys.capture = None;
        self.keys.restore = false;
        if section != "agents" {
            self.agents.known = None;
        }
        self.section = section.to_owned();
        self.scroll.set_offset(point(px(0.0), px(0.0)));
        self.machines_cancel_edit();
        self.entered(section, window, cx);
        cx.notify();
    }

    /// Saves what was typed in a field (on Enter or when it loses focus).
    fn commit(&mut self, id: Field, window: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = self.fields.iter_mut().find(|f| f.id == id).and_then(|f| f.text.take()) else { return };
        let text = text.trim().to_owned();
        match id {
            Field::Directory => {
                let value = if text.is_empty() { general::DIRECTORY.to_owned() } else { text };
                self.change(window, cx, |q| general::set_directory(q, &value));
            }
            Field::Prefix => mods::set_shared("worktree.prefix".into(), json!(text)),
            Field::Template => mods::set_shared("worktree.template".into(), json!(if text.is_empty() { "{name}".to_owned() } else { text })),
            Field::OpenIn => {
                if !text.is_empty() {
                    general::add_open_in(&text);
                    if let Some(f) = self.fields.iter().find(|f| f.id == id) {
                        f.input.update(cx, |s, cx| s.set_value("", window, cx));
                    }
                }
            }
        }
        cx.notify();
    }

    /// The text field of `id`, showing what is stored unless the person is typing.
    fn text_field(&mut self, id: Field, stored: &str, width: f32, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let f = self.fields.iter().find(|f| f.id == id).expect("every field exists");
        let input = f.input.clone();
        if !f.focused && f.text.is_none() && input.read(cx).value().as_ref() != stored {
            input.update(cx, |s, cx| s.set_value(stored.to_owned(), window, cx));
        }
        controls::field_ring(&input, width, f.focused)
    }

    fn waiting_update() -> bool {
        let u = crate::update::status();
        let ready = |v: &Value| !v.is_null();
        ready(&u["app"]["ready"]) || (ready(&u["core"]["ready"]) && u["core"]["managed"].is_null()) || u["fork"]["newer"] == json!(true)
    }

    fn nav(&self, cx: &mut Context<Self>) -> Div {
        let waiting = Self::waiting_update();
        let mut nav = div()
            .flex_none()
            .w(settings::z(190.0))
            .h_full()
            .p(settings::z(10.0))
            .flex()
            .flex_col()
            .gap(settings::z(2.0))
            .border_r_1()
            .border_color(c("border"))
            .child(div().px(settings::z(10.0)).pt(settings::z(6.0)).pb(settings::z(12.0)).text_size(settings::z(16.0)).font_weight(FontWeight::SEMIBOLD).text_color(c("foreground")).child(i18n::t("settings.settings")))
            .child(
                div()
                    .id("st-search")
                    .flex_none()
                    .h(settings::z(30.0))
                    .px(settings::z(10.0))
                    .mb(settings::z(8.0))
                    .flex()
                    .items_center()
                    .rounded(settings::z(7.0))
                    .border_1()
                    .border_color(c("border"))
                    .bg(super::dialog::field_bg())
                    .text_size(settings::z(12.5))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(div().size_full().child(gpui_base::input::Input::new(&self.search))),
            );
        for (title, sections) in search::GROUPS {
            nav = nav.child(div().px(settings::z(10.0)).pt(settings::z(10.0)).pb(settings::z(4.0)).text_size(settings::z(11.0)).font_weight(FontWeight::SEMIBOLD).text_color(tone("muted")).child(i18n::t(title).to_uppercase()));
            for &id in *sections {
                let on = id == self.section && self.query.is_empty();
                nav = nav.child(
                    div()
                        .id(SharedString::from(format!("st-s-{id}")))
                        .flex_none()
                        .h(settings::z(30.0))
                        .px(settings::z(10.0))
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(settings::z(10.0))
                        .rounded(settings::z(7.0))
                        .border_1()
                        .border_color(gpui::transparent_black())
                        .cursor_pointer()
                        .when(on, |d| d.bg(c("raised")))
                        .hover(|s| s.bg(c("raised")))
                        .tab_index(0)
                        .focus_visible(controls::ring)
                        .on_click(cx.listener(move |this, _, w, cx| this.go(id, w, cx)))
                        .child(svg().data(icon(id)).size(settings::z(15.0)).text_color(if on { c("accent") } else { c("muted") }))
                        .child(div().flex_1().text_size(settings::z(13.5)).text_color(if on { c("foreground") } else { c("muted") }).child(i18n::t(search::section_title(id))))
                        .when(id == "updates" && waiting, |d| d.child(div().w(settings::z(7.0)).h(settings::z(7.0)).rounded_full().bg(c("accent")))),
                );
            }
        }
        nav.child(div().flex_1()).child(controls::button("st-feedback", i18n::t("settings.send_feedback"), self.act_feedback(cx)))
    }

    fn act_feedback(&self, cx: &mut Context<Self>) -> Act {
        self.act(cx, |this, w, cx| {
            let _ = this.app.update(cx, |app, cx| {
                app.close_overlay(NATIVE_ID, w, cx);
                app.run("app.feedback", json!({}), w, cx);
            });
        })
    }

    /// The results in place of the page: where each one lives, the best first.
    fn results(&self, cx: &mut Context<Self>) -> Div {
        let hits = search::rank(&self.query, 12);
        let place = |id: &str| {
            let group = search::GROUPS.iter().find(|(_, s)| s.contains(&id)).map(|(t, _)| i18n::t(t)).unwrap_or_default();
            format!("{group} › {}", i18n::t(search::section_title(id)))
        };
        let mut out = div().w_full().flex().flex_col().gap(settings::z(4.0)).child(controls::heading(i18n::t("settings.search_results")));
        if hits.is_empty() {
            out = out.child(div().text_size(settings::z(12.5)).text_color(tone("muted")).child(i18n::tf("settings.no_results", &[("query", &self.query)])));
        }
        for (i, hit) in hits.into_iter().enumerate() {
            let section = hit.section;
            out = out.child(
                div()
                    .id(SharedString::from(format!("st-hit-{i}")))
                    .w_full()
                    .px(settings::z(12.0))
                    .py(settings::z(8.0))
                    .flex()
                    .flex_col()
                    .gap(settings::z(2.0))
                    .rounded(settings::z(8.0))
                    .cursor_pointer()
                    .when(i == self.at, |d| d.bg(c("selected")))
                    .hover(|s| s.bg(c("raised")))
                    .tab_index(0)
                    .focus_visible(controls::ring)
                    .on_click(cx.listener(move |this, _, w, cx| this.go(section, w, cx)))
                    .child(div().text_size(settings::z(14.0)).text_color(c("foreground")).child(hit.title))
                    .child(div().text_size(settings::z(12.0)).text_color(tone("muted")).child(place(hit.section))),
            );
        }
        out
    }
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_person();
        let page = if !self.query.is_empty() {
            self.results(cx)
        } else {
            match self.section.as_str() {
                "git" => self.git(window, cx),
                "editor" => self.editor(cx),
                "alerts" => self.alerts(cx),
                "look" => self.look_page(cx),
                "terminal" => self.terminal(cx),
                "keyboard" => self.keyboard(cx),
                "agents" => self.agents(window, cx),
                "machines" => self.machines_page(window, cx),
                "updates" => self.updates_page(cx),
                "privacy" => self.privacy_page(cx),
                "mods" => self.mods_page(window, cx),
                _ => self.general(window, cx),
            }
        };
        div()
            .id("settings")
            .track_focus(&self.focus)
            .w_full()
            .h(settings::z(620.0))
            .min_h_0()
            .flex()
            .flex_row()
            .items_start()
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, w, cx| {
                let key = e.keystroke.key.as_str();
                if key == "tab" && this.keys.capture.is_none() {
                    this.cycle_focus(!e.keystroke.modifiers.shift, w, cx);
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                if this.keys.capture.is_some() {
                    this.capture_key(e, w, cx);
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                if key == "escape" && this.fonts.open.is_some() {
                    this.close_fonts(w, cx);
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                if !this.query.is_empty() {
                    let n = search::rank(&this.query, 12).len();
                    match key {
                        "down" | "up" if n > 0 => {
                            this.at = (this.at + if key == "down" { 1 } else { n - 1 }) % n;
                        }
                        "escape" => {
                            this.query.clear();
                            this.at = 0;
                            this.search.update(cx, |s, cx| s.set_value("", w, cx));
                        }
                        _ => return,
                    }
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .child(self.nav(cx))
            .child(
                // A bar that always shows: the pages are taller than the dialog.
                div()
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .child(
                        div()
                            .id("st-body")
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll)
                            .child(div().px(settings::z(28.0)).py(settings::z(22.0)).child(page)),
                    )
                    .child(div().absolute().inset_0().child(Scrollbar::vertical(&self.scroll).mode(ScrollbarMode::Always))),
            )
    }
}

impl AppWindow {
    /// Opens the native Settings window on `section` (see `is_native`).
    pub(super) fn open_native_settings(&mut self, section: &str, window: &mut Window, cx: &mut Context<Self>) {
        let app = cx.weak_entity();
        let section = section.to_owned();
        let view = cx.new(|cx| SettingsView::new(app, &section, window, cx));
        let focus = cx.focus_handle();
        self.overlays.push(Overlay {
            id: NATIVE_ID.into(),
            kind: OverlayKind::Dialog,
            slot: format!("overlay:{NATIVE_ID}"),
            at: self.pointer,
            size: size(880.0, 620.0),
            ttl: None,
            expires: None,
            focus,
            layout: None,
            top: None,
            fit: false,
            modal: false,
            bare: false,
            place: None,
            key: None,
            frame: true,
            view: Some(view.into()),
            initial: None,
        });
        cx.notify();
    }
}
