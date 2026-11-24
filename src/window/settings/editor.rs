//! Settings > File editor: what the `files` mod reads from shared() `editor.*`.

use super::*;
use controls::{row, stepper};

const FONT_SIZE: &str = "editor.font_size";

/// The size after moving `now` by `d`, kept inside 8..32.
pub fn moved(now: f64, d: f64) -> f64 {
    (now + d).clamp(8.0, 32.0)
}

impl SettingsView {
    pub(super) fn editor(&mut self, cx: &mut Context<Self>) -> Div {
        let size = store::get(FONT_SIZE).as_f64().unwrap_or(12.0);
        let mut by = |d: f64| self.act(cx, move |_, _, _| mods::set_shared(FONT_SIZE.into(), json!(moved(store::get(FONT_SIZE).as_f64().unwrap_or(12.0), d))));
        let shown = if size.fract() == 0.0 { format!("{}", size as i64) } else { format!("{size}") };
        div()
            .w_full()
            .child(controls::heading(i18n::t("settings.editor_title")))
            .child(controls::note(i18n::t("settings.editor_about")))
            .child(row(i18n::t("settings.editor_font_size"), Some(i18n::t("settings.editor_font_size_about")), stepper("editor-size", shown, by(-1.0), by(1.0))))
            .child(row(i18n::t("settings.editor_line_numbers"), None, self.shared_switch(cx, "st-editor-lines", "editor.line_numbers")))
            .child(row(i18n::t("settings.editor_folding"), Some(i18n::t("settings.editor_folding_about")), self.shared_switch(cx, "st-editor-folding", "editor.folding")))
            .child(row(i18n::t("settings.editor_wrap"), Some(i18n::t("settings.editor_wrap_about")), self.shared_switch(cx, "st-editor-wrap", "editor.wrap")))
    }
}

#[cfg(test)]
mod tests {
    use super::moved;

    #[test]
    fn the_size_stays_between_8_and_32() {
        assert_eq!(moved(12.0, 1.0), 13.0);
        assert_eq!(moved(8.0, -1.0), 8.0);
        assert_eq!(moved(32.0, 1.0), 32.0);
    }
}
