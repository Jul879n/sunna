//! Settings > Terminal: the text, the surface and the behavior of the
//! terminals. Values live in settings.json (the person's), so where a layout
//! sets its own, the page says so.

use super::*;
use controls::{button, row, stepper, switch};
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
    if on == !fallback {
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

    fn flag_row(&self, cx: &mut Context<Self>, id: &'static str, key: &'static str, title: String, about: String, on: bool, fallback: bool) -> Div {
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
            .child(self.number_row(cx, "terminal_opacity", "terminal_opacity", i18n::t("settings.term_opacity"), Some(i18n::t("settings.term_opacity_about")), f64::from(s.terminal_opacity), format!("{}%", (s.terminal_opacity * 100.0).round()), (0.0, 1.0, 0.05)))
            .child(padding(self, cx, 0, "padding-x", i18n::t("settings.term_padding_x")))
            .child(padding(self, cx, 1, "padding-y", i18n::t("settings.term_padding_y")))
            .child(self.flag_row(cx, "st-scrollbars", "pane_scrollbars", i18n::t("settings.term_scrollbars"), i18n::t("settings.term_scrollbars_about"), s.pane_scrollbars, true))
            .child(controls::gap())
            .child(controls::heading(i18n::t("settings.term_behavior")))
            .child(self.flag_row(cx, "st-copy-on-select", "copy_on_select", i18n::t("settings.term_copy_on_select"), i18n::t("settings.term_copy_on_select_about"), s.copy_on_select, true))
            .child(self.flag_row(cx, "st-osc52", "clipboard_write", i18n::t("settings.term_osc52"), i18n::t("settings.term_osc52_about"), s.clipboard_write, true))
            .child(self.flag_row(cx, "st-scroll-on-input", "scroll_on_input", i18n::t("settings.term_scroll_on_input"), i18n::t("settings.term_scroll_on_input_about"), s.scroll_on_input, true))
            .child(fixed(self, cx, "scroll-speed", "terminal_scroll_speed", i18n::t("settings.terminal_scroll_speed"), s.terminal_scroll_speed, 1.0, (0.1, 5.0, 0.1)));
        if cfg!(target_os = "macos") {
            page = page.child(row(
                i18n::t("settings.term_option"),
                Some(i18n::t("settings.term_option_about")),
                button("st-option-link", i18n::t("settings.term_open_keyboard"), self.act(cx, |this, w, cx| this.go("keyboard", w, cx))),
            ));
        }
        page
    }
}

#[cfg(test)]
mod tests {
    use super::{Object, flip_flag, move_padding, moved, same};
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
}
