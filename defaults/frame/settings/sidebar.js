// Settings > Layout > Sidebar: which panels the right sidebar offers, in what order, and
// adding panels that mods bring (a mod whose mod.json has a `panel`).

import { div } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";
import { run, layout, layouts, shared, setShared, appHome, tr } from "herdr";
import { listDirs, readJson } from "./fsx.js";

const DEFAULT_PANELS = ["files", "changes", "search"];
const text = (k) => (typeof k === "string" && tr(k) !== k ? tr(k) : k);

function find(node, id) {
  if (node?.id === id) return node;
  for (const child of node?.children ?? []) {
    const found = find(child, id);
    if (found) return found;
  }
  return null;
}

export class SidebarEditor {
  /** `shipped`: the layouts Herdr ships, which are read-only: they are copied to `my-<name>` first. */
  constructor(host, shipped) {
    this.host = host;
    this.shipped = shipped;
    this.info = null; // mod -> its mod.json `panel`
    this.available = []; // [{ mod, title }]: mods with a panel the layout doesn't hold yet
    this.pending = null; // { from, op }: waits for the copy to be shown
    this.loading = false;
  }

  enabled() {
    const v = shared("workspace.panels");
    return Array.isArray(v) ? v : DEFAULT_PANELS;
  }

  async load() {
    const info = {};
    for (const mod of await listDirs(`${appHome()}/mods`)) {
      const panel = (await readJson(`${appHome()}/mods/${mod}/mod.json`))?.panel;
      if (panel) info[mod] = panel;
    }
    this.info = info;
  }

  rows() {
    const body = find(layout(), "panel-body");
    return (body?.children ?? []).map((c) => {
      const mod = c.content?.mod ?? c.id;
      return { id: c.id, mod, title: text(this.info?.[mod]?.title) ?? c.id };
    });
  }

  candidates() {
    const held = new Set(this.rows().map((r) => r.mod));
    return Object.entries(this.info ?? {}).filter(([mod]) => !held.has(mod)).map(([mod, p]) => ({ mod, title: text(p.title) ?? mod }));
  }

  /** Applies `op` to the tree of a layout the person owns. */
  apply(op) {
    const tree = JSON.parse(JSON.stringify(layout()));
    const body = find(tree, "panel-body");
    if (!body) return;
    const kids = body.children;
    let on = this.enabled();
    if (op.type === "add") {
      const id = `panel-${op.mod}`;
      if (!kids.some((k) => k.id === id)) kids.push({ id, type: "block", content: { mod: op.mod } });
      on = [...on, id];
    } else if (op.type === "remove") {
      body.children = kids.filter((k) => k.id !== op.id);
      on = on.filter((x) => x !== op.id);
      if (body.active === op.id) body.active = body.children[0]?.id;
    } else if (op.type === "move") {
      const i = kids.findIndex((k) => k.id === op.id);
      const j = i + op.dir;
      if (i >= 0 && j >= 0 && j < kids.length) [kids[i], kids[j]] = [kids[j], kids[i]];
    }
    setShared("workspace.panels", on);
    run("layout.set", { layout: tree });
  }

  /** A structural change: on a Herdr layout, first on its personal copy (the way layout.move does). */
  edit(op) {
    const name = String(shared("layout") ?? "");
    if (!this.shipped.includes(name)) return this.apply(op);
    this.pending = { from: name, op };
    if (layouts().includes(`my-${name}`)) run("layout.switch", { name: `my-${name}` });
    else run("layout.clone", { name, as: `my-${name}` });
  }

  toggle(id) {
    const on = this.enabled();
    const next = on.includes(id) ? on.filter((x) => x !== id) : [...on, id];
    setShared("workspace.panels", next);
    const body = find(layout(), "panel-body");
    if (on.includes(id) && (body?.active === id || next.length === 0)) run("layout.hide", { id: "panel" });
  }

  render(t, cx) {
    const h = this.host;
    if (this.info === null && !this.loading && cx?.spawn) {
      this.loading = true;
      cx.spawn(async (cx) => { await this.load(); this.loading = false; cx.notify(); });
    }
    const name = String(shared("layout") ?? "");
    if (this.pending && name === `my-${this.pending.from}`) {
      const { op } = this.pending;
      this.pending = null;
      this.apply(op);
    }
    const out = v_flex().w_full().gap(2).child(h.heading(t, tr("settings.sidebar")));
    const rows = this.rows();
    if (!find(layout(), "panel-body")) return out.child(div().text_size(12.5).text_color(t.dim).child(tr("settings.sidebar_unsupported")));
    const on = this.enabled();
    const icon = (id, label, onClick) => div().id(id).px(6).py(3).rounded(5).cursor_pointer().text_size(12).text_color(t.muted).hover((s) => s.bg(t.hover)).on_click(onClick).child(label);
    return out
      .children(rows.map((r, i) =>
        h_flex().id(`st-sb-${r.id}`).w_full().gap(8).py(4).items_center()
          .child(div().flex_1().min_w(0).truncate().text_size(13).text_color(on.includes(r.id) ? t.foreground : t.dim).child(r.title))
          .child(i > 0 ? icon(`st-sb-up-${r.id}`, "▲", () => this.edit({ type: "move", id: r.id, dir: -1 })) : div().w(24))
          .child(i < rows.length - 1 ? icon(`st-sb-down-${r.id}`, "▼", () => this.edit({ type: "move", id: r.id, dir: 1 })) : div().w(24))
          .child(r.id.startsWith("panel-") ? icon(`st-sb-rm-${r.id}`, tr("settings.sidebar_remove"), () => this.edit({ type: "remove", id: r.id })) : div())
          .child(h.toggle(t, `st-sb-t-${r.id}`, on.includes(r.id), () => this.toggle(r.id)))))
      .children(this.candidates().map((c) =>
        h_flex().id(`st-sb-c-${c.mod}`).w_full().gap(8).py(4).items_center()
          .child(div().flex_1().min_w(0).truncate().text_size(13).text_color(t.muted).child(c.title))
          .child(h.button(t, `st-sb-add-${c.mod}`, tr("settings.sidebar_add"), () => this.edit({ type: "add", mod: c.mod })))));
  }
}
