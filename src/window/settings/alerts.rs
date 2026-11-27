//! Settings > Alerts: which agent alerts show a toast (settings.json
//! `notifications`), a test, and the sounds (`sounds`).

use super::*;
use controls::{button, row, switch};

/// `notifications` after flipping `field`; absent again once everything is on.
pub fn flipped(q: &mut Object, field: &str) {
    let mut n = json!({ "enabled": true, "done": true, "blocked": true });
    if let Some(Value::Object(stored)) = q.get("notifications") {
        for (k, v) in stored {
            n[k] = v.clone();
        }
    }
    let now = n[field].as_bool().unwrap_or(default_of(field));
    n[field] = json!(!now);
    // Gone again once every field is what absent means.
    if ["enabled", "done", "blocked", "bell", "suppress_when_focused"].iter().all(|k| n[*k].as_bool().unwrap_or(default_of(k)) == default_of(k)) {
        q.remove("notifications");
    } else {
        q.insert("notifications".into(), n);
    }
}

/// What a notifications field is when it isn't written: all on, except the bell.
fn default_of(field: &str) -> bool {
    field != "bell"
}

fn notification(p: &Object, field: &str) -> bool {
    p.get("notifications").and_then(|n| n.get(field)).and_then(Value::as_bool).unwrap_or(default_of(field))
}

impl SettingsView {
    pub(super) fn alerts(&mut self, cx: &mut Context<Self>) -> Div {
        let p = self.person.clone();
        let flag = |this: &Self, cx: &mut Context<Self>, id: &'static str, field: &'static str| {
            switch(id, notification(&p, field), this.act(cx, move |this, w, cx| this.change(w, cx, |q| flipped(q, field))))
        };
        let sounds = p.get("sounds").and_then(Value::as_object).cloned().unwrap_or_default();
        let sounds_on = sounds.get("enabled") != Some(&json!(false));
        let file = |key: &str| match sounds.get(key).and_then(Value::as_str) {
            Some(f) if !f.is_empty() => f.rsplit(['/', '\\']).next().unwrap_or(f).to_owned(),
            _ => i18n::t("settings.herdr_s_sound"),
        };
        let play = |id: &'static str, sound: &'static str| button(id, i18n::t("settings.play"), controls::act(move |_, _| crate::sound::play(sound)));
        let test = self.act(cx, |this, w, cx| {
            this.notify(json!({ "title": i18n::t("settings.notif_test_title"), "body": i18n::t("settings.notif_test_body"), "level": "info" }), w, cx)
        });
        div()
            .w_full()
            .child(controls::heading(i18n::t("settings.notif_title")))
            .child(row(i18n::t("settings.notif_enabled"), Some(i18n::t("settings.notif_enabled_about")), flag(self, cx, "st-notif-enabled", "enabled")))
            .child(row(i18n::t("settings.notif_done"), Some(i18n::t("settings.notif_done_about")), flag(self, cx, "st-notif-done", "done")))
            .child(row(i18n::t("settings.notif_blocked"), Some(i18n::t("settings.notif_blocked_about")), flag(self, cx, "st-notif-blocked", "blocked")))
            .child(row(i18n::t("settings.notif_bell"), Some(i18n::t("settings.notif_bell_about")), flag(self, cx, "st-notif-bell", "bell")))
            .child(row(i18n::t("settings.notif_suppress"), Some(i18n::t("settings.notif_suppress_about")), flag(self, cx, "st-notif-suppress", "suppress_when_focused")))
            .child(row(i18n::t("settings.notif_test"), Some(i18n::t("settings.notif_test_about")), button("st-notif-test", i18n::t("settings.notif_test_send"), test)))
            .child(controls::gap())
            .child(controls::heading(i18n::t("settings.sounds")))
            .child(row(
                i18n::t("settings.play_sounds"),
                Some(i18n::t("settings.when_an_agent_needs_you")),
                switch("st-sounds", sounds_on, self.act(cx, move |this, w, cx| this.change(w, cx, |q| {
                    let mut s = q.get("sounds").and_then(Value::as_object).cloned().unwrap_or_default();
                    s.insert("enabled".into(), json!(!sounds_on));
                    q.insert("sounds".into(), Value::Object(s));
                }))),
            ))
            .child(row(i18n::t("settings.an_agent_needs_you"), Some(file("blocked")), play("st-play-blocked", "blocked")))
            .child(row(i18n::t("settings.an_agent_finished"), Some(file("done")), play("st-play-done", "done")))
            .child(div().pt(settings::z(12.0)).text_size(settings::z(12.5)).line_height(settings::z(18.0)).text_color(tone("muted")).child(i18n::t("settings.want_your_own_sounds_ask")))
    }
}

#[cfg(test)]
mod tests {
    use super::{Object, flipped};
    use serde_json::Value;

    fn obj(text: &str) -> Object {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn turning_one_off_stores_all_three() {
        let mut q = obj(r#"{"zoom": 1}"#);
        flipped(&mut q, "done");
        assert_eq!(Value::Object(q), serde_json::from_str::<Value>(r#"{"zoom":1,"notifications":{"enabled":true,"done":false,"blocked":true}}"#).unwrap());
    }

    #[test]
    fn the_bell_is_off_and_silencing_on_until_flipped() {
        let mut q = obj("{}");
        flipped(&mut q, "bell");
        assert_eq!(q["notifications"]["bell"], true);
        flipped(&mut q, "suppress_when_focused");
        assert_eq!(q["notifications"]["suppress_when_focused"], false);
        flipped(&mut q, "bell");
        flipped(&mut q, "suppress_when_focused");
        assert!(q.is_empty(), "back to the defaults leaves nothing: {q:?}");
        assert!(!super::notification(&q, "bell") && super::notification(&q, "suppress_when_focused"));
    }

    #[test]
    fn turning_it_back_on_removes_the_key() {
        let mut q = obj(r#"{"notifications": {"done": false}}"#);
        flipped(&mut q, "done");
        assert!(q.is_empty());
    }
}
