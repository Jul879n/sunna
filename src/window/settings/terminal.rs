//! Settings > Terminal: the text, the surface and the behavior of the
//! terminals. Values live in settings.json (the person's), so where a layout
//! sets its own, the page says so.

use super::*;
use controls::{button, row, segmented, stepper, switch};
use fonts::{DEFAULT_FONT, Size};

fn round2(n: f64) -> f64 {
    (n * 100.0).round() / 100.0
}

/// The settings value after moving `now` by `d` inside lo..=hi, as the page always did.
pub fn moved(now: f64, d: f64, lo: f64, hi: f64) -> f64 {
    round2(round2(now + d).clamp(lo, hi))
}

/// Numbers come back from the app as 32-bit floats (1.1 as 1.100000023841858).
pub fn same(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-4
}

/// `key` after pressing its switch: the opposite of `on`, and gone when that is what absent means (`fallback`).
pub fn flip_flag(q: &mut Object, key: &str, on: bool, fallback: bool) {
    if on != fallback {
        q.remove(key);
    } else {
        q.insert(key.into(), json!(!on));
    }
}

/// `terminal_padding` with one axis moved by `d`, in 0..=64; the other stays.
pub fn move_padding(q: &mut Object, shown: [f64; 2], index: usize, d: f64) {
    let mut now = q
        .get("terminal_padding")
        .and_then(Value::as_array)
        .and_then(|l| Some([l.first()?.as_f64()?, l.get(1)?.as_f64()?]))
        .unwrap_or(shown);
    now[index] = (now[index] + d).clamp(0.0, 64.0);
    q.insert("terminal_padding".into(), json!([store::num(now[0]), store::num(now[1])]));
}

/// `divider_width` after a step of `d` px: from what shows now (4 when it is the layout's), inside 0..=16.
pub fn move_divider(q: &mut Object, shown: Option<f32>, d: f64) {
    let at = q.get("divider_width").and_then(Value::as_f64).or(shown.map(f64::from)).unwrap_or(4.0);
    q.insert("divider_width".into(), store::num((at + d).clamp(0.0, 16.0)));
}

/// `shell` after picking `path` (None: the system's, which is absent).
pub fn set_shell(q: &mut Object, path: Option<&str>) {
    match path.map(str::trim).filter(|p| !p.is_empty()) {
        Some(p) => drop(q.insert("shell".into(), json!(p))),
        None => drop(q.remove("shell")),
    }
}

/// The shells installed here (`/etc/shells`, those that exist), for the picker; `now` stays in the list when it isn't one.
pub fn shell_choices(listed: &str, now: Option<&str>, exists: impl Fn(&str) -> bool) -> Vec<String> {
    let mut out: Vec<String> = listed
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('/') && exists(l))
        .map(str::to_owned)
        .collect();
    out.dedup();
    let mut seen = std::collections::HashSet::new();
    out.retain(|p| seen.insert(p.rsplit('/').next().unwrap_or(p).to_owned()));
    if let Some(now) = now.filter(|n| !out.iter().any(|o| o == n)) {
        out.push(now.to_owned());
    }
    out
}

impl SettingsView {
    /// A number of settings.json moved by the buttons: its row.
    fn number_row(&self, cx: &mut Context<Self>, id: &'static str, key: &'static str, title: String, about: Option<String>, now: f64, shown: String, (lo, hi, by): (f64, f64, f64)) -> Div {
        let mut step = |d: f64| {
            self.act(cx, move |this, w, cx| {
                this.change(w, cx, |q| {
                    let at = q.get(key).and_then(Value::as_f64).unwrap_or(now);
                    q.insert(key.into(), store::num(moved(at, d, lo, hi)));
                })
            })
        };
        row(title, about, stepper(id, shown, step(-by), step(by)))
    }

    /// The handler of a segmented control: `edit` puts the picked value into settings.json.
    fn act_pick<V: 'static>(view: &WeakEntity<Self>, edit: impl Fn(&mut Object, V) + 'static) -> impl Fn(V, &mut Window, &mut App) + 'static {
        let view = view.clone();
        move |value, w, cx| {
            let _ = view.update(cx, |this, cx| this.change(w, cx, |q| edit(q, value)));
        }
    }

    pub(super) fn flag_row(&self, cx: &mut Context<Self>, id: &'static str, key: &'static str, title: String, about: String, on: bool, fallback: bool) -> Div {
        row(title, Some(about), switch(id, on, self.act(cx, move |this, w, cx| this.change(w, cx, |q| flip_flag(q, key, on, fallback)))))
    }

    pub(super) fn terminal(&mut self, cx: &mut Context<Self>) -> Div {
        let s = settings::get();
        let p = self.person.clone();
        let own = |key: &str, composed: f64, def: f64| !same(p.get(key).and_then(Value::as_f64).unwrap_or(def), composed);
        let hint = |own: bool, about: Option<String>| if own { Some(i18n::t("settings.this_layout_sets_its_own")) } else { about };
        let font_own = p.get("font").and_then(Value::as_str).unwrap_or(DEFAULT_FONT) != s.font_family;
        let size_now = f64::from(s.font_size);
        let me = cx.entity().downgrade();
        let size = |d: f64| {
            Self::act_on(&me, move |this, w, cx| {
                this.change(w, cx, |q| {
                    let at = q.get("font_size").and_then(Value::as_f64).unwrap_or(size_now);
                    q.insert("font_size".into(), store::num((at + d).clamp(8.0, 32.0)));
                })
            })
        };
        let weight_now = f64::from(s.font_weight);
        let weight = |d: f64| {
            Self::act_on(&me, move |this, w, cx| {
                this.change(w, cx, |q| {
                    let at = q.get("font_weight").and_then(Value::as_f64).unwrap_or(weight_now);
                    q.insert("font_weight".into(), store::num((at + d).clamp(100.0, 900.0)));
                })
            })
        };
        let pad = [f64::from(s.terminal_padding[0]), f64::from(s.terminal_padding[1])];
        let padding = |this: &Self, cx: &mut Context<Self>, index: usize, id: &'static str, title: String| {
            let mut step = |d: f64| this.act(cx, move |this, w, cx| this.change(w, cx, |q| move_padding(q, pad, index, d)));
            row(title, None, stepper(id, format!("{} px", pad[index].round()), step(-2.0), step(2.0)))
        };
        let fixed = |this: &Self, cx: &mut Context<Self>, id: &'static str, key: &'static str, title: String, now: f32, def: f64, range: (f64, f64, f64)| {
            let now = f64::from(now);
            this.number_row(cx, id, key, title, hint(own(key, now, def), None), now, format!("{}%", (now * 100.0).round()), range)
        };
        let mut page = div().w_full().child(controls::heading(i18n::t("settings.term_text")));
        page = page.child(self.font_row(
            "font",
            i18n::t("settings.terminal_font"),
            hint(font_own, None),
            s.font_family.clone(),
            DEFAULT_FONT,
            Some(Size { shown: format!("{}", size_now.round()), minus: size(-1.0), plus: size(1.0) }),
            cx,
        ));
        page = page
            .child(row(
                i18n::t("settings.terminal_font_weight"),
                hint(own("font_weight", weight_now, 500.0), Some(i18n::t("settings.bold_text_stays_at_700"))),
                stepper("font_weight", format!("{}", s.font_weight), weight(-100.0), weight(100.0)),
            ))
            .child(self.flag_row(cx, "st-ligatures", "font_ligatures", i18n::t("settings.term_ligatures"), i18n::t("settings.term_ligatures_about"), s.font_ligatures, false))
            .child(fixed(self, cx, "line-height", "terminal_line_height", i18n::t("settings.terminal_line_height"), s.terminal_line_height, 1.0, (0.8, 2.0, 0.1)))
            .child(fixed(self, cx, "cell-width", "terminal_cell_width", i18n::t("settings.terminal_cell_width"), s.terminal_cell_width, 1.0, (0.8, 2.0, 0.05)))
            .child(self.number_row(cx, "minimum_contrast", "minimum_contrast", i18n::t("settings.term_contrast"), Some(i18n::t("settings.term_contrast_about")), f64::from(s.minimum_contrast), format!("{}", round2(f64::from(s.minimum_contrast))), (1.0, 21.0, 0.5)))
            .child(controls::gap())
            .child(controls::heading(i18n::t("settings.term_surface")))
            .child(self.number_row(cx, "inactive_opacity", "inactive_pane_opacity", i18n::t("settings.inactive_opacity"), Some(i18n::t("settings.inactive_opacity_about")), f64::from(s.inactive_pane_opacity), format!("{}%", (s.inactive_pane_opacity * 100.0).round()), (0.3, 1.0, 0.1)))
            .child(row(
                i18n::t("settings.divider_width"),
                Some(i18n::t("settings.divider_width_about")),
                {
                    let shown = s.divider_width;
                    let mut step = |d: f64| self.act(cx, move |this, w, cx| this.change(w, cx, |q| move_divider(q, shown, d)));
                    stepper("divider", shown.map_or_else(|| i18n::t("settings.divider_layout"), |w| format!("{} px", w.round())), step(-1.0), step(1.0))
                },
            ))
            .child(self.number_row(cx, "terminal_opacity", "terminal_opacity", i18n::t("settings.term_opacity"), Some(i18n::t("settings.term_opacity_about")), f64::from(s.terminal_opacity), format!("{}%", (s.terminal_opacity * 100.0).round()), (0.0, 1.0, 0.05)))
            .child(padding(self, cx, 0, "padding-x", i18n::t("settings.term_padding_x")))
            .child(padding(self, cx, 1, "padding-y", i18n::t("settings.term_padding_y")))
            .child(self.flag_row(cx, "st-scrollbars", "pane_scrollbars", i18n::t("settings.term_scrollbars"), i18n::t("settings.term_scrollbars_about"), s.pane_scrollbars, true))
            .child(controls::gap())
            .child(controls::heading(i18n::t("settings.term_cursor")))
            .child(row(i18n::t("settings.cursor_shape"), Some(i18n::t("settings.cursor_shape_about")), {
                let now = p.get("cursor_shape").and_then(Value::as_str).map(str::to_owned);
                let choices = vec![
                    (None, i18n::t("settings.cursor_program")),
                    (Some("block".to_owned()), i18n::t("settings.cursor_block")),
                    (Some("bar".to_owned()), i18n::t("settings.cursor_bar")),
                    (Some("underline".to_owned()), i18n::t("settings.cursor_underline")),
                ];
                segmented("cursor-shape", choices, &now, Self::act_pick(&me, |q, v: Option<String>| match v {
                    Some(v) => drop(q.insert("cursor_shape".into(), json!(v))),
                    None => drop(q.remove("cursor_shape")),
                }))
            }))
            .child(self.flag_row(cx, "st-cursor-blink", "cursor_blink", i18n::t("settings.cursor_blink"), i18n::t("settings.cursor_blink_about"), s.cursor_blink, false))
            .child(self.number_row(cx, "cursor-opacity", "cursor_opacity", i18n::t("settings.cursor_opacity"), Some(i18n::t("settings.cursor_opacity_about")), f64::from(s.cursor_opacity), format!("{}%", (s.cursor_opacity * 100.0).round()), (0.1, 1.0, 0.1)))
            .child(controls::gap())
            .child(controls::heading(i18n::t("settings.term_behavior")))
            .child(self.flag_row(cx, "st-copy-on-select", "copy_on_select", i18n::t("settings.term_copy_on_select"), i18n::t("settings.term_copy_on_select_about"), s.copy_on_select, true))
            .child(self.flag_row(cx, "st-osc52", "clipboard_write", i18n::t("settings.term_osc52"), i18n::t("settings.term_osc52_about"), s.clipboard_write, true))
            .child(self.flag_row(cx, "st-scroll-on-input", "scroll_on_input", i18n::t("settings.term_scroll_on_input"), i18n::t("settings.term_scroll_on_input_about"), s.scroll_on_input, true))
            .child(fixed(self, cx, "scroll-speed", "terminal_scroll_speed", i18n::t("settings.terminal_scroll_speed"), s.terminal_scroll_speed, 1.0, (0.1, 5.0, 0.1)))
            .child(self.flag_row(cx, "st-focus-follows", "focus_follows_mouse", i18n::t("settings.focus_follows_mouse"), i18n::t("settings.focus_follows_mouse_about"), s.focus_follows_mouse, false))
            .child(self.flag_row(cx, "st-confirm-close", "confirm_close_running", i18n::t("settings.confirm_close_running"), i18n::t("settings.confirm_close_running_about"), s.confirm_close_running, false))
            .child(self.flag_row(cx, "st-middle-paste", "middle_click_paste", i18n::t("settings.middle_paste"), i18n::t("settings.middle_paste_about"), s.middle_click_paste, settings::MIDDLE_PASTE_DEFAULT))
            .child(self.flag_row(cx, "st-trim-margin", "trim_copy_margin", i18n::t("settings.trim_margin"), i18n::t("settings.trim_margin_about"), s.trim_copy_margin, false));
        if cfg!(target_os = "macos") {
            page = page.child(self.flag_row(cx, "st-hide-mouse", "hide_mouse_on_type", i18n::t("settings.hide_mouse"), i18n::t("settings.hide_mouse_about"), s.hide_mouse_on_type, false));
        }
        if !cfg!(windows) {
            let now = p.get("shell").and_then(Value::as_str).map(str::to_owned);
            let listed = std::fs::read_to_string("/etc/shells").unwrap_or_default();
            let mut choices = vec![(None, i18n::t("settings.shell_default"))];
            // The shells people pick from; any other path in settings.json still shows, so it can be seen and changed.
            const COMMON: [&str; 5] = ["zsh", "bash", "fish", "nu", "sh"];
            choices.extend(shell_choices(&listed, now.as_deref(), |p| std::path::Path::new(p).exists()).into_iter().filter(|p| now.as_deref() == Some(p.as_str()) || COMMON.contains(&p.rsplit('/').next().unwrap_or(p))).map(|p| (Some(p.clone()), p.rsplit('/').next().unwrap_or(&p).to_owned())));
            page = page.child(row(i18n::t("settings.shell"), Some(i18n::t("settings.shell_about")), segmented("shell", choices, &now, Self::act_pick(&me, |q, v: Option<String>| set_shell(q, v.as_deref())))));
        }
        if cfg!(target_os = "macos") {
            page = page.child(row(
                i18n::t("settings.term_option"),
                Some(i18n::t("settings.term_option_about")),
                button("st-option-link", i18n::t("settings.term_open_keyboard"), self.act(cx, |this, w, cx| this.go("keyboard", w, cx))),
            ));
        }
        page.child(controls::gap()).child(controls::heading(i18n::t("settings.quick_heading"))).child(self.quick_commands(cx))
    }
}

impl SettingsView {
    /// Quick commands: the list with Edit and Delete on each, and Add (`window/quick.rs` keeps them).
    fn quick_commands(&mut self, cx: &mut Context<Self>) -> Div {
        let list = super::super::quick::load();
        let mut out = div().w_full().flex().flex_col().child(div().text_size(settings::z(12.5)).line_height(settings::z(18.0)).text_color(tone("muted")).pb(settings::z(8.0)).child(i18n::t("settings.quick_about")));
        if list.is_empty() {
            out = out.child(div().text_size(settings::z(13.0)).text_color(tone("muted")).pb(settings::z(8.0)).child(i18n::t("settings.quick_empty")));
        }
        for (i, q) in list.into_iter().enumerate() {
            let (edit, delete) = (q.name.clone(), q.name.clone());
            let buttons = div()
                .flex()
                .gap(settings::z(6.0))
                .child(button(format!("quick-edit-{i}"), i18n::t("settings.quick_edit"), self.act(cx, move |this, w, cx| this.run("quick.edit", json!({ "name": edit }), w, cx))))
                .child(button(format!("quick-del-{i}"), i18n::t("settings.quick_delete"), self.act(cx, move |this, w, cx| this.run("quick.delete", json!({ "name": delete }), w, cx))));
            out = out.child(row(q.name, Some(super::super::quick::preview(&q.command)), buttons));
        }
        out.child(div().flex().child(button("quick-add", i18n::t("settings.quick_add"), self.act(cx, |this, w, cx| this.run("quick.edit", json!({}), w, cx)))))
    }
}

#[cfg(test)]
mod tests {
    use super::{Object, flip_flag, move_divider, move_padding, moved, same, set_shell, shell_choices};
    use serde_json::Value;

    fn obj(text: &str) -> Object {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn numbers_step_inside_their_range_and_round_to_two_places() {
        assert_eq!(moved(1.0, 0.1, 0.8, 2.0), 1.1);
        assert_eq!(moved(0.8, -0.1, 0.8, 2.0), 0.8);
        assert_eq!(moved(1.95, 0.1, 0.8, 2.0), 2.0);
        assert_eq!(moved(0.1 + 0.2, 0.1, 0.0, 1.0), 0.4);
        assert_eq!(moved(500.0, 100.0, 100.0, 900.0), 600.0);
        assert!(same(1.1, 1.100000023841858));
        assert!(!same(1.1, 1.2));
    }

    #[test]
    fn a_switch_is_dropped_when_back_to_what_absent_means() {
        let mut q = obj(r#"{"copy_on_select": false, "zoom": 1}"#);
        flip_flag(&mut q, "copy_on_select", false, true);
        assert_eq!(Value::Object(q), serde_json::from_str::<Value>(r#"{"zoom":1}"#).unwrap());
        let mut q = obj("{}");
        flip_flag(&mut q, "copy_on_select", true, true);
        assert_eq!(q["copy_on_select"], false);
        let mut q = obj("{}");
        flip_flag(&mut q, "font_ligatures", false, false);
        assert_eq!(q["font_ligatures"], true);
        flip_flag(&mut q, "font_ligatures", true, false);
        assert!(q.is_empty());
    }

    #[test]
    fn padding_moves_one_axis_and_keeps_the_other_inside_0_to_64() {
        let mut q = obj("{}");
        move_padding(&mut q, [10.0, 6.0], 0, 2.0);
        assert_eq!(Value::Object(q.clone()), serde_json::from_str::<Value>(r#"{"terminal_padding":[12,6]}"#).unwrap());
        move_padding(&mut q, [10.0, 6.0], 1, -10.0);
        assert_eq!(q["terminal_padding"], serde_json::json!([12, 0]));
        let mut q = obj(r#"{"terminal_padding": [63, 1]}"#);
        move_padding(&mut q, [10.0, 6.0], 0, 2.0);
        assert_eq!(q["terminal_padding"][0], 64);
    }

    #[test]
    fn the_divider_steps_from_what_shows_and_stays_inside_0_to_16() {
        let mut q = obj("{}");
        move_divider(&mut q, None, 1.0);
        assert_eq!(q["divider_width"], 5, "from the layout's, taken as 4");
        move_divider(&mut q, None, -10.0);
        assert_eq!(q["divider_width"], 0);
        move_divider(&mut q, Some(15.5), 2.0);
        assert_eq!(q["divider_width"], 2, "what is stored wins over what shows");
        let mut q = obj("{}");
        move_divider(&mut q, Some(15.5), 2.0);
        assert_eq!(q["divider_width"], 16);
    }

    #[test]
    fn the_shell_is_stored_and_the_default_removes_it() {
        let mut q = obj(r#"{"zoom":1}"#);
        set_shell(&mut q, Some("/bin/zsh"));
        assert_eq!(q["shell"], "/bin/zsh");
        set_shell(&mut q, None);
        assert!(!q.contains_key("shell"));
        set_shell(&mut q, Some("  "));
        assert!(!q.contains_key("shell"));
    }

    #[test]
    fn the_shell_picker_lists_installed_shells_once_by_name() {
        let listed = "# comment\n/bin/sh\n/bin/zsh\n/usr/bin/zsh\n/bin/gone\nbash\n/bin/bash\n";
        let have = |p: &str| p != "/bin/gone";
        assert_eq!(shell_choices(listed, None, have), ["/bin/sh", "/bin/zsh", "/bin/bash"]);
        assert_eq!(shell_choices(listed, Some("/opt/fish"), have).last().map(String::as_str), Some("/opt/fish"));
        assert_eq!(shell_choices(listed, Some("/bin/zsh"), have).len(), 3);
    }
}
