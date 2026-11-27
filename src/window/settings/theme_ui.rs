//! Themes in Settings > Appearance: the card that draws one (with its own
//! colors and style) and the editor for a theme of your own.

use super::theme_import::{ThemeDoc, free_name};
use super::*;
use crate::settings::UiStyle;
use controls::{button, field_state, icon_button, row, segmented, stepper};
use gpui_base::input::{InputEvent, InputState};
use std::collections::{BTreeMap, BTreeSet};

const PENCIL: &[u8] = include_bytes!("../../../assets/icons/settings/pencil.svg");
const CHECK: &[u8] = include_bytes!("../../../assets/icons/settings/check.svg");

/// A theme as the cards draw it.
#[derive(Clone, Debug)]
pub struct Th {
    pub name: String,
    pub colors: BTreeMap<String, String>,
    pub style: UiStyle,
    pub shipped: bool,
}

/// A theme drawn small. Pressing it uses it; a personal one also offers to edit it.
pub fn card(id: String, th: &Th, selected: bool, (w, h): (f32, f32), on_click: Option<Act>, on_edit: Option<(String, Act)>) -> Stateful<Div> {
    let col = |k: &str| th.colors.get(k).map_or_else(|| c("#808080"), |v| c(v));
    let z = settings::z;
    let dot = |k: &str| div().size(z(6.0)).flex_none().rounded(z(3.0)).bg(col(k));
    let bar = |k: &str, opacity: f32, frac: f32| div().h(z(5.0)).w(z(((w - 44.0) * frac).round())).rounded(z(2.0)).bg(col(k)).opacity(opacity);
    let thumb = div()
        .relative()
        .w(z(w))
        .h(z(h))
        .flex_none()
        .flex()
        .flex_row()
        .rounded(z(th.style.pane_radius.unwrap_or(6.0)))
        .overflow_hidden()
        .bg(col("background"))
        .child(div().w(z(22.0)).h_full().flex_none().bg(col("chrome")))
        .child(
            div()
                .flex_1()
                .h_full()
                .flex()
                .flex_col()
                .child(div().w_full().h(z(12.0)).flex_none().px(z(5.0)).flex().flex_row().items_center().gap(z(3.0)).bg(col("chrome")).child(dot("ansi1")).child(dot("ansi2")))
                .child(
                    div()
                        .flex_1()
                        .p(z(7.0))
                        .flex()
                        .flex_col()
                        .gap(z(5.0))
                        .child(bar("terminal_foreground", 0.7, 0.8))
                        .child(bar("ansi2", 0.45, 0.55))
                        .child(bar("ansi4", 0.6, 0.7))
                        .child(bar("ansi5", 0.3, 0.45)),
                ),
        )
        .child(div().absolute().right(z(6.0)).bottom(z(6.0)).px(z(6.0)).py(z(2.0)).rounded(z(th.style.radius.unwrap_or(6.0))).bg(col("accent")).text_color(col("accent_foreground")).text_size(z(10.0)).child("Aa"));
    let personal = !th.shipped;
    let label = th.name.replace('-', " ");
    let clickable = on_click.is_some();
    let mut name_row = div()
        .w(z(w))
        .min_w_0()
        .h(z(32.0))
        .flex()
        .flex_row()
        .items_center()
        .gap(z(5.0))
        .child(div().flex_1().min_w_0().truncate().text_size(z(12.5)).text_color(if selected { c("foreground") } else { c("muted") }).child(label));
    if personal {
        name_row = name_row.child(div().flex_none().px(z(5.0)).py(z(1.0)).rounded(z(5.0)).bg(c("hover")).text_size(z(10.5)).text_color(tone("muted")).child(i18n::t("settings.theme_personal")));
        if let Some((edit_id, edit)) = on_edit {
            name_row = name_row.child(icon_button(edit_id, PENCIL, act(move |w, cx| {
                cx.stop_propagation();
                edit(w, cx)
            })));
        }
    }
    if selected {
        name_row = name_row.child(svg().data(CHECK).size(z(12.0)).flex_none().text_color(c("accent")));
    }
    let mut out = div()
        .id(SharedString::from(id))
        .flex()
        .flex_col()
        .gap(z(4.0))
        .p(z(6.0))
        .rounded(z(10.0))
        .when(selected, |d| d.border_2().border_color(c("accent")))
        .when(!selected, |d| d.border_1().border_color(c("border")))
        .child(thumb)
        .child(name_row);
    if let Some(on) = on_click {
        out = out
            .cursor_pointer()
            .hover(|s| s.bg(c("raised")))
            .active(|s| s.bg(c("selected")))
            .tab_index(0)
            .focus_visible(|s| s.border_2().border_color(c("foreground")))
            .on_click(move |_, w, cx| on(w, cx));
    }
    let _ = clickable;
    out
}

// Editor ---------------------------------------------------------------------

pub const UI_COLORS: [&str; 7] = ["accent", "chrome", "canvas", "surface", "border", "foreground", "muted"];
const ANSI_LABELS: [&str; 8] = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];
/// (key, min, max, value when first set, label)
pub const NUMBERS: [(&str, f32, f32, f32, &str); 5] = [
    ("radius", 0.0, 24.0, 6.0, "settings.style_radius"),
    ("pane_radius", 0.0, 24.0, 6.0, "settings.style_pane_radius"),
    ("pane_gap", 0.0, 32.0, 0.0, "settings.style_pane_gap"),
    ("border_width", 0.0, 4.0, 1.0, "settings.style_border_width"),
    ("font_size", 6.0, 72.0, 14.0, "settings.style_font_size"),
];

fn is_hex6(v: &str) -> bool {
    v.len() == 7 && v.starts_with('#') && v[1..].chars().all(|c| c.is_ascii_hexdigit())
}

fn is_hex(v: &str) -> bool {
    is_hex6(v) || (v.len() == 9 && v.starts_with('#') && v[1..].chars().all(|c| c.is_ascii_hexdigit()))
}

pub fn style_get(s: &UiStyle, key: &str) -> Option<f32> {
    match key {
        "radius" => s.radius,
        "pane_radius" => s.pane_radius,
        "pane_gap" => s.pane_gap,
        "border_width" => s.border_width,
        _ => s.font_size,
    }
}

pub fn style_set(s: &mut UiStyle, key: &str, v: Option<f32>) {
    match key {
        "radius" => s.radius = v,
        "pane_radius" => s.pane_radius = v,
        "pane_gap" => s.pane_gap = v,
        "border_width" => s.border_width = v,
        _ => s.font_size = v,
    }
}

/// One step of a style number: unset starts at `def`; going under `min` unsets it.
pub fn step_style(s: &mut UiStyle, key: &str, (min, max, def): (f32, f32, f32), d: f32) {
    let n = style_get(s, key).map_or(def, |v| v + d);
    style_set(s, key, if n < min { None } else { Some(n.min(max)) });
}

/// A theme being written: what is typed stays as typed until it is saved.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Draft {
    pub name: String,
    pub background: String,
    pub foreground: String,
    pub selection: String,
    pub cursor: String,
    pub ansi: Vec<String>,
    pub colors: BTreeMap<String, String>,
    pub style: UiStyle,
}

impl Draft {
    pub fn from_doc(name: &str, d: &ThemeDoc) -> Self {
        Self {
            name: name.to_owned(),
            background: d.background.clone(),
            foreground: d.foreground.clone(),
            selection: d.selection.clone(),
            cursor: d.cursor.clone().unwrap_or_default(),
            ansi: d.ansi.clone(),
            colors: d.colors.clone(),
            style: d.style.clone(),
        }
    }

    /// A new theme from one that is shown: its terminal colors and style.
    pub fn from_theme(name: &str, th: &Th) -> Self {
        let g = |k: &str| th.colors.get(k).cloned().unwrap_or_default();
        Self {
            name: name.to_owned(),
            background: g("background"),
            foreground: g("terminal_foreground"),
            selection: g("selection"),
            cursor: g("cursor"),
            ansi: (0..16).map(|i| g(&format!("ansi{i}"))).collect(),
            colors: BTreeMap::new(),
            style: th.style.clone(),
        }
    }

    /// What typing in the field called `key` changes.
    pub fn set(&mut self, key: &str, v: &str) {
        let v = v.trim().to_owned();
        match key {
            "name" => self.name = v,
            "background" => self.background = v,
            "foreground" => self.foreground = v,
            "selection" => self.selection = v,
            "cursor" => self.cursor = v,
            "style.font" => self.style.font = (!v.is_empty()).then_some(v),
            "style.ui_font" => self.style.ui_font = (!v.is_empty()).then_some(v),
            _ => {
                if let Some(i) = key.strip_prefix("ansi").and_then(|i| i.parse::<usize>().ok()) {
                    if let Some(slot) = self.ansi.get_mut(i) {
                        *slot = v;
                    }
                } else if let Some(k) = key.strip_prefix("colors.") {
                    if v.is_empty() {
                        self.colors.remove(k);
                    } else {
                        self.colors.insert(k.to_owned(), v);
                    }
                }
            }
        }
    }

    /// The fields that keep it from being saved.
    pub fn invalid(&self) -> BTreeSet<String> {
        let mut bad = BTreeSet::new();
        if self.name.is_empty() || !self.name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            bad.insert("name".to_owned());
        }
        for (k, v) in [("background", &self.background), ("foreground", &self.foreground), ("selection", &self.selection)] {
            if !is_hex6(v) {
                bad.insert(k.to_owned());
            }
        }
        if !self.cursor.is_empty() && !is_hex6(&self.cursor) {
            bad.insert("cursor".to_owned());
        }
        for (i, v) in self.ansi.iter().enumerate() {
            if !is_hex6(v) {
                bad.insert(format!("ansi{i}"));
            }
        }
        for (k, v) in &self.colors {
            if !v.is_empty() && !is_hex(v) {
                bad.insert(format!("colors.{k}"));
            }
        }
        bad
    }

    /// The file's content: empty values left out.
    pub fn doc(&self) -> ThemeDoc {
        ThemeDoc {
            background: self.background.clone(),
            foreground: self.foreground.clone(),
            selection: self.selection.clone(),
            cursor: (!self.cursor.is_empty()).then(|| self.cursor.clone()),
            ansi: self.ansi.clone(),
            colors: self.colors.iter().filter(|(_, v)| !v.is_empty()).map(|(k, v)| (k.clone(), v.clone())).collect(),
            style: self.style.clone(),
        }
    }

    /// The draft as the card draws it: the current colors with the draft's valid ones over them.
    pub fn preview(&self, base: &BTreeMap<String, String>) -> Th {
        let mut colors = base.clone();
        let mut put = |k: &str, v: &str| {
            if is_hex(v) {
                colors.insert(k.to_owned(), v.to_owned());
            }
        };
        put("background", &self.background);
        put("terminal_foreground", &self.foreground);
        put("selection", &self.selection);
        for (i, v) in self.ansi.iter().enumerate() {
            put(&format!("ansi{i}"), v);
        }
        for (k, v) in &self.colors {
            put(k, v);
        }
        Th { name: if self.name.is_empty() { "theme".into() } else { self.name.clone() }, colors, style: self.style.clone(), shipped: false }
    }
}

pub struct Editor {
    /// Name and text of the file being edited (None: a new theme).
    pub original: Option<(String, String)>,
    pub draft: Draft,
    pub inputs: Vec<(String, Entity<InputState>)>,
    pub focused: Option<String>,
    pub confirm_delete: bool,
}

impl Editor {
    pub fn new(original: Option<(String, String)>, draft: Draft, window: &mut Window, cx: &mut Context<SettingsView>) -> Self {
        let mut keys: Vec<(String, String, String)> = vec![("name".into(), draft.name.clone(), String::new())];
        let hex = "#rrggbb".to_owned();
        keys.push(("background".into(), draft.background.clone(), hex.clone()));
        keys.push(("foreground".into(), draft.foreground.clone(), hex.clone()));
        keys.push(("selection".into(), draft.selection.clone(), hex.clone()));
        keys.push(("cursor".into(), draft.cursor.clone(), hex.clone()));
        for (i, v) in draft.ansi.iter().enumerate() {
            keys.push((format!("ansi{i}"), v.clone(), hex.clone()));
        }
        for k in UI_COLORS {
            keys.push((format!("colors.{k}"), draft.colors.get(k).cloned().unwrap_or_default(), hex.clone()));
        }
        keys.push(("style.font".into(), draft.style.font.clone().unwrap_or_default(), String::new()));
        keys.push(("style.ui_font".into(), draft.style.ui_font.clone().unwrap_or_default(), String::new()));
        let mut inputs = Vec::new();
        for (key, value, placeholder) in keys {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
            input.update(cx, |s, cx| {
                s.set_editor_style(controls::input_style());
                s.set_value(value, window, cx);
            });
            let k = key.clone();
            cx.subscribe_in(&input, window, move |this, input, ev: &InputEvent, _w, cx| {
                let Some(e) = this.look.editor.as_mut() else { return };
                match ev {
                    InputEvent::Change => {
                        let v = input.read(cx).value().to_string();
                        e.draft.set(&k, &v);
                    }
                    InputEvent::Focus => e.focused = Some(k.clone()),
                    InputEvent::Blur => {
                        if e.focused.as_deref() == Some(k.as_str()) {
                            e.focused = None;
                        }
                    }
                    _ => return,
                }
                cx.notify();
            })
            .detach();
            inputs.push((key, input));
        }
        Self { original, draft, inputs, focused: None, confirm_delete: false }
    }

    fn input(&self, key: &str) -> &Entity<InputState> {
        &self.inputs.iter().find(|(k, _)| k == key).expect("every field exists").1
    }
}

/// The text of a style number: the value, or a dash while unset.
fn shown(v: Option<f32>) -> String {
    v.map_or_else(|| "—".to_owned(), |v| format!("{v}"))
}

impl SettingsView {
    /// A new theme or the edit of a personal one opens under the Theme heading.
    pub(super) fn open_editor(&mut self, original: Option<(String, String)>, draft: Draft, window: &mut Window, cx: &mut Context<Self>) {
        self.look.editor = Some(Editor::new(original, draft, window, cx));
        self.look.importing = None;
        cx.notify();
    }

    pub(super) fn edit_theme(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let text = std::fs::read_to_string(crate::mods::app_dir().join("themes").join(format!("{name}.json"))).unwrap_or_default();
        match serde_json::from_str::<ThemeDoc>(&text) {
            Ok(doc) => self.open_editor(Some((name.to_owned(), text)), Draft::from_doc(name, &doc), window, cx),
            Err(e) => self.notify(json!({ "level": "error", "title": i18n::t("settings.theme_save_failed"), "body": e.to_string() }), window, cx),
        }
    }

    pub(super) fn new_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let all = shown_themes();
        let now = settings::get().theme.clone();
        let Some(from) = all.iter().find(|t| t.name == now).or(all.first()).cloned() else { return };
        let taken: Vec<String> = all.iter().map(|t| t.name.clone()).collect();
        let name = free_name(&format!("{}-mio", from.name), &taken);
        self.open_editor(None, Draft::from_theme(&name, &from), window, cx);
    }

    fn save_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(e) = self.look.editor.as_ref() else { return };
        if !e.draft.invalid().is_empty() {
            return;
        }
        let name = e.draft.name.clone();
        let original = e.original.clone();
        let taken: Vec<String> = shown_themes().into_iter().map(|t| t.name).collect();
        if original.as_ref().map(|(n, _)| n) != Some(&name) && taken.contains(&name) {
            self.notify(json!({ "level": "error", "title": i18n::t("settings.theme_name_taken") }), window, cx);
            return;
        }
        let dir = crate::mods::app_dir().join("themes");
        let write = std::fs::create_dir_all(&dir).and_then(|_| crate::persist::write_atomic(&dir.join(format!("{name}.json")), e.draft.doc().to_file()));
        match write {
            Ok(()) => {
                if let Some((old, _)) = &original
                    && *old != name
                {
                    let _ = std::fs::remove_file(dir.join(format!("{old}.json")));
                }
                self.run("app.theme", json!({ "name": name }), window, cx);
                self.look.editor = None;
            }
            Err(err) => self.notify(json!({ "level": "error", "title": i18n::t("settings.theme_save_failed"), "body": err.to_string() }), window, cx),
        }
        cx.notify();
    }

    fn delete_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((name, _)) = self.look.editor.as_ref().and_then(|e| e.original.clone()) else { return };
        match std::fs::remove_file(crate::mods::app_dir().join("themes").join(format!("{name}.json"))) {
            Ok(()) => {
                if settings::get().theme == name {
                    let fallback = if settings::theme_is_light(&name) { "paper" } else { "graphite" };
                    self.run("app.theme", json!({ "name": fallback }), window, cx);
                }
                self.look.editor = None;
            }
            Err(err) => self.notify(json!({ "level": "error", "title": i18n::t("settings.theme_save_failed"), "body": err.to_string() }), window, cx),
        }
        cx.notify();
    }

    fn swatch(&self, key: &str, label: &str) -> Div {
        let Some(e) = self.look.editor.as_ref() else { return div() };
        let value = match key {
            "background" => e.draft.background.clone(),
            "foreground" => e.draft.foreground.clone(),
            "selection" => e.draft.selection.clone(),
            "cursor" => e.draft.cursor.clone(),
            k => k.strip_prefix("ansi").and_then(|i| i.parse::<usize>().ok()).and_then(|i| e.draft.ansi.get(i).cloned()).or_else(|| k.strip_prefix("colors.").map(|k| e.draft.colors.get(k).cloned().unwrap_or_default())).unwrap_or_default(),
        };
        let bad = e.draft.invalid().contains(key);
        let z = settings::z;
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(z(8.0))
            .child(div().w(z(78.0)).flex_none().text_size(z(12.0)).text_color(tone("muted")).child(label.to_owned()))
            .child(field_state(e.input(key), 112.0, e.focused.as_deref() == Some(key), bad))
            .child(div().size(z(20.0)).flex_none().rounded(z(4.0)).border_1().border_color(c("border")).bg(if is_hex(&value) { c(&value) } else { gpui::transparent_black() }))
    }

    /// The editor panel: the draft's colors and style next to a live card.
    pub(super) fn theme_editor(&mut self, cx: &mut Context<Self>) -> Div {
        let z = settings::z;
        let Some(e) = self.look.editor.as_ref() else { return div() };
        let bad = e.draft.invalid();
        let preview = e.draft.preview(&settings::theme());
        let personal = e.original.is_some();
        let confirm = e.confirm_delete;
        let style = e.draft.style.clone();
        let name_bad = bad.contains("name");
        let name_input = e.input("name").clone();
        let focus_name = e.focused.as_deref() == Some("name");
        let heading = |text: String| div().pt(z(10.0)).pb(z(4.0)).text_size(z(12.0)).font_weight(FontWeight::SEMIBOLD).text_color(tone("muted")).child(text.to_uppercase());
        let mut terminal = div().flex().flex_col().gap(z(6.0));
        for (k, l) in [("background", "background"), ("foreground", "foreground"), ("selection", "selection"), ("cursor", "cursor")] {
            terminal = terminal.child(self.swatch(k, l));
        }
        let mut ansi = div().flex().flex_row().flex_wrap().gap_x(z(20.0)).gap_y(z(6.0));
        for i in 0..16 {
            let l = ANSI_LABELS[i % 8];
            ansi = ansi.child(self.swatch(&format!("ansi{i}"), &if i < 8 { l.to_owned() } else { format!("{l}+") }));
        }
        let mut ui = div().flex().flex_row().flex_wrap().gap_x(z(20.0)).gap_y(z(6.0));
        for k in UI_COLORS {
            ui = ui.child(self.swatch(&format!("colors.{k}"), k));
        }
        let me = cx.entity().downgrade();
        let mut page = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(z(6.0))
            .p(z(14.0))
            .mb(z(10.0))
            .rounded(z(10.0))
            .border_1()
            .border_color(c("border"))
            .bg(c("surface"))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(z(16.0))
                    .items_start()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(z(6.0))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap(z(8.0))
                                    .child(div().w(z(78.0)).flex_none().text_size(z(12.0)).text_color(tone("muted")).child(i18n::t("settings.theme_name")))
                                    .child(field_state(&name_input, 200.0, focus_name, name_bad)),
                            )
                            .child(heading(i18n::t("settings.theme_terminal_colors")))
                            .child(terminal),
                    )
                    .child(div().flex_none().child(card("st-te-preview".into(), &preview, false, (200.0, 124.0), None, None))),
            )
            .child(ansi)
            .child(heading(i18n::t("settings.theme_ui_colors")))
            .child(ui)
            .child(heading(i18n::t("settings.theme_style")));
        for (k, min, max, def, label) in NUMBERS {
            let v = style_get(&style, k);
            let step = |d: f32| {
                SettingsView::act_on(&me, move |this, _, _| {
                    if let Some(e) = this.look.editor.as_mut() {
                        step_style(&mut e.draft.style, k, (min, max, def), d);
                    }
                })
            };
            page = page.child(row(i18n::t(label), None, stepper(&format!("te-n-{k}"), shown(v), step(-1.0), step(1.0))));
        }
        let shadows: Vec<(Option<String>, String)> = vec![(None, "—".into()), (Some("none".into()), "none".into()), (Some("sm".into()), "sm".into()), (Some("md".into()), "md".into()), (Some("lg".into()), "lg".into())];
        let view = me.clone();
        page = page.child(row(
            i18n::t("settings.style_shadow"),
            None,
            segmented("te-sh", shadows, &style.shadow, move |v: Option<String>, _w, cx| {
                let _ = view.update(cx, |this, cx| {
                    if let Some(e) = this.look.editor.as_mut() {
                        e.draft.style.shadow = v;
                    }
                    cx.notify();
                });
            }),
        ));
        for (k, label) in [("style.font", "settings.style_font"), ("style.ui_font", "settings.style_ui_font")] {
            let (input, focused) = self.look.editor.as_ref().map(|e| (e.input(k).clone(), e.focused.as_deref() == Some(k))).expect("editing");
            page = page.child(row(i18n::t(label), None, field_state(&input, 200.0, focused, false)));
        }
        let save = button("st-te-save", i18n::t("settings.theme_save"), SettingsView::act_on(&me, |this, w, cx| this.save_theme(w, cx))).opacity(if bad.is_empty() { 1.0 } else { 0.5 });
        let mut actions = div().flex().flex_row().gap(z(8.0)).pt(z(10.0)).child(save).child(button("st-te-cancel", i18n::t("settings.cancel"), SettingsView::act_on(&me, |this, _, _| this.look.editor = None)));
        if personal {
            actions = actions.child(button(
                "st-te-delete",
                if confirm { i18n::t("settings.confirm_delete") } else { i18n::t("settings.theme_delete") },
                SettingsView::act_on(&me, |this, w, cx| {
                    if this.look.editor.as_ref().is_some_and(|e| e.confirm_delete) {
                        this.delete_theme(w, cx);
                    } else if let Some(e) = this.look.editor.as_mut() {
                        e.confirm_delete = true;
                    }
                }),
            ));
        }
        page.child(actions)
    }
}

/// Every theme that can be shown, as cards draw them.
pub fn shown_themes() -> Vec<Th> {
    settings::themes().into_iter().map(|(name, colors, style, shipped)| Th { name, colors, style, shipped }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::prelude::v1::test; // gpui::* (through super) has a `test` too

    fn doc() -> ThemeDoc {
        ThemeDoc {
            background: "#101010".into(),
            foreground: "#eeeeee".into(),
            selection: "#202020".into(),
            cursor: None,
            ansi: (0..16).map(|i| format!("#{i:02x}0000")).collect(),
            colors: BTreeMap::new(),
            style: UiStyle::default(),
        }
    }

    #[test]
    fn a_draft_from_a_file_saves_back_the_same_file() {
        let d = doc();
        assert_eq!(Draft::from_doc("x", &d).doc(), d);
        assert!(Draft::from_doc("x", &d).invalid().is_empty());
    }

    #[test]
    fn an_invalid_hex_or_name_is_named_and_empty_values_are_left_out() {
        let mut d = Draft::from_doc("my-theme", &doc());
        d.set("background", "#12345");
        d.set("ansi3", "red");
        d.set("cursor", "#abc");
        d.set("colors.accent", "#zzzzzz");
        d.set("name", "Bad Name");
        let bad: Vec<String> = d.invalid().into_iter().collect();
        assert_eq!(bad, ["ansi3", "background", "colors.accent", "cursor", "name"]);
        d.set("background", "#123456");
        d.set("ansi3", "#030000");
        d.set("cursor", "");
        d.set("colors.accent", "");
        d.set("name", "good-1");
        assert!(d.invalid().is_empty());
        let out = d.doc();
        assert_eq!(out.cursor, None);
        assert!(out.colors.is_empty());
        d.set("colors.accent", "#aabbccdd");
        assert!(d.invalid().is_empty());
        assert_eq!(d.doc().colors["accent"], "#aabbccdd");
    }

    #[test]
    fn style_numbers_start_at_their_default_and_unset_under_the_minimum() {
        let mut s = UiStyle::default();
        step_style(&mut s, "radius", (0.0, 24.0, 6.0), 1.0);
        assert_eq!(s.radius, Some(6.0));
        step_style(&mut s, "radius", (0.0, 24.0, 6.0), 1.0);
        assert_eq!(s.radius, Some(7.0));
        s.radius = Some(24.0);
        step_style(&mut s, "radius", (0.0, 24.0, 6.0), 1.0);
        assert_eq!(s.radius, Some(24.0));
        s.radius = Some(0.0);
        step_style(&mut s, "radius", (0.0, 24.0, 6.0), -1.0);
        assert_eq!(s.radius, None);
        step_style(&mut s, "pane_gap", (0.0, 32.0, 0.0), -1.0);
        assert_eq!(s.pane_gap, Some(0.0));
    }

    #[test]
    fn a_new_theme_takes_the_terminal_colors_and_style_of_the_shown_one() {
        let mut colors = BTreeMap::new();
        for (k, v) in [("background", "#111111"), ("terminal_foreground", "#222222"), ("selection", "#333333"), ("cursor", "#444444")] {
            colors.insert(k.to_owned(), v.to_owned());
        }
        for i in 0..16 {
            colors.insert(format!("ansi{i}"), format!("#{i:02x}1111"));
        }
        let th = Th { name: "ink".into(), colors, style: UiStyle { radius: Some(9.0), ..Default::default() }, shipped: true };
        let d = Draft::from_theme("ink-mio", &th);
        assert_eq!((d.foreground.as_str(), d.cursor.as_str(), d.ansi.len(), d.style.radius), ("#222222", "#444444", 16, Some(9.0)));
        assert!(d.invalid().is_empty());
        let p = d.preview(&BTreeMap::from([("accent".to_owned(), "#abcdef".to_owned())]));
        assert_eq!(p.colors["accent"], "#abcdef");
        assert_eq!(p.colors["ansi15"], "#0f1111");
    }
}
