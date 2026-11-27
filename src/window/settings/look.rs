//! Settings > Appearance: the layout, the theme (cards, importing, your own),
//! light or dark, zoom and fonts, the right sidebar, and how the spaces, the
//! status bar and the tabs look.

use super::fonts::{DEFAULT_UI_FONT, Size};
use super::theme_import::{self, Source, free_name, slug};
use super::theme_ui::{self, Th, shown_themes};
use super::*;
use controls::{button, row, segmented};
use gpui_base::input::{InputEvent, InputState};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// What each shipped layout is called to a person (others show their file name).
fn look_name(name: &str) -> String {
    match name {
        "main" => "Sunna".to_owned(),
        "terminals" => i18n::t("settings.terminals"),
        "win98" => "Herdr 98".to_owned(),
        other => other.to_owned(),
    }
}

/// Layout names in the welcome's order: the shipped ones first, then the person's by name.
pub fn ordered(mut names: Vec<String>) -> Vec<String> {
    let shipped = ["main", "terminals", "win98"];
    let rank = |n: &str| shipped.iter().position(|s| *s == n).unwrap_or(shipped.len());
    names.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a.cmp(b)));
    names
}

/// The themes of one lightness that this layout shows: a theme named like a
/// layout (themes/win98.json) belongs to that layout alone.
pub fn for_layout(all: Vec<Th>, light: bool, layout: &str, layouts: &[String]) -> Vec<Th> {
    all.into_iter().filter(|t| settings::theme_is_light(&t.name) == light && (t.name == layout || !layouts.contains(&t.name))).collect()
}

pub struct Import {
    pub sources: Vec<Source>,
    pub extra: Vec<PathBuf>,
    pub filter: String,
    pub input: Entity<InputState>,
}

#[derive(Default)]
pub struct State {
    /// layout name -> the picture its file names (`"preview"` on its root).
    pub previews: BTreeMap<String, String>,
    pub previews_of: String,
    pub importing: Option<Import>,
    pub editor: Option<theme_ui::Editor>,
}

impl State {
    /// `importing` for the pages' code that only asks whether the panel is open.
    pub fn is_importing(&self) -> bool {
        self.importing.is_some()
    }
}

fn load_previews(names: &[String]) -> BTreeMap<String, String> {
    names
        .iter()
        .filter_map(|n| {
            let text = std::fs::read_to_string(layout_path(n)).ok()?;
            let p = serde_json::from_str::<Value>(&text).ok()?["preview"].as_str()?.to_owned();
            (!p.is_empty()).then(|| (n.clone(), p))
        })
        .collect()
}

/// A small drawing of a layout in the theme's colors, for those that name no picture.
fn drawing(name: &str) -> Div {
    let z = settings::z;
    let row = || div().flex().flex_row();
    let col = || div().flex().flex_col();
    let term = || div().flex_1().rounded(z(2.0)).bg(c("background")).border_1().border_color(c("border"));
    let line = |w: f32, color: Hsla| div().h(z(3.0)).w(z(w)).flex_none().rounded(z(2.0)).bg(color);
    let lines = |n: usize| col().gap(z(4.0)).p(z(5.0)).children((0..n).map(|i| line(if i % 2 == 1 { 22.0 } else { 32.0 }, c("muted"))));
    let frame = |d: Div| d.size_full().rounded(z(6.0)).overflow_hidden().border_1().border_color(c("border")).bg(c("surface"));
    match name {
        "main" => frame(row())
            .child(col().w(z(42.0)).border_r_1().border_color(c("border")).child(lines(4)))
            .child(
                col()
                    .flex_1()
                    .child(row().h(z(11.0)).gap(z(2.0)).p(z(2.0)).border_b_1().border_color(c("border")).child(div().w(z(20.0)).rounded(z(2.0)).bg(c("accent"))).child(div().w(z(16.0)).rounded(z(2.0)).bg(c("raised"))).child(div().w(z(16.0)).rounded(z(2.0)).bg(c("raised"))))
                    .child(row().flex_1().p(z(4.0)).gap(z(3.0)).child(term()).child(term())),
            ),
        "terminals" => {
            let card = || col().flex_1().rounded(z(4.0)).border_1().border_color(c("#2c2f34")).bg(c("#17191c")).child(row().h(z(7.0)).px(z(3.0)).items_center().border_b_1().border_color(c("#25282c")).child(line(14.0, c("#5d6168"))));
            frame(col())
                .bg(c("#131619"))
                .p(z(4.0))
                .gap(z(4.0))
                .child(row().h(z(10.0)).gap(z(3.0)).p(z(1.0)).rounded(z(4.0)).bg(c("#1d1f22")).child(div().w(z(30.0)).rounded(z(3.0)).bg(c("#34373c")).border_1().border_color(c("#4a4d53"))).child(div().w(z(30.0))))
                .child(row().flex_1().gap(z(4.0)).child(card()).child(card()))
        }
        _ => frame(col()).child(row().h(z(11.0)).border_b_1().border_color(c("border"))).child(row().flex_1().p(z(4.0)).child(term())),
    }
}

impl SettingsView {
    fn layout_picker(&mut self, cx: &mut Context<Self>) -> Div {
        let z = settings::z;
        let names = ordered(layout_names());
        let key = names.join("\n");
        if key != self.look.previews_of {
            self.look.previews = load_previews(&names);
            self.look.previews_of = key;
        }
        let current = store::get_str("layout");
        let current = if current.is_empty() { "main".to_owned() } else { current };
        let mut out = div().w_full().flex().flex_row().flex_wrap().gap(z(8.0));
        for name in names {
            let on = name == current;
            let picture = match self.look.previews.get(&name) {
                Some(p) => img(mods::app_dir().join(p)).w_full().h(z(62.0)).rounded(z(6.0)).border_1().border_color(c("border")).into_any_element(),
                None => drawing(&name).into_any_element(),
            };
            let pick = name.clone();
            out = out.child(
                div()
                    .id(SharedString::from(format!("st-l-{name}")))
                    .w(z(124.0))
                    .flex()
                    .flex_col()
                    .gap(z(6.0))
                    .p(z(6.0))
                    .rounded(z(10.0))
                    .border_2()
                    .border_color(if on { c("accent") } else { gpui::transparent_black() })
                    .bg(if on { c("raised") } else { gpui::transparent_black() })
                    .cursor_pointer()
                    .hover(|s| s.bg(c("raised")))
                    .active(|s| s.bg(c("selected")))
                    .tab_index(0)
                    .focus_visible(|s| s.border_color(c("foreground")))
                    .on_click({
                        let go = self.act(cx, move |this, w, cx| this.run("layout.switch", json!({ "name": pick }), w, cx));
                        move |_, w, cx| go(w, cx)
                    })
                    .child(div().h(z(62.0)).w_full().overflow_hidden().child(picture))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(z(5.0))
                            .h(z(24.0))
                            .child(div().flex_1().min_w_0().truncate().text_size(z(12.5)).text_color(if on { c("foreground") } else { c("muted") }).child(look_name(&name)))
                            .when(on, |d| d.child(svg().data(include_bytes!("../../../assets/icons/settings/check.svg").as_slice()).size(z(12.0)).flex_none().text_color(c("accent")))),
                    ),
            );
        }
        out
    }

    fn theme_picker(&self, light: bool, cx: &mut Context<Self>) -> Div {
        let s = settings::get();
        let now = if light { s.theme_light.clone() } else { s.theme_dark.clone() };
        let layout = store::get_str("layout");
        let mut out = div().w_full().flex().flex_row().flex_wrap().gap(settings::z(10.0));
        for th in for_layout(shown_themes(), light, &layout, &layout_names()) {
            let (pick, edit) = (th.name.clone(), th.name.clone());
            let on_edit = (!th.shipped).then(|| (format!("st-te-{}", th.name), self.act(cx, move |this, w, cx| this.edit_theme(&edit, w, cx))));
            out = out.child(theme_ui::card(format!("st-t-{}", th.name), &th, th.name == now, (132.0, 84.0), Some(self.act(cx, move |this, w, cx| this.run("app.theme", json!({ "name": pick }), w, cx))), on_edit));
        }
        out
    }

    fn open_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(i18n::t("settings.import_filter")));
        input.update(cx, |s, _| s.set_editor_style(controls::input_style()));
        cx.subscribe_in(&input, window, |this, input, ev: &InputEvent, _w, cx| {
            if matches!(ev, InputEvent::Change)
                && let Some(i) = this.look.importing.as_mut()
            {
                i.filter = input.read(cx).value().to_lowercase();
                cx.notify();
            }
        })
        .detach();
        let sources = theme_import::scan(&mods::home(), &[]);
        self.look.importing = Some(Import { sources, extra: Vec::new(), filter: String::new(), input });
        self.look.editor = None;
    }

    /// Reads one file, installs it as `themes/<name>.json` and uses it.
    fn import_file(&mut self, name: &str, path: &std::path::Path, window: &mut Window, cx: &mut Context<Self>) {
        let result = std::fs::read_to_string(path)
            .map_err(|_| theme_import::Failure { key: "settings.import_unreadable", vars: vec![] })
            .and_then(|text| theme_import::parse(name, &text));
        match result {
            Ok(doc) => {
                let taken: Vec<String> = shown_themes().into_iter().map(|t| t.name).collect();
                let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
                let dir = mods::app_dir().join("themes");
                let mut file = free_name(&slug(stem), &taken);
                while dir.join(format!("{file}.json")).exists() {
                    file = free_name(&format!("{}-x", file), &taken);
                }
                let _ = std::fs::create_dir_all(&dir);
                match crate::persist::write_atomic(&dir.join(format!("{file}.json")), doc.to_file()) {
                    Ok(()) => {
                        self.run("app.theme", json!({ "name": file }), window, cx);
                        self.notify(json!({ "title": i18n::tf("settings.import_done", &[("name", &file)]) }), window, cx);
                    }
                    Err(e) => self.notify(json!({ "level": "error", "title": i18n::t("settings.import_failed"), "body": e.to_string() }), window, cx),
                }
            }
            Err(f) => self.notify(json!({ "level": "error", "title": i18n::t("settings.import_failed"), "body": f.text() }), window, cx),
        }
        self.look.importing = None;
    }

    fn import_panel(&self, cx: &mut Context<Self>) -> Div {
        let z = settings::z;
        let Some(imp) = self.look.importing.as_ref() else { return div() };
        let me = cx.entity().downgrade();
        let mut shown: Vec<(&Source, Vec<&(String, PathBuf)>)> = imp.sources.iter().map(|s| (s, s.files.iter().filter(|(n, _)| imp.filter.is_empty() || n.to_lowercase().contains(&imp.filter)).collect::<Vec<_>>())).filter(|(_, f)| !f.is_empty()).collect();
        shown.truncate(50);
        let mut out = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(z(10.0))
            .p(z(12.0))
            .mb(z(10.0))
            .rounded(z(10.0))
            .border_1()
            .border_color(c("border"))
            .bg(c("surface"))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(z(8.0))
                    .child(controls::field_fill(&imp.input, false))
                    .child(button("st-imp-pick", i18n::t("settings.import_pick_folder"), Self::act_on(&me, |this, w, cx| {
                        this.pick_folder(i18n::t("settings.import_pick_folder"), w, cx, |this, folder, _, _| {
                            if let Some(i) = this.look.importing.as_mut() {
                                let dir = PathBuf::from(folder);
                                if !i.extra.contains(&dir) {
                                    i.extra.push(dir);
                                }
                                i.sources = theme_import::scan(&mods::home(), &i.extra);
                            }
                        })
                    })))
                    .child(button("st-imp-close", i18n::t("settings.cancel"), Self::act_on(&me, |this, _, _| this.look.importing = None))),
            );
        if shown.is_empty() {
            out = out.child(div().text_size(z(12.5)).text_color(tone("muted")).child(i18n::t("settings.import_nothing_found")));
        }
        for (i, (source, files)) in shown.into_iter().enumerate() {
            let mut chips = div().flex().flex_row().flex_wrap().gap(z(6.0));
            for (j, (name, path)) in files.into_iter().take(60).enumerate() {
                let (n, p) = (name.clone(), path.clone());
                chips = chips.child(
                    div()
                        .id(SharedString::from(format!("st-imp-{i}-{j}")))
                        .min_h(z(32.0))
                        .px(z(10.0))
                        .flex()
                        .items_center()
                        .rounded(z(6.0))
                        .border_1()
                        .border_color(c("border"))
                        .bg(c("raised"))
                        .cursor_pointer()
                        .text_size(z(12.5))
                        .text_color(c("foreground"))
                        .hover(|s| s.bg(c("hover")))
                        .active(|s| s.bg(c("selected")))
                        .tab_index(0)
                        .focus_visible(controls::ring)
                        .on_click({
                            let go = Self::act_on(&me, move |this, w, cx| this.import_file(&n, &p, w, cx));
                            move |_, w, cx| go(w, cx)
                        })
                        .child(name.rsplit_once('.').map_or(name.as_str(), |(s, _)| s).to_owned()),
                );
            }
            out = out.child(div().flex().flex_col().gap(z(4.0)).child(div().text_size(z(12.0)).font_weight(FontWeight::MEDIUM).text_color(tone("muted")).child(source.label.clone())).child(chips));
        }
        out
    }

    fn theme_tools(&mut self, cx: &mut Context<Self>) -> Div {
        let mut out = div().w_full().flex().flex_col().gap(settings::z(10.0)).child(
            div()
                .flex()
                .flex_row()
                .gap(settings::z(8.0))
                .pb(settings::z(6.0))
                .child(button("st-theme-import", i18n::t("settings.import_theme"), self.act(cx, |this, w, cx| if this.look.is_importing() { this.look.importing = None } else { this.open_import(w, cx) })))
                .child(button("st-theme-new", i18n::t("settings.theme_new"), self.act(cx, |this, w, cx| this.new_theme(w, cx)))),
        );
        if self.look.is_importing() {
            out = out.child(self.import_panel(cx));
        }
        if self.look.editor.is_some() {
            out = out.child(self.theme_editor(cx));
        }
        out
    }

    pub(super) fn look_page(&mut self, cx: &mut Context<Self>) -> Div {
        let z = settings::z;
        let s = settings::get();
        let p = self.person.clone();
        let appearance = p.get("appearance").and_then(Value::as_str).map(str::to_owned);
        let appearance_pick = segmented(
            "ap",
            vec![(None, i18n::t("settings.system")), (Some("light".to_owned()), i18n::t("settings.light")), (Some("dark".to_owned()), i18n::t("settings.dark"))],
            &appearance,
            {
                let view = cx.entity().downgrade();
                move |value: Option<String>, w, cx| {
                    let _ = view.update(cx, |this, cx| this.change(w, cx, |q| match &value {
                        Some(v) => drop(q.insert("appearance".into(), json!(v))),
                        None => drop(q.remove("appearance")),
                    }));
                }
            },
        );
        let zoom = {
            let me = cx.entity().downgrade();
            let run = |name: &'static str| Self::act_on(&me, move |this, w, cx| this.run(name, json!({}), w, cx));
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(z(8.0))
                .child(controls::stepper("zoom", format!("{}%", (s.zoom * 100.0).round()), run("app.zoom_out"), run("app.zoom_in")))
                .child(button("st-zoom-reset", i18n::t("settings.reset"), run("app.zoom_reset")))
        };
        let ui_own = p.get("ui_font").and_then(Value::as_str).unwrap_or(DEFAULT_UI_FONT) != s.ui_font_family;
        let tools = self.theme_tools(cx);
        let layout = self.layout_picker(cx);
        let dark = self.theme_picker(false, cx);
        let light = self.theme_picker(true, cx);
        let ui_font = self.font_row("ui_font", i18n::t("settings.interface_font"), ui_own.then(|| i18n::t("settings.this_layout_sets_its_own")), s.ui_font_family.clone(), DEFAULT_UI_FONT, None::<Size>, cx);
        let sidebar = self.sidebar(cx);
        let sub = |t: String, top: f32| div().pt(z(top)).pb(z(6.0)).text_size(z(12.5)).font_weight(FontWeight::MEDIUM).text_color(tone("muted")).child(t);
        let flag = |this: &Self, cx: &mut Context<Self>, id: &'static str, key: &'static str, field: &'static str, title: &str, about: Option<String>| row(title.to_owned(), about, this.object_switch(cx, id, key, field));
        let mut page = div()
            .w_full()
            .child(controls::heading(i18n::t("settings.layout")))
            .child(layout)
            .child(controls::gap())
            .child(controls::heading(i18n::t("settings.theme")))
            .child(tools)
            .child(row(i18n::t("settings.appearance"), Some(i18n::t("settings.light_or_dark_or_as")), appearance_pick))
            .child(div().pt(z(12.0)).pb(z(8.0)).text_size(z(12.5)).text_color(tone("muted")).child(i18n::t("settings.for_the_layout_you_re_in")))
            .child(sub(i18n::t("settings.dark"), 0.0))
            .child(dark)
            .child(sub(i18n::t("settings.light"), 10.0))
            .child(light)
            .child(controls::gap())
            .child(controls::heading(i18n::t("settings.size_and_fonts")))
            .child(row("Zoom".to_owned(), Some(i18n::t("settings.everything_terminals_panels_text")), zoom))
            .child(ui_font)
            .child(controls::gap())
            .child(sidebar)
            .child(controls::gap())
            .child(controls::heading(i18n::t("settings.orca_card")))
            .child(flag(self, cx, "st-card-git", "space.card", "git", &i18n::t("settings.orca_card_git"), Some(i18n::t("settings.orca_card_git_about"))))
            .child(flag(self, cx, "st-card-ago", "space.card", "ago", &i18n::t("settings.orca_card_ago"), Some(i18n::t("settings.orca_card_ago_about"))))
            .child(flag(self, cx, "st-card-agents", "space.card", "agents", &i18n::t("settings.orca_card_agents"), Some(i18n::t("settings.orca_card_agents_about"))))
            .child(flag(self, cx, "st-btn-search", "sidebar.buttons", "search", &i18n::t("settings.orca_button_search"), Some(i18n::t("settings.orca_buttons_about"))))
            .child(flag(self, cx, "st-btn-bots", "sidebar.buttons", "bots", &i18n::t("settings.orca_button_bots"), None))
            .child(controls::gap())
            .child(controls::heading(i18n::t("settings.orca_status")))
            .child(flag(self, cx, "st-status-machine", "status.items", "machine", &i18n::t("settings.orca_status_machine"), None))
            .child(flag(self, cx, "st-status-branch", "status.items", "branch", &i18n::t("settings.orca_status_branch"), None))
            .child(flag(self, cx, "st-status-agents", "status.items", "agents", &i18n::t("settings.orca_status_agents"), None));
        if !cfg!(windows) {
            page = page.child(flag(self, cx, "st-status-awake", "status.items", "awake", &i18n::t("settings.orca_status_awake"), None));
        }
        page.child(flag(self, cx, "st-status-hint", "status.items", "hint", &i18n::t("settings.orca_status_hint"), None))
            .child(row(
                i18n::t("settings.orca_status_format"),
                Some(i18n::t("settings.orca_status_format_about")),
                self.shared_pick(cx, "status-format", "status.format", &[("total", "settings.orca_format_total"), ("working", "settings.orca_format_working")]),
            ))
            .child(controls::gap())
            .child(controls::heading(i18n::t("settings.orca_tabs_files")))
            .child(row(
                i18n::t("settings.orca_tab_order"),
                Some(i18n::t("settings.orca_tab_order_about")),
                self.shared_pick(cx, "tabs-order", "tabs.order", &[("strip", "settings.orca_order_strip"), ("recent", "settings.orca_order_recent")]),
            ))
            .child(row(i18n::t("settings.orca_ignored"), Some(i18n::t("settings.orca_ignored_about")), self.shared_switch(cx, "st-files-ignored", "files.ignored")))
            .child(controls::gap())
            .child(controls::heading(i18n::t("settings.window_heading")))
            .child(self.flag_row(cx, "st-reduce-effects", "reduce_effects", i18n::t("settings.reduce_effects"), i18n::t("settings.reduce_effects_about"), s.reduce_effects, false))
            .children(cfg!(target_os = "macos").then(|| self.flag_row(cx, "st-app-name", "show_app_name", i18n::t("settings.app_name"), i18n::t("settings.app_name_about"), s.show_app_name, false)))
            .child(controls::note(i18n::t("settings.window_limits")))
    }
}

#[cfg(test)]
mod tests {
    use super::{for_layout, ordered};
    use crate::settings::UiStyle;
    use super::super::theme_ui::Th;
    use std::collections::BTreeMap;

    #[test]
    fn layouts_go_shipped_first_then_by_name() {
        let got = ordered(["zeta", "win98", "alpha", "main", "terminals"].map(String::from).to_vec());
        assert_eq!(got, ["main", "terminals", "win98", "alpha", "zeta"]);
    }

    #[test]
    fn a_theme_named_like_a_layout_is_only_that_layouts() {
        let th = |n: &str| Th { name: n.into(), colors: BTreeMap::new(), style: UiStyle::default(), shipped: true };
        let layouts = vec!["main".to_owned(), "zz-layout".to_owned()];
        let names = |layout: &str| for_layout(vec![th("graphite"), th("zz-layout")], false, layout, &layouts).into_iter().map(|t| t.name).collect::<Vec<_>>();
        // "graphite" is dark; neither name is a shipped theme, so both count as dark.
        assert_eq!(names("main"), ["graphite"]);
        assert_eq!(names("zz-layout"), ["graphite", "zz-layout"]);
    }
}
