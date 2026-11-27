//! Settings > Privacy: whether anonymous usage data is shared (settings.json
//! `telemetry`) and exactly what would leave this computer. Sending
//! feedback is the sidebar's button (`app.feedback`).

use super::*;
use controls::{button, row, switch};

/// A report as it goes out, one line per line of JSON.
pub fn report_lines(report: &Value) -> Vec<String> {
    serde_json::to_string_pretty(report).unwrap_or_default().lines().map(str::to_owned).collect()
}

/// `telemetry` after switching: absent means on.
pub fn set_telemetry(q: &mut Object, on: bool) {
    if on {
        q.remove("telemetry");
    } else {
        q.insert("telemetry".into(), json!(false));
    }
}

fn report(id: String, title: String, about: Option<String>, data: &Value) -> Div {
    let z = settings::z;
    let lines = report_lines(data);
    div()
        .w_full()
        .pt(z(14.0))
        .flex()
        .flex_col()
        .gap(z(4.0))
        .child(div().text_size(z(13.5)).text_color(c("foreground")).child(title))
        .when_some(about, |d, a| d.child(div().text_size(z(12.5)).text_color(tone("dim")).child(a)))
        .child(
            div()
                .id(SharedString::from(id))
                .mt(z(4.0))
                .w_full()
                .p(z(10.0))
                .flex()
                .flex_col()
                .rounded(z(8.0))
                .border_1()
                .border_color(c("border"))
                .bg(super::super::dialog::field_bg())
                .overflow_x_scroll()
                .children(lines.into_iter().map(|l| {
                    div().font_family("JetBrains Mono").text_size(z(12.0)).line_height(z(17.0)).text_color(tone("muted")).whitespace_nowrap().child(if l.is_empty() { " ".to_owned() } else { l })
                })),
        )
}

impl SettingsView {
    pub(super) fn privacy_page(&mut self, cx: &mut Context<Self>) -> Div {
        let s = crate::telemetry::status();
        let z = settings::z;
        let on = if s.is_null() { self.person.get("telemetry") != Some(&json!(false)) } else { s["setting"] != json!(false) };
        let why = s["off_because"].as_str().map(|r| i18n::tf("settings.off_for_now_reason_is", &[("reason", r)]));
        let sends_nothing = !s.is_null() && s["sends"] == json!(false);
        let mut page = div()
            .w_full()
            .child(controls::heading(i18n::t("settings.usage_data")))
            .child(row(
                i18n::t("settings.share_anonymous_usage_data"),
                Some(why.unwrap_or_else(|| i18n::t("settings.once_a_day_which_agents"))),
                switch("st-telemetry", on, self.act(cx, move |this, w, cx| this.change(w, cx, |q| set_telemetry(q, !on)))),
            ))
            .child(row(
                i18n::t("settings.what_we_send"),
                Some(if sends_nothing { i18n::t("settings.this_build_never_sends_it_s") } else { i18n::t("settings.exactly_what_leaves_your_computer") }),
                button("st-telemetry-show", if self.show_data { i18n::t("settings.hide") } else { i18n::t("settings.show") }, self.act(cx, |this, _, _| this.show_data = !this.show_data)),
            ));
        if !self.show_data {
            return page;
        }
        if s["enabled"] != json!(true) {
            return page.child(div().pt(z(14.0)).text_size(z(13.0)).text_color(tone("dim")).child(i18n::t("settings.nothing_is_collected_or_sent")));
        }
        for (i, r) in s["queued"].as_array().into_iter().flatten().enumerate() {
            let day = r["day"].as_str().unwrap_or_default();
            page = page.child(report(format!("st-tq-{i}"), i18n::tf("settings.waiting_to_send_day", &[("day", day)]), Some(i18n::t("settings.sent_as_is_soon")), r));
        }
        if !s["draft"].is_null() {
            page = page.child(report("st-td".into(), i18n::t("settings.today_so_far"), Some(i18n::t("settings.a_draft_it_changes_until")), &s["draft"]));
        }
        let last = &s["last_sent"];
        if !last.is_null() {
            let day = last["report"]["day"].as_str().unwrap_or_default();
            let at = last["at"].as_i64().map(timefmt::stamp).unwrap_or_default();
            page = page.child(report("st-tl".into(), i18n::tf("settings.last_sent_day", &[("day", day)]), Some(i18n::tf("settings.sent_time", &[("time", &at)])), &last["report"]));
        }
        page
    }
}

#[cfg(test)]
mod tests {
    use super::{Object, report_lines, set_telemetry};
    use serde_json::json;

    #[test]
    fn turning_it_off_stores_false_and_on_removes_the_key() {
        let mut q = Object::new();
        q.insert("zoom".into(), json!(1));
        set_telemetry(&mut q, false);
        assert_eq!(q["telemetry"], json!(false));
        set_telemetry(&mut q, true);
        assert!(q.get("telemetry").is_none() && q.contains_key("zoom"));
    }

    #[test]
    fn a_report_shows_one_line_per_json_line() {
        let lines = report_lines(&json!({ "day": "2026-11-24", "n": [1] }));
        assert_eq!(lines.first().map(String::as_str), Some("{"));
        assert!(lines.iter().any(|l| l.contains("\"day\": \"2026-11-24\"")));
        assert_eq!(lines.last().map(String::as_str), Some("}"));
    }
}
