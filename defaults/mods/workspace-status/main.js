import { View, div, svg } from "gpui-kit";
import { h_flex } from "gpui-base";
import { spaces, agents, theme, run, window, keys, settings, openOverlay, shared, tr, style } from "herdr";

// Radius scale: the theme's `style.radius` (6 = the original sizes).
const R = (n) => Math.round(n * style().radius / 6);

// Settings > Look > Status bar: which items show, and how agents are counted (shared keys, all on by default).
const ITEMS = { machine: true, branch: true, agents: true, awake: true, hint: true };

export default class WorkspaceStatus extends View {
  // The palette key as actually bound, not a hardcoded one; without a
  // binding the hint stays useful ("commands") instead of lying.
  hint(t) {
    const mac = window()?.platform === "macos";
    const mod = { cmd: mac ? "⌘" : "Ctrl+", ctrl: mac ? "⌃" : "Ctrl+", shift: mac ? "⇧" : "Shift+", alt: mac ? "⌥" : "Alt+", option: mac ? "⌥" : "Alt+" };
    const b = keys().find((k) => k.section === "global" && k.action === "app.palette");
    const parts = b && b.keys.endsWith("--") ? [...b.keys.slice(0, -2).split("-"), "-"] : b?.keys.split("-");
    const key = parts && parts.map((p) => mod[p.toLowerCase()] ?? p.toUpperCase()).join("");
    return key ? tr("workspace-status.key_commands", { key }) : tr("workspace-status.commands");
  }

  // The cup: keeps the computer awake (always, while agents work, or never).
  awake(t, list) {
    const mode = settings().keep_awake ?? "off";
    const active = mode === "on" || (mode === "auto" && list.some(a => a.status === "working"));
    return h_flex().id("awake").h(20).px(6).gap(5).rounded(R(5)).items_center()
      .bg(mode === "off" ? t.chrome : t.selected).text_color(mode === "off" ? t.dim : t.foreground)
      .hover(s => s.bg(t.hover)).tooltip(tr("awake.tooltip"))
      .on_click(() => openOverlay({ mod: "awake-menu", kind: "popup", width: 240, height: 112 }))
      .child(svg("mods/workspace-status/coffee.svg").size(13))
      .when(mode !== "off", el => el.child(tr(mode === "on" ? "awake.on" : "awake.auto")).child(div().size(6).rounded(R(3)).bg(active ? t.working : t.dim)));
  }

  render() {
    const t = theme();
    const space = spaces().find(s => s.active);
    const list = agents();
    const needs = list.filter(a => a.status === "blocked").length;
    const show = { ...ITEMS, ...(shared("status.items") ?? {}) };
    const working = shared("status.format") === "working";
    const count = working ? list.filter(a => a.status === "working").length : list.length;
    const label = (icon, text) => h_flex().gap(6).items_center().child(svg(`mods/workspace-chrome/${icon}.svg`).size(13)).child(text);
    return h_flex().size_full().px(14).gap(16).items_center().bg(t.chrome).text_size(11.8).text_color(t.muted)
      .when(show.machine, el => el.child(label(space?.machine ? "server" : "monitor", space?.machine ?? tr("workspace-status.this_computer"))))
      .when(show.branch && Boolean(space?.branch), el => el.child(label("branch", space.branch)))
      .when(show.agents, el => el.child(h_flex().gap(6).items_center().child(div().size(6).rounded(R(3)).bg(t.dim)).child(tr(working ? "workspace-status.n_working" : "workspace-status.n_agents", { n: count }))
        .when(needs > 0, el => el.child(div().text_color(t.blocked).child(tr("workspace-status.n_needs_you", { n: needs }))))))
      .child(div().flex_1())
      .when(show.awake && window()?.platform !== "windows", el => el.child(this.awake(t, list)))
      .when(show.hint, el => el.child(div().id("palette").text_color(t.dim).on_click(() => run("app.palette")).child(this.hint(t))));
  }
}
