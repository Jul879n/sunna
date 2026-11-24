//! Settings > Git and spaces: worktree branch names, the base the Changes
//! panel compares against, and how it groups files.

use super::*;
use controls::row;

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
    }
}
