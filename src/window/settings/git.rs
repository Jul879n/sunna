//! Settings > Git and spaces: worktree branch names, the base the Changes
//! panel compares against, and how it groups files.

use super::*;
use controls::row;
use crate::settings::z;

impl SettingsView {
    pub(super) fn git(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let mut page = div().w_full().child(controls::heading(i18n::t("settings.orca_git"))).child(row(
            i18n::t("settings.orca_prefix"),
            Some(i18n::t("settings.orca_prefix_about")),
            self.shared_pick(cx, "wt-prefix", "worktree.prefix_mode", &[("none", "settings.orca_prefix_none"), ("git", "settings.orca_prefix_git"), ("custom", "settings.orca_prefix_custom")]),
        ));
        if store::get_str("worktree.prefix_mode") == "custom" {
            let stored = store::get_str("worktree.prefix");
            let field = self.text_field(Field::Prefix, &stored, 240.0, window, cx);
            page = page.child(row(i18n::t("settings.orca_prefix_text"), Some(i18n::t("settings.orca_prefix_text_about")), field));
        }
        let template = store::get_str("worktree.template");
        let template = self.text_field(Field::Template, &template, 240.0, window, cx);
        page.child(row(i18n::t("settings.orca_template"), Some(i18n::t("settings.orca_template_about")), template))
            .child(row(i18n::t("settings.orca_fresh"), Some(i18n::t("settings.orca_fresh_about")), self.shared_switch(cx, "st-wt-fresh", "worktree.fresh")))
            .child(row(i18n::t("settings.orca_branch_names"), Some(i18n::t("settings.orca_branch_names_about")), self.shared_switch(cx, "st-branch-names", "space.branch_names")))
            .child(row(
                i18n::t("settings.orca_base"),
                Some(i18n::t("settings.orca_base_about")),
                self.shared_pick(cx, "changes-base", "changes.base", &[("head", "settings.orca_base_head"), ("default", "settings.orca_base_default"), ("upstream", "settings.orca_base_upstream")]),
            ))
            .child(row(
                i18n::t("settings.orca_groups"),
                Some(i18n::t("settings.orca_groups_about")),
                self.shared_pick(cx, "changes-groups", "changes.groups", &[("flat", "settings.orca_groups_flat"), ("status", "settings.orca_groups_status"), ("new_first", "settings.orca_groups_new_first")]),
            ))
            .child(controls::gap())
            .child(self.cleanup_section(cx))
    }

    /// A number kept in shared(), changed by `step` within `min..=max`.
    fn shared_stepper(&self, cx: &mut Context<Self>, id: &'static str, key: &'static str, unit: &'static str, step: u64, min: u64, max: u64) -> Div {
        let now = store::get(key).as_u64().unwrap_or(min).clamp(min, max);
        let set = move |n: u64| mods::set_shared(key.into(), json!(n.clamp(min, max)));
        controls::stepper(id, i18n::tf(unit, &[("n", &now.to_string())]), self.act(cx, move |_, _, _| set(now.saturating_sub(step))), self.act(cx, move |_, _, _| set(now + step)))
    }

    /// Settings > Git and spaces > Space cleanup: the rules, the schedule, «Clean up now» and what the last runs did.
    fn cleanup_section(&mut self, cx: &mut Context<Self>) -> Div {
        let last = mods::shared("cleanup.last_run").and_then(|v| v.as_u64()).filter(|t| *t > 0);
        let when = last.map_or_else(|| i18n::t("settings.cleanup_never"), |t| timefmt::stamp((t / 1000) as i64));
        let log: Vec<Value> = mods::shared("cleanup.log").and_then(|v| v.as_array().cloned()).unwrap_or_default();
        let lines = log.iter().take(6).enumerate().map(|(i, e)| {
            let field = |k: &str| e.get(k).and_then(Value::as_str).unwrap_or_default().to_owned();
            let name = [field("branch"), field("path")].into_iter().find(|s| !s.is_empty()).unwrap_or_default();
            let outcome = i18n::t(&format!("settings.cleanup_{}", field("outcome")));
            let note = [i18n::t(&field("reason")), field("note")].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ");
            div().id(SharedString::from(format!("st-cl-log-{i}"))).min_h(z(32.0)).py(z(4.0)).flex().items_center().gap(z(10.0)).text_size(z(12.5))
                .child(div().w(z(80.0)).flex_none().text_color(c("foreground")).child(outcome))
                .child(div().flex_1().min_w_0().truncate().text_color(c("foreground")).child(name))
                .child(div().flex_none().text_color(tone("muted")).child(note))
        });
        div().w_full()
            .child(controls::heading(i18n::t("settings.cleanup_heading")))
            .child(controls::note(i18n::t("settings.cleanup_about")))
            .child(row(i18n::t("settings.cleanup_auto"), Some(i18n::t("settings.cleanup_auto_about")), self.shared_switch(cx, "st-cl-auto", "cleanup.auto")))
            .child(row(i18n::t("settings.cleanup_every"), Some(i18n::t("settings.cleanup_every_about")), self.shared_stepper(cx, "cl-every", "cleanup.every_hours", "settings.cleanup_hours", 6, 1, 720)))
            .child(row(i18n::t("settings.cleanup_merged"), Some(i18n::t("settings.cleanup_merged_about")), self.shared_switch(cx, "st-cl-merged", "cleanup.merged")))
            .child(row(i18n::t("settings.cleanup_unchanged"), Some(i18n::t("settings.cleanup_unchanged_about")), self.shared_switch(cx, "st-cl-unchanged", "cleanup.unchanged")))
            .child(row(i18n::t("settings.cleanup_days"), Some(i18n::t("settings.cleanup_days_about")), self.shared_stepper(cx, "cl-days", "cleanup.unchanged_days", "settings.cleanup_days_n", 1, 1, 365)))
            .child(row(i18n::t("settings.cleanup_bots"), Some(i18n::t("settings.cleanup_bots_about")), self.shared_switch(cx, "st-cl-bots", "cleanup.bots")))
            .child(row(
                i18n::t("settings.cleanup_now"),
                Some(i18n::tf("settings.cleanup_last", &[("when", &when)])),
                controls::button("st-cl-now", i18n::t("settings.cleanup_now_go"), self.act(cx, |this, window, cx| this.run("cleanup.run", json!({}), window, cx))),
            ))
            .when(!log.is_empty(), |d| d.child(div().pt(z(8.0)).pb(z(4.0)).text_size(z(12.0)).font_weight(FontWeight::SEMIBOLD).text_color(tone("muted")).child(i18n::t("settings.cleanup_log"))).children(lines))
    }
}
