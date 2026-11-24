// Agents panel: every agent with filters and counts, and a feed of when they finished or needed you.

import { View, div, svg, image } from "gpui-kit";
import { v_flex, h_flex, Scrollbar, InputState, Input } from "gpui-base";
import { agents, run, theme, shared, setShared, tr, style } from "herdr";
import { STATUSES, filterAgents, counts, apps, unread } from "./model.js";

// Radius scale: the theme's `style.radius` (6 = the original sizes).
const R = (n) => Math.round(n * style().radius / 6);

const when = (ms, now) => {
  const m = Math.floor((now - ms) / 60000);
  return m < 1 ? tr("activity.now") : m < 60 ? `${m}m` : m < 1440 ? `${Math.floor(m / 60)}h` : `${Math.floor(m / 1440)}d`;
};

function icon(p, size, color) {
  if (!p?.icon) return null;
  return p.tint ? svg(p.icon).size(size).flex_none().text_color(color) : image(p.icon).w(size).h(size).flex_none();
}

export default class Activity extends View {
  init(_props, cx) {
    this.tab = "agents"; // "agents" | "feed"
    this.status = "all"; this.app = "all"; this.query = "";
    this.queryInput = InputState.new({ placeholder: tr("activity.search") });
    this.queryInput.on("change", (_e, cx) => { this.query = this.queryInput.value(); cx.notify(); });
    cx.timer.every(30000, (cx) => cx.notify());
  }

  chip(t, id, label, on, pick, count) {
    return div().id(id).h(24).px(8).gap(5).flex().items_center().flex_none().rounded(R(6)).border_1().border_color(on ? t.accent : t.border).bg(on ? t.selected : t.background)
      .text_size(11).text_color(on ? t.foreground : t.muted).cursor_pointer().hover((s) => s.bg(t.hover)).on_click((_e, cx) => { pick(cx); cx.notify(); })
      .child(label).when(count !== undefined, (el) => el.child(div().text_color(t.dim).child(String(count))));
  }

  dot(t, status) {
    const color = t[status] ?? t.dim;
    if (status === "working") return svg("mods/workspace-chrome/spinner.svg").size(12).flex_none().text_color(t.working).spin(900);
    if (status === "idle") return div().size(8).flex_none().rounded_full().border_1().border_color(t.dim);
    return div().size(8).flex_none().rounded_full().bg(color);
  }

  go(address) { run("agent.focus", { terminal: address }); }

  agentsView(t, list) {
    const n = counts(list);
    const shown = filterAgents(list, { status: this.status, app: this.app, query: this.query });
    const names = apps(list);
    const label = (s) => tr(`activity.status_${s}`);
    return v_flex().flex_1().min_h(0)
      .child(v_flex().flex_none().p(10).gap(8).border_b(1).border_color(t.border)
        .child(div().id("act-search").h(30).px(10).flex().items_center().rounded(R(8)).border_1().border_color(t.border)
          .on_mouse_down("left", (_e, cx) => cx.stop_propagation()).child(Input.new(this.queryInput).h_full().text_size(12).text_color(t.foreground)))
        .child(h_flex().gap(6).flex_wrap().children(STATUSES.map((s) => this.chip(t, `act-st-${s}`, label(s), this.status === s, () => { this.status = s; }, n[s]))))
        .when(names.length > 1, (el) => el.child(h_flex().gap(6).flex_wrap().children(["all", ...names].map((a) => this.chip(t, `act-app-${a}`, a === "all" ? tr("activity.status_all") : a, this.app === a, () => { this.app = a; }))))))
      .child(div().flex_1().min_h(0).relative()
        .child(v_flex().id("act-list").absolute().inset_0().overflow_y_scroll().p(8).gap(4)
          .when(list.length === 0, (el) => el.child(div().p(8).text_size(12).text_color(t.dim).child(tr("activity.none"))))
          .when(list.length > 0 && shown.length === 0, (el) => el.child(v_flex().p(8).gap(8).items_start()
            .child(div().text_size(12).text_color(t.dim).child(tr("activity.no_match")))
            .child(this.chip(t, "act-clear", tr("activity.clear_filters"), false, () => { this.status = "all"; this.app = "all"; this.query = ""; this.queryInput.set_value(""); }))))
          .children(shown.map((a) => h_flex().id(`act-a-${a.address}`).w_full().flex_none().h(44).px(10).gap(10).items_center().rounded(R(8)).cursor_pointer().hover((s) => s.bg(t.hover))
            .when(a.focused, (el) => el.bg(t.selected))
            .on_click(() => this.go(a.address))
            .child(icon(a.program, 16, t.foreground) ?? div().size(16).flex_none())
            .child(v_flex().flex_1().min_w(0)
              .child(div().truncate().text_size(13).text_color(t.foreground).child(a.space_name))
              .child(div().truncate().text_size(11).text_color(t.dim).child(a.tab_name || a.title || "")))
            .child(this.dot(t, a.status)))))
        .child(Scrollbar.vertical("act-list").absolute().inset_0()));
  }

  feedView(t, events, now) {
    const kinds = { blocked: ["activity.event_blocked", t.blocked], done: ["activity.event_done", t.done ?? t.accent] };
    return v_flex().flex_1().min_h(0)
      .child(h_flex().flex_none().h(36).px(12).gap(8).items_center().border_b(1).border_color(t.border)
        .child(div().flex_1().text_size(11).text_color(t.dim).child(tr("activity.feed_count", { count: events.length })))
        .child(this.chip(t, "act-read", tr("activity.mark_read"), false, () => setShared("activity.seen", Date.now())))
        .child(this.chip(t, "act-wipe", tr("activity.clear"), false, () => { setShared("activity.events", []); setShared("activity.seen", Date.now()); })))
      .child(div().flex_1().min_h(0).relative()
        .child(v_flex().id("act-feed").absolute().inset_0().overflow_y_scroll().p(8).gap(4)
          .when(events.length === 0, (el) => el.child(div().p(8).text_size(12).text_color(t.dim).child(tr("activity.feed_empty"))))
          .children(events.map((e, i) => {
            const [key, color] = kinds[e.kind] ?? kinds.done;
            const fresh = e.at > (shared("activity.seen") ?? 0);
            return h_flex().id(`act-e-${i}`).w_full().flex_none().h(44).px(10).gap(10).items_center().rounded(R(8)).cursor_pointer().hover((s) => s.bg(t.hover))
              .on_click(() => this.go(e.address))
              .child(div().size(8).flex_none().rounded_full().bg(color))
              .child(v_flex().flex_1().min_w(0)
                .child(div().truncate().text_size(13).text_color(t.foreground).child(e.name))
                .child(div().truncate().text_size(11).text_color(t.dim).child(tr(key))))
              .child(div().flex_none().text_size(11).text_color(fresh ? t.accent : t.dim).child(when(e.at, now)));
          })))
        .child(Scrollbar.vertical("act-feed").absolute().inset_0()));
  }

  render(cx) {
    const t = theme();
    const now = Date.now();
    const list = agents();
    const events = shared("activity.events") ?? [];
    const fresh = unread(events, shared("activity.seen") ?? 0);
    return v_flex().id("activity").size_full().min_h(0).rounded(R(10)).overflow_hidden().bg(t.background).border_1().border_color(t.border).text_color(t.foreground).text_size(13)
      .child(h_flex().h(42).flex_none().px(14).gap(8).items_center().border_b(1).border_color(t.border)
        .child(div().font_semibold().child(tr("tabs.activity"))).child(div().flex_1())
        .child(this.chip(t, "act-tab-agents", tr("activity.tab_agents"), this.tab === "agents", () => { this.tab = "agents"; }, list.length))
        .child(this.chip(t, "act-tab-feed", tr("activity.tab_feed"), this.tab === "feed", () => { this.tab = "feed"; if (fresh) setShared("activity.seen", Date.now()); }, fresh || undefined))
        .child(div().id("panel-close").size(24).flex_none().flex().items_center().justify_center().rounded(R(6)).text_color(t.muted).cursor_pointer().hover((s) => s.bg(t.hover)).tooltip(tr("menu.close")).on_click(() => run("layout.hide", { id: "panel" })).child(svg("mods/pane/close.svg").size(13))))
      .child(this.tab === "agents" ? this.agentsView(t, list) : this.feedView(t, events, now));
  }
}
