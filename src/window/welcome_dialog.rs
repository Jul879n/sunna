//! The first-run welcome, native: four short pages in the dialog every window uses (what Sunna is and the language,
//! theme and size; the other machines; the panels; how to make it yours). It opens on the first start and from the
//! palette's «Bienvenida» (`app.welcome`). Esc or the ✕ skip it; the last page's button ends it. What it records is
//! what the `frame-welcome` mod recorded: shared `frame.welcomed` is `finished` or `skipped`. Left and Right move
//! between pages, Enter goes forward. The empty-state block of the layout (`welcome`) is another thing.

use super::dialog::{self, Act, Dialog, Tone};
use gpui_base::{Scrollbar, ScrollbarMode};
use super::native::{icon, outline, tone, z};
use super::palette::key_chips;
use super::*;
use crate::i18n;

/// Overlay id (the notes' clips close it by this name).
pub(super) const ID: &str = "frame-welcome";
const WIDTH: f32 = 640.0;
const PAGES: usize = 4;
const ASK: &str = "Read ~/.config/sunna/AGENTS.md, then tell me what you can change in Sunna.";
const MIGRATE: &str = "Read `sunna --skill`, then migrate my keybinds and settings from ~/.config/herdr/config.toml";
const BOT: &[u8] = include_bytes!("../../assets/icons/welcome/bot.svg");
const MESSAGES: &[u8] = include_bytes!("../../assets/icons/welcome/messages.svg");
const LAYERS: &[u8] = include_bytes!("../../assets/icons/welcome/layers.svg");
const TERMINAL: &[u8] = include_bytes!("../../assets/icons/welcome/terminal.svg");
const ARROW_RIGHT: &[u8] = include_bytes!("../../assets/icons/welcome/arrow-right.svg");
const MONITOR: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/monitor.svg");
const SERVER: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/server.svg");
const PLUS: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/plus.svg");
const FILES: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/files.svg");
const COMPARE: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/git-compare.svg");
const SEARCH: &[u8] = include_bytes!("../../defaults/mods/workspace-chrome/search.svg");

/// The panels the layout offers (its `panel-body` stack): id, icon, title and about (text keys).
const PANELS: [(&str, &[u8], &str, &str); 3] = [
    ("files", FILES, "welcome.files", "welcome.the_folder_you_re_in_as"),
    ("changes", COMPARE, "welcome.changes", "welcome.what_your_agents_changed_every"),
    ("search", SEARCH, "welcome.search", "welcome.find_text_across_the_files_of"),
];

// Rules ------------------------------------------------------------------------------

/// The page after pressing `by` (1 or -1): never outside 1..=PAGES.
pub(super) fn turn(page: usize, by: i32) -> usize {
    (page as i64 + by as i64).clamp(1, PAGES as i64) as usize
}

/// The panels the tab bar shows: what `workspace.panels` says, else all of them.
pub(super) fn panels_on(shared: Option<&Value>) -> Vec<String> {
    match shared.and_then(Value::as_array) {
        Some(list) => list.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect(),
        None => PANELS.iter().map(|p| p.0.to_owned()).collect(),
    }
}

/// The panels after switching `id`.
pub(super) fn toggled(on: &[String], id: &str) -> Vec<String> {
    if on.iter().any(|p| p == id) { on.iter().filter(|p| *p != id).cloned().collect() } else { on.iter().cloned().chain([id.to_owned()]).collect() }
}

/// A node of the layout document by id.
fn find<'a>(node: &'a Value, id: &str) -> Option<&'a Value> {
    if node.get("id").and_then(Value::as_str) == Some(id) {
        return Some(node);
    }
    node.get("children").and_then(Value::as_array)?.iter().find_map(|c| find(c, id))
}

/// What a machine's state says in the list.
pub(super) fn machine_note(state: &str) -> String {
    match state {
        "connected" => i18n::t("welcome.connected"),
        "" | "offline" => i18n::t("welcome.offline"),
        other => other.replace('_', " "),
    }
}

// The view ---------------------------------------------------------------------------

pub(super) struct WelcomeView {
    app: WeakEntity<AppWindow>,
    focus: dialog::Focus,
    page: usize,
    machines: Vec<Value>,
    copied: Option<String>,
    appearance: Option<String>,
    language: Option<String>,
    /// Is a space open already (the last button says «Hecho» instead of «Abrir un espacio»).
    has_spaces: bool,
    scroll: ScrollHandle,
}

impl WelcomeView {
    fn new(app: WeakEntity<AppWindow>, has_spaces: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let saved = settings_ui::read_settings();
        let text = |k: &str| saved.get(k).and_then(Value::as_str).map(str::to_owned);
        let focus = dialog::Focus::new(cx);
        focus.put(dialog::initial_stop(0, &[Tone::Primary]), 1, window, cx);
        // The machines added meanwhile (the Add a machine dialog) appear: read now and every 3 seconds on page 2.
        cx.spawn(async move |this, cx| {
            let mut first = true;
            loop {
                let on_page = this.update(cx, |this, _| this.page == 2).unwrap_or(false);
                if first || on_page {
                    let listed = cx.background_executor().spawn(async { mods::run_blocking("hc", &["machine".into(), "ls".into(), "--json".into()], None, Duration::from_secs(20)) }).await;
                    let machines: Vec<Value> = listed
                        .ok()
                        .filter(|(code, _, _)| *code == 0)
                        .and_then(|(_, out, _)| serde_json::from_str::<Value>(&out).ok())
                        .and_then(|v| v.get("machines").and_then(Value::as_array).cloned())
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|m| m.get("temporary").and_then(Value::as_bool) != Some(true))
                        .collect();
                    if this.update(cx, |this, cx| {
                        this.machines = machines;
                        cx.notify();
                    }).is_err() {
                        return;
                    }
                }
                first = false;
                cx.background_executor().timer(Duration::from_secs(3)).await;
            }
        })
        .detach();
        Self { app, focus, page: 1, machines: Vec::new(), copied: None, appearance: text("appearance"), language: text("language"), has_spaces, scroll: ScrollHandle::new() }
    }

    fn done(&self, outcome: &str, window: &mut Window, cx: &mut Context<Self>) {
        mods::set_shared("frame.welcomed".into(), json!(outcome));
        let _ = self.app.update(cx, |app, cx| app.close_overlay(ID, window, cx));
    }

    fn start(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.done("finished", window, cx);
        if !self.has_spaces {
            let _ = self.app.update(cx, |app, cx| app.run("space.new", json!({}), window, cx));
        }
    }

    fn go(&mut self, by: i32, cx: &mut Context<Self>) {
        self.page = turn(self.page, by);
        // The first look at the panels page asks: nothing is picked yet (skipping the welcome leaves them all on).
        if self.page == 3 && mods::shared("workspace.panels").is_none_or(|v| v.is_null()) {
            mods::set_shared("workspace.panels".into(), json!([]));
        }
        cx.notify();
    }

    fn run(&self, name: &'static str, args: Value, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.app.update(cx, |app, cx| app.run(name, args, window, cx));
    }

    fn set_setting(&mut self, key: &'static str, value: Option<String>, cx: &mut Context<Self>) {
        settings_ui::edit_settings(|q| match &value {
            Some(v) => drop(q.insert(key.into(), json!(v))),
            None => drop(q.remove(key)),
        });
        if key == "appearance" { self.appearance = value } else { self.language = value }
        cx.notify();
    }

    fn toggle_panel(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let was = panels_on(mods::shared("workspace.panels").as_ref());
        let on = was.iter().any(|p| p == id);
        let next = toggled(&was, id);
        mods::set_shared("workspace.panels".into(), json!(next));
        let layout = mods::layout_snapshot();
        if find(&layout, "panel").is_some() {
            if on {
                if find(&layout, "panel-body").and_then(|n| n.get("active")).and_then(Value::as_str) == Some(id) || next.is_empty() {
                    self.run("layout.hide", json!({ "id": "panel" }), window, cx);
                }
            } else {
                self.run("layout.select", json!({ "stack": "panel-body", "child": id }), window, cx);
                self.run("layout.show", json!({ "id": "panel" }), window, cx);
            }
        }
        cx.notify();
    }

    // Pieces ---------------------------------------------------------------------------

    fn para(text: impl Into<SharedString>) -> Div {
        div().w_full().text_size(z(13.0)).line_height(z(20.0)).text_color(tone("muted")).child(text.into())
    }

    fn title(text: impl Into<SharedString>, sub: Option<String>) -> Div {
        div()
            .flex()
            .flex_col()
            .gap(z(6.0))
            .child(div().text_size(z(18.0)).font_weight(FontWeight::SEMIBOLD).child(text.into()))
            .when_some(sub, |d, s| d.child(Self::para(s)))
    }

    fn badge(glyph: &'static [u8], color: Hsla) -> Div {
        div().flex_none().size(z(32.0)).rounded(z(9.0)).bg(c("raised")).border_1().border_color(c("border")).flex().items_center().justify_center().child(icon(glyph, 16.0, color))
    }

    fn feature(glyph: &'static [u8], title: String, body: String) -> Div {
        div()
            .flex()
            .flex_row()
            .gap(z(12.0))
            .items_start()
            .child(Self::badge(glyph, c("accent")))
            .child(div().flex_1().min_w_0().flex().flex_col().gap(z(3.0)).child(div().text_size(z(13.5)).font_weight(FontWeight::SEMIBOLD).child(title)).child(Self::para(body)))
    }

    /// A box with a line to paste somewhere, and the button that copies it.
    fn copy_line(&self, text: &str, id: String, cx: &mut Context<Self>) -> Div {
        let copied = self.copied.as_deref() == Some(id.as_str());
        let (line, mine) = (text.to_owned(), id.clone());
        let this = cx.entity();
        let press = move |_: &mut Window, cx: &mut App| {
            cx.write_to_clipboard(ClipboardItem::new_string(line.clone()));
            this.update(cx, |v, cx| {
                v.copied = Some(mine.clone());
                cx.notify();
            });
        };
        let button = dialog::side_button(SharedString::from(id), i18n::t(if copied { "welcome.copied" } else { "welcome.copy" }), press);
        div()
            .flex()
            .flex_col()
            .gap(z(10.0))
            .p(z(12.0))
            .rounded(z(10.0))
            .bg(c("background"))
            .border_1()
            .border_color(outline())
            .child(div().text_size(z(13.0)).line_height(z(20.0)).child(text.to_owned()))
            .child(div().flex().flex_row().child(button))
    }

    fn page_one(&self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let _ = window;
        let this = cx.entity();
        let language = settings_ui::controls::segmented(
            "w-lang",
            vec![(None, i18n::t("settings.language_system")), (Some("es".to_owned()), i18n::t("settings.language_es")), (Some("en".to_owned()), i18n::t("settings.language_en"))],
            &self.language,
            {
                let this = this.clone();
                move |v: Option<String>, _, cx: &mut App| this.update(cx, |t, cx| t.set_setting("language", v, cx))
            },
        );
        let now = settings::get();
        let layout = mods::shared("layout").and_then(|v| v.as_str().map(str::to_owned));
        let layouts = crate::window::layout_names();
        let cards = settings_ui::shown_themes()
            .into_iter()
            .filter(|t| Some(&t.name) == layout.as_ref() || !layouts.contains(&t.name))
            .map(|th| {
                let light = settings::theme_is_light(&th.name);
                let selected = th.name == if light { now.theme_light.clone() } else { now.theme_dark.clone() } && light == (self.appearance.as_deref() == Some("light"));
                let (this, name) = (this.clone(), th.name.clone());
                settings_ui::theme_card(
                    format!("welcome-theme-{}", th.name),
                    &th,
                    selected,
                    (96.0, 60.0),
                    Some(settings_ui::controls::act(move |w, cx| {
                        this.update(cx, |t, cx| {
                            t.set_setting("appearance", Some(if light { "light" } else { "dark" }.to_owned()), cx);
                            t.run("app.theme", json!({ "name": name }), w, cx);
                        });
                    })),
                    None,
                )
            });
        let size = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(z(8.0))
            .child(dialog::side_button("w-zoom-out", "\u{2212}", dialog::run(&this, |v, w, cx| { v.run("app.zoom_out", json!({}), w, cx); cx.notify() })))
            .child(div().w(z(48.0)).flex().justify_center().text_size(z(13.0)).child(format!("{}%", (settings::zoom() * 100.0).round())))
            .child(dialog::side_button("w-zoom-in", "+", dialog::run(&this, |v, w, cx| { v.run("app.zoom_in", json!({}), w, cx); cx.notify() })));
        let row = |label: String, control: Div| div().flex().flex_row().gap(z(10.0)).items_center().child(div().w(z(84.0)).text_size(z(13.0)).text_color(tone("muted")).child(label)).child(control);
        div()
            .flex()
            .flex_col()
            .gap(z(18.0))
            .child(Self::title(i18n::t("welcome.herdr_runs_your_terminals_in"), Some(i18n::t("welcome.the_terminals_and_agents_you"))))
            .child(Self::feature(BOT, i18n::t("welcome.agents_tell_you_when_they"), i18n::t("welcome.a_ring_while_one_works")))
            .child(Self::feature(MESSAGES, i18n::t("welcome.agents_can_start_helpers"), i18n::t("welcome.they_show_up_under_the")))
            .child(Self::feature(LAYERS, i18n::t("welcome.your_other_machines_in_the"), i18n::t("welcome.a_server_or_a_mac")))
            .child(div().h(px(1.0)).bg(c("border")))
            .child(row(i18n::t("settings.language"), language))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(z(8.0))
                    .child(div().text_size(z(13.0)).text_color(tone("muted")).child(i18n::t("welcome.appearance")))
                    .child(div().flex().flex_row().flex_wrap().gap(z(8.0)).children(cards)),
            )
            .child(row(i18n::t("welcome.size"), size))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(z(12.0))
                    .p(z(16.0))
                    .rounded(z(12.0))
                    .border_1()
                    .border_color(c("border"))
                    .bg(c("raised"))
                    .child(div().flex().flex_row().gap(z(10.0)).items_center().child(icon(TERMINAL, 16.0, tone("muted"))).child(div().text_size(z(13.5)).font_weight(FontWeight::SEMIBOLD).child(i18n::t("welcome.coming_from_herdr"))))
                    .child(Self::para(i18n::t("welcome.this_is_a_new_app")))
                    .child(Self::para(i18n::t("welcome.same_idea_as_before_spaces")))
                    .child(Self::para(i18n::t("welcome.want_your_keybinds_and_settings")))
                    .child(self.copy_line(MIGRATE, "w-migrate-copy".into(), cx)),
            )
    }

    fn machine_row(glyph: &'static [u8], name: String, note: String, ok: bool) -> Div {
        div()
            .flex()
            .flex_row()
            .h(z(42.0))
            .px(z(12.0))
            .gap(z(10.0))
            .items_center()
            .border_b_1()
            .border_color(c("border"))
            .child(icon(glyph, 15.0, tone("muted")))
            .child(div().flex_1().min_w_0().truncate().text_size(z(13.0)).child(name))
            .child(div().text_size(z(12.0)).text_color(tone("muted")).child(note))
            .child(div().size(z(8.0)).rounded(z(4.0)).bg(if ok { c("ansi2") } else { tone("muted") }))
    }

    fn page_two(&self, cx: &mut Context<Self>) -> Div {
        let this = cx.entity();
        let mut list = div().flex().flex_col().rounded(z(12.0)).border_1().border_color(c("border")).bg(c("background")).overflow_hidden().child(Self::machine_row(
            MONITOR,
            i18n::t("welcome.this_computer"),
            i18n::t("welcome.here"),
            true,
        ));
        for m in &self.machines {
            let name = m.get("name").and_then(Value::as_str).unwrap_or_default().to_owned();
            let state = m.get("state").and_then(Value::as_str).unwrap_or_default();
            list = list.child(
                div()
                    .flex()
                    .flex_col()
                    .child(Self::machine_row(SERVER, name.clone(), machine_note(state), state == "connected"))
                    .when(state == "needs_login", |d| {
                        d.child(
                            div()
                                .flex()
                                .flex_col()
                                .p(z(12.0))
                                .gap(z(6.0))
                                .child(div().text_size(z(12.0)).text_color(tone("blocked")).child(i18n::t("welcome.needs_a_login_run")))
                                .child(self.copy_line(&format!("hc machine login {name}"), format!("w-login-copy-{name}"), cx)),
                        )
                    }),
            );
        }
        let add = Rc::new(dialog::run(&this, |v, w, cx| v.run("mod.open", json!({ "name": "machine-add" }), w, cx)));
        let add_row = div()
            .id("w-add-machine")
            .focusable()
            .tab_stop(true)
            .h(z(42.0))
            .px(z(12.0))
            .flex()
            .flex_row()
            .items_center()
            .gap(z(10.0))
            .cursor_pointer()
            .text_size(z(13.0))
            .text_color(c("accent"))
            .hover(|s| s.bg(c("hover")))
            .focus_visible(|s| s.border_2().border_color(graphic("accent")))
            .on_key_down({
                let add = add.clone();
                move |e: &KeyDownEvent, w, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                        add(w, cx);
                        cx.stop_propagation();
                    }
                }
            })
            .on_click(move |_, w, cx| add(w, cx))
            .child(icon(PLUS, 15.0, c("accent")))
            .child(i18n::t("welcome.add_a_machine"));
        div()
            .flex()
            .flex_col()
            .gap(z(18.0))
            .child(Self::title(i18n::t("welcome.add_your_other_machines"), Some(i18n::t("welcome.pick_a_host_from_your"))))
            .child(list.child(add_row))
            .child(Self::para(i18n::t("welcome.not_now_add_machine_is")))
    }

    fn page_three(&self, cx: &mut Context<Self>) -> Div {
        let on_now = panels_on(mods::shared("workspace.panels").as_ref());
        let cards = PANELS.iter().map(|(id, glyph, title, about)| {
            let on = on_now.iter().any(|p| p == id);
            let id = (*id).to_owned();
            let toggle = {
                let (this, id) = (cx.entity(), id.clone());
                Rc::new(move |w: &mut Window, cx: &mut App| this.update(cx, |v, cx| v.toggle_panel(&id, w, cx)))
            };
            let key_toggle = toggle.clone();
            div()
                .id(SharedString::from(format!("w-panel-{id}")))
                .focusable()
                .tab_stop(true)
                .flex()
                .flex_row()
                .p(z(12.0))
                .gap(z(12.0))
                .items_start()
                .rounded(z(12.0))
                .border_1()
                .border_color(if on { c("accent") } else { outline() })
                .when(on, |d| d.bg(Hsla { a: 0.08, ..c("accent") }))
                .cursor_pointer()
                .focus_visible(|s| s.border_2().border_color(graphic("accent")))
                .on_key_down(move |e: &KeyDownEvent, w, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                        key_toggle(w, cx);
                        cx.stop_propagation();
                    }
                })
                .on_click(move |_, w, cx| toggle(w, cx))
                .child(Self::badge(glyph, c("foreground")))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(z(4.0))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap(z(8.0))
                                .items_center()
                                .child(div().text_size(z(13.5)).font_weight(FontWeight::SEMIBOLD).child(i18n::t(title)))
                                .child(div().px(z(6.0)).rounded(z(5.0)).bg(c("hover")).text_size(z(11.0)).text_color(tone("muted")).child(format!("mods/{id}"))),
                        )
                        .child(Self::para(i18n::t(about))),
                )
                .child(
                    div()
                        .flex_none()
                        .mt(z(4.0))
                        .w(z(34.0))
                        .h(z(20.0))
                        .rounded(z(10.0))
                        .p(z(2.0))
                        .bg(if on { c("accent") } else { outline() })
                        .flex()
                        .when(on, |d| d.justify_end())
                        .child(div().size(z(16.0)).rounded(z(8.0)).bg(if on { on_accent() } else { tone("muted") })),
                )
        });
        div()
            .flex()
            .flex_col()
            .gap(z(14.0))
            .child(Self::title(i18n::t("welcome.choose_your_panels"), Some(i18n::t("welcome.herdr_comes_with_a_couple"))))
            .children(cards)
            .child(div().p(z(12.0)).rounded(z(10.0)).bg(c("raised")).child(Self::para(i18n::t("welcome.ask_your_agent_for_a"))))
    }

    fn page_four(&self, cx: &mut Context<Self>) -> Div {
        let mac = cfg!(target_os = "macos");
        let palette_key = crate::keys::listing().into_iter().find(|(s, _, a)| s == "global" && a == "app.palette").map(|(_, k, _)| key_chips(&k, mac).join(if mac { "" } else { "+" }));
        let note = |title: String, body: String| div().flex().flex_col().gap(z(6.0)).p(z(12.0)).rounded(z(10.0)).bg(c("raised")).child(div().text_size(z(13.5)).font_weight(FontWeight::SEMIBOLD).child(title)).child(Self::para(body));
        div()
            .flex()
            .flex_col()
            .gap(z(16.0))
            .when_some(palette_key, |d, key| {
                d.child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(z(12.0))
                        .items_center()
                        .p(z(12.0))
                        .rounded(z(10.0))
                        .border_1()
                        .border_color(c("accent"))
                        .bg(Hsla { a: 0.08, ..c("accent") })
                        .child(div().flex_none().px(z(8.0)).py(z(3.0)).rounded(z(6.0)).border_1().border_color(c("border")).bg(c("raised")).text_size(z(13.0)).font_weight(FontWeight::SEMIBOLD).child(key))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(z(2.0))
                                .child(div().text_size(z(13.5)).font_weight(FontWeight::SEMIBOLD).child(i18n::t("welcome.one_key_opens_everything")))
                                .child(Self::para(i18n::t("welcome.it_opens_the_command_palette"))),
                        ),
                )
            })
            .child(Self::title(i18n::t("welcome.make_it_yours"), Some(i18n::t("welcome.every_part_of_this_window"))))
            .child(self.copy_line(ASK, "w-copy".into(), cx))
            .child(Self::para(i18n::t("welcome.then_ask_for_anything_move")))
            .child(note(i18n::t("welcome.your_terminals_outside_this_window"), i18n::t("welcome.hc_is_herdr_core_s_command_line")))
            .child(note(i18n::t("welcome.more_examples_in_settings"), i18n::t("welcome.settings_look_has_more")))
            .child(Self::para(i18n::t("welcome.something_off_or_missing_settings")))
    }

    fn dots(&self) -> Div {
        div().flex().flex_row().gap(z(5.0)).items_center().children((1..=PAGES).map(|p| {
            div().w(z(if p == self.page { 16.0 } else { 6.0 })).h(z(6.0)).rounded(z(3.0)).bg(if p == self.page { c("foreground") } else { c("input") })
        }))
    }
}

impl Render for WelcomeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();
        let skip = dialog::run(&this, |v, w, cx| v.done("skipped", w, cx));
        let page = match self.page {
            1 => self.page_one(window, cx),
            2 => self.page_two(cx),
            3 => self.page_three(cx),
            _ => self.page_four(cx),
        };
        let room = (f32::from(window.viewport_size().height) / settings::zoom() * 0.62).clamp(220.0, 560.0);
        let body = div()
            .flex()
            .flex_col()
            .gap(z(14.0))
            .child(
                div()
                    .relative()
                    .child(div().id("welcome-body").max_h(z(room)).overflow_y_scroll().track_scroll(&self.scroll).child(div().pr(z(14.0)).child(page)))
                    // A bar that always shows: a page taller than the card should look it.
                    .child(div().absolute().inset_0().child(Scrollbar::vertical(&self.scroll).mode(ScrollbarMode::Always))),
            )
            .child(self.dots());
        let mut card = Dialog::new(i18n::t("welcome.welcome_to_herdr"), &self.focus, skip).width(WIDTH).no_cancel().body(body);
        if self.page > 1 {
            card = card.action(Act::new(i18n::t("welcome.back"), Tone::Neutral, dialog::run(&this, |v, _, cx| v.go(-1, cx))));
        }
        card = if self.page < PAGES {
            let label = if self.page == 2 && self.machines.is_empty() { "welcome.not_now" } else { "welcome.next" };
            card.action(Act::primary(i18n::t(label), dialog::run(&this, |v, _, cx| v.go(1, cx))))
        } else {
            card.action(Act::primary(i18n::t(if self.has_spaces { "welcome.done" } else { "welcome.open_a_space" }), dialog::run(&this, |v, w, cx| v.start(w, cx))))
        };
        let _ = ARROW_RIGHT;
        // Left and Right turn the pages (Enter goes forward through the primary button).
        div().on_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
            let m = e.keystroke.modifiers;
            if m.control || m.alt || m.platform {
                return;
            }
            match e.keystroke.key.as_str() {
                "right" => this.go(1, cx),
                "left" => this.go(-1, cx),
                _ => return,
            }
            cx.stop_propagation();
        })).child(card.render("welcome"))
    }
}

impl AppWindow {
    /// First start and `app.welcome`: the welcome over the live app.
    pub(super) fn show_welcome(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_overlay(ID, window, cx);
        let (app, has_spaces) = (cx.weak_entity(), !self.model.spaces.is_empty());
        let view = cx.new(|cx| WelcomeView::new(app, has_spaces, window, cx));
        self.push_dialog_overlay(ID.into(), WIDTH + 48.0, view.into(), window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `super::*` brings gpui's own `test` macro: the plain one is the std attribute.
    use core::prelude::v1::test;

    #[test]
    fn pages_stay_between_one_and_four() {
        assert_eq!((turn(1, -1), turn(1, 1), turn(4, 1), turn(3, 1)), (1, 2, 4, 4));
    }

    #[test]
    fn the_panels_are_all_on_until_the_person_picks() {
        assert_eq!(panels_on(None), ["files", "changes", "search"]);
        assert!(panels_on(Some(&json!([]))).is_empty());
        let on = panels_on(Some(&json!(["files"])));
        assert_eq!(toggled(&on, "search"), ["files", "search"]);
        assert!(toggled(&on, "files").is_empty());
    }

    #[test]
    fn a_node_is_found_by_id_in_the_layout() {
        let layout = json!({ "id": "root", "children": [{ "id": "a" }, { "id": "panel", "children": [{ "id": "panel-body", "active": "files" }] }] });
        assert_eq!(find(&layout, "panel-body").and_then(|n| n["active"].as_str()), Some("files"));
        assert!(find(&layout, "nope").is_none());
    }

    #[test]
    fn a_machine_state_reads_as_words() {
        assert_eq!(machine_note("connected"), i18n::t("welcome.connected"));
        assert_eq!(machine_note(""), i18n::t("welcome.offline"));
        assert_eq!(machine_note("needs_login"), "needs login");
    }
}
