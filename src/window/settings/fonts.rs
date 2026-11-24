//! The font picker the Terminal and Appearance pages share: the family in use,
//! written in itself; pressing it opens a filterable list under its row.

use super::*;
use controls::row;
use gpui_base::input::{InputEvent, InputState};

/// What the app uses when settings.json names no font (settings.rs).
pub const DEFAULT_FONT: &str = "JetBrains Mono";
pub const DEFAULT_UI_FONT: &str = "Geist";

/// How many matches the list draws.
const SHOWN: usize = 200;

pub struct State {
    /// "font" or "ui_font" while its list is open.
    pub open: Option<&'static str>,
    pub query: String,
    pub input: Entity<InputState>,
    pub focused: bool,
}

/// The families that contain `query` (any case), at most `SHOWN`.
pub fn matching(all: &[String], query: &str) -> Vec<String> {
    let q = query.to_lowercase();
    all.iter().filter(|f| f.to_lowercase().contains(&q)).take(SHOWN).cloned().collect()
}

/// A font control of a row: size stepper next to the picker.
pub struct Size {
    pub shown: String,
    pub minus: Act,
    pub plus: Act,
}

impl State {
    pub fn new(window: &mut Window, cx: &mut Context<SettingsView>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(i18n::t("settings.type_to_search_fonts")));
        input.update(cx, |s, _| s.set_editor_style(controls::input_style()));
        cx.subscribe_in(&input, window, |this, input, ev: &InputEvent, _window, cx| {
            match ev {
                InputEvent::Change => this.fonts.query = input.read(cx).value().to_string(),
                InputEvent::Focus => this.fonts.focused = true,
                InputEvent::Blur => this.fonts.focused = false,
                _ => return,
            }
            cx.notify();
        })
        .detach();
        Self { open: None, query: String::new(), input, focused: false }
    }
}

impl SettingsView {
    pub(super) fn close_fonts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.fonts.open = None;
        self.focus.focus(window, cx);
    }

    /// The row of a font setting and, under it while it is open, the list.
    /// `which` is the settings.json key: "font" or "ui_font".
    pub(super) fn font_row(&mut self, which: &'static str, title: String, about: Option<String>, family: String, def: &'static str, size: Option<Size>, cx: &mut Context<Self>) -> Div {
        let open = self.fonts.open == Some(which);
        let trigger = div()
            .id(SharedString::from(format!("st-f-{which}")))
            .w(settings::z(220.0))
            .h(settings::z(32.0))
            .px(settings::z(12.0))
            .flex()
            .items_center()
            .rounded(settings::z(7.0))
            .border_1()
            .border_color(if open { c("accent") } else { c("border") })
            .bg(c("raised"))
            .cursor_pointer()
            .hover(|s| s.bg(c("hover")))
            .active(|s| s.bg(c("selected")))
            .tab_index(0)
            .focus_visible(controls::ring)
            .on_click(cx.listener(move |this, _, w, cx| {
                if this.fonts.open == Some(which) {
                    this.close_fonts(w, cx);
                } else {
                    this.fonts.open = Some(which);
                    this.fonts.query.clear();
                    this.fonts.input.update(cx, |s, cx| {
                        s.set_value("", w, cx);
                        s.focus(w, cx);
                    });
                }
                cx.notify();
            }))
            .child(div().truncate().font_family(family.clone()).text_size(settings::z(13.5)).text_color(c("foreground")).child(family));
        let mut control = div().flex().flex_row().items_center().gap(settings::z(8.0)).child(trigger);
        if let Some(s) = size {
            control = control.child(controls::stepper(&format!("{which}-size"), s.shown, s.minus, s.plus));
        }
        let mut out = div().w_full().child(row(title, about, control));
        if open {
            out = out.child(self.font_list(which, def, cx));
        }
        out
    }

    fn font_list(&mut self, which: &'static str, def: &'static str, cx: &mut Context<Self>) -> Stateful<Div> {
        let matches = matching(&mods::fonts(), &self.fonts.query);
        let mut choose = |name: Option<String>| {
            self.act(cx, move |this, w, cx| {
                this.change(w, cx, |q| match &name {
                    Some(n) => drop(q.insert(which.into(), json!(n))),
                    None => drop(q.remove(which)),
                });
                this.close_fonts(w, cx);
            })
        };
        let item = |id: String, label: String, family: String, on: Act| {
            div()
                .id(SharedString::from(id))
                .min_h(settings::z(32.0))
                .px(settings::z(10.0))
                .flex()
                .items_center()
                .rounded(settings::z(6.0))
                .cursor_pointer()
                .hover(|s| s.bg(c("hover")))
                .active(|s| s.bg(c("selected")))
                .tab_index(0)
                .focus_visible(controls::ring)
                .on_click(move |_, w, cx| on(w, cx))
                .child(div().font_family(family).text_size(settings::z(13.5)).text_color(c("foreground")).child(label))
        };
        let mut list = div().id(SharedString::from(format!("st-fl-{which}"))).h(settings::z(220.0)).p(settings::z(4.0)).flex().flex_col().overflow_y_scroll();
        if self.fonts.query.is_empty() {
            list = list.child(item(format!("st-fd-{which}"), i18n::tf("settings.default_def", &[("def", def)]), def.to_owned(), choose(None)));
        }
        if matches.is_empty() {
            list = list.child(div().px(settings::z(10.0)).py(settings::z(8.0)).text_size(settings::z(12.5)).text_color(c("muted")).child(i18n::tf("settings.no_results", &[("query", &self.fonts.query)])));
        }
        for (i, f) in matches.into_iter().enumerate() {
            list = list.child(item(format!("st-fi-{which}-{i}"), f.clone(), f.clone(), choose(Some(f))));
        }
        div()
            .id(SharedString::from(format!("st-font-list-{which}")))
            .w_full()
            .mb(settings::z(8.0))
            .flex()
            .flex_col()
            .rounded(settings::z(10.0))
            .border_1()
            .border_color(c("border"))
            .bg(c("background"))
            .child(div().p(settings::z(8.0)).child(controls::field_fill(&self.fonts.input, self.fonts.focused)))
            .child(list)
    }
}

#[cfg(test)]
mod tests {
    use super::{SHOWN, matching};

    #[test]
    fn the_filter_ignores_case_and_the_list_is_capped() {
        let all: Vec<String> = ["Fira Code", "JetBrains Mono", "Geist", "FiraGO"].map(String::from).to_vec();
        assert_eq!(matching(&all, "FIRA"), ["Fira Code", "FiraGO"]);
        assert_eq!(matching(&all, ""), all);
        assert!(matching(&all, "zzz").is_empty());
        let many: Vec<String> = (0..SHOWN * 2).map(|i| format!("f{i}")).collect();
        assert_eq!(matching(&many, "f").len(), SHOWN);
    }
}
