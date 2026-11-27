//! What's New, native: the release notes of the versions this launch has not shown (`update_notes`), in the
//! dialog every window uses. A highlight is a card (media, headline, detail); the fixes are a short list. Opening
//! it with spotlights marks them as presented, so the corner note does not repeat. Enter, Esc, the ✕ and
//! «Entendido» close it. It is what the `frame-whatsnew` mod drew, with the same notes.

use super::dialog::{self, Act, Dialog};
use gpui_base::{Scrollbar, ScrollbarMode};
use super::native::{tone, z};
use super::*;
use crate::i18n;
use crate::update_notes::{Item, Note};

/// Overlay id: opening it again replaces it.
pub(super) const ID: &str = "whatsnew";
const WIDTH: f32 = 640.0;
const MONTHS: [&str; 12] = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];

/// `2026-10-23` as «23 oct»; text that is not a date stays as it is.
pub(super) fn date_text(date: &str) -> String {
    let mut parts = date.split('-').map(|p| p.parse::<usize>().ok());
    match (parts.next().flatten(), parts.next().flatten(), parts.next().flatten()) {
        (Some(_), Some(month @ 1..=12), Some(day @ 1..=31)) => {
            i18n::tf("whatsnew.month_day", &[("month", &i18n::t(&format!("whatsnew.{}", MONTHS[month - 1]))), ("day", &day.to_string())])
        }
        _ => date.to_owned(),
    }
}

/// The notes split the way the dialog draws them: highlights as cards, fixes as one list.
pub(super) fn split(note: &Note) -> (Vec<&Item>, Vec<&Item>) {
    (note.items.iter().filter(|i| !i.fixed).collect(), note.items.iter().filter(|i| i.fixed).collect())
}

/// A fix's line: its headline and detail in one sentence.
pub(super) fn fix_line(item: &Item) -> String {
    [item.headline.as_str(), item.detail.as_str()].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(" ")
}

pub(super) struct WhatsNewView {
    app: WeakEntity<AppWindow>,
    notes: Vec<Note>,
    focus: dialog::Focus,
    scroll: ScrollHandle,
}

impl WhatsNewView {
    fn done(&self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.app.update(cx, |app, cx| app.close_overlay(ID, window, cx));
    }

    fn media(media: &crate::update_notes::Media, inset: f32) -> AnyElement {
        // Drawn as wide as the card's content, as tall as its shape says.
        let width = WIDTH - 40.0 - 14.0 - inset;
        img(mods::app_dir().join(&media.path)).w_full().h(z((width * media.height as f32 / media.width.max(1) as f32).max(1.0))).flex_none().into_any_element()
    }

    fn note(&self, note: &Note) -> Div {
        let (cards, fixes) = split(note);
        let mut out = div().flex().flex_col().gap(z(14.0)).child(
            div().flex().flex_row().gap(z(10.0)).items_center()
                .child(div().text_size(z(13.0)).font_weight(FontWeight::MEDIUM).text_color(c("foreground")).child(note.version.clone()))
                .child(div().text_size(z(12.0)).text_color(tone("muted")).child(date_text(&note.date))),
        );
        for item in cards {
            out = out.child(
                div().flex().flex_col().w_full().rounded(z(10.0)).border_1().border_color(c("border")).overflow_hidden()
                    .children(item.media.iter().map(|m| Self::media(m, 2.0)))
                    .child(
                        div().flex().flex_col().p(z(14.0)).gap(z(5.0))
                            .when(!item.headline.is_empty(), |d| d.child(div().text_size(z(14.0)).font_weight(FontWeight::SEMIBOLD).text_color(c("foreground")).child(item.headline.clone())))
                            .when(!item.detail.is_empty(), |d| d.child(div().text_size(z(13.0)).line_height(z(20.0)).text_color(tone("muted")).child(item.detail.clone()))),
                    ),
            );
        }
        if !fixes.is_empty() {
            out = out.child(
                div().flex().flex_col().gap(z(5.0))
                    .child(div().mb(z(3.0)).text_size(z(11.0)).font_weight(FontWeight::MEDIUM).text_color(tone("muted")).child(i18n::t("whatsnew.also_fixed")))
                    .children(fixes.into_iter().map(|item| {
                        div().flex().flex_col().gap(z(6.0))
                            .child(div().text_size(z(12.0)).line_height(z(17.0)).text_color(tone("muted")).child(format!("• {}", fix_line(item))))
                            .children(item.media.iter().map(|m| Self::media(m, 0.0)))
                    })),
            );
        }
        out
    }
}

impl Render for WhatsNewView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();
        let done = move |w: &mut Window, cx: &mut App| this.update(cx, |v, cx| v.done(w, cx));
        // As tall as the notes, never more than the window lets the card be.
        let room = (f32::from(window.viewport_size().height) / settings::zoom() * 0.62).clamp(160.0, 560.0);
        let body = div()
            .relative()
            .child(
                div()
                    .id("whatsnew-body")
                    .max_h(z(room))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .child(div().pr(z(14.0)).flex().flex_col().gap(z(22.0)).children(self.notes.iter().map(|n| self.note(n)))),
            )
            .child(div().absolute().inset_0().child(Scrollbar::vertical(&self.scroll).mode(ScrollbarMode::Always)));
        Dialog::new(i18n::t("whatsnew.what_s_new_in_herdr"), &self.focus, done.clone())
            .description(i18n::t("whatsnew.something_off_or_missing_settings"))
            .width(WIDTH)
            .body(body)
            .no_cancel()
            .action(Act::primary(i18n::t("whatsnew.got_it"), done))
            .render("whatsnew")
    }
}

impl AppWindow {
    /// Shows `notes` (the versions of this update). Spotlights shown here count as presented.
    pub(super) fn show_whats_new(&mut self, notes: Vec<Note>, window: &mut Window, cx: &mut Context<Self>) {
        let spotlights: Vec<String> = notes.iter().filter(|n| n.spotlight).map(|n| n.version.clone()).collect();
        crate::update_notes::presented(&spotlights);
        self.close_overlay(ID, window, cx);
        let app = cx.weak_entity();
        // The keyboard starts on the «Entendido» button (the primary action).
        let view = cx.new(|cx| {
            let focus = dialog::Focus::new(cx);
            focus.put(dialog::initial_stop(0, &[dialog::Tone::Primary]), 1, window, cx);
            WhatsNewView { app, notes, focus, scroll: ScrollHandle::new() }
        });
        self.push_dialog_overlay(ID.into(), WIDTH + 48.0, view.into(), window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // `super::*` brings gpui's own `test` macro: the plain one is the std attribute.
    use core::prelude::v1::test;

    fn item(headline: &str, detail: &str, fixed: bool) -> Item {
        Item { headline: headline.into(), detail: detail.into(), fixed, media: Vec::new() }
    }

    #[test]
    fn a_date_reads_as_day_and_month() {
        let oct = i18n::t("whatsnew.oct");
        assert_eq!(date_text("2026-10-23"), i18n::tf("whatsnew.month_day", &[("month", &oct), ("day", "23")]));
        assert_eq!(date_text("soon"), "soon");
        assert_eq!(date_text("2026-13-01"), "2026-13-01");
    }

    #[test]
    fn highlights_are_cards_and_fixes_are_lines() {
        let note = Note { version: "1.0".into(), date: "2026-01-02".into(), spotlight: true, items: vec![item("New", "thing", false), item("Fixed", "a bug", true), item("", "only detail", true)] };
        let (cards, fixes) = split(&note);
        assert_eq!(cards.len(), 1);
        assert_eq!(fixes.iter().map(|i| fix_line(i)).collect::<Vec<_>>(), ["Fixed a bug", "only detail"]);
    }
}
