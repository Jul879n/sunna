import { expect, test } from "bun:test";
import { tr } from "./tr-stub";
import { readFile } from "node:fs/promises";
import { loadModule } from "./mod-module";
import * as themeImport from "../defaults/frame/settings/theme-import.js";

// What a click in the Settings window writes to settings.json, without a window or a core.
const source = await readFile(new URL("../defaults/frame/settings/main.js", import.meta.url), "utf8");
// settings.json as the mod sees it through readTextFile / writeTextFile.
const disk: { raw: string | null; writes: number; fail?: string; readFail?: string; toasts: any[]; keys?: any[] } = { raw: null, writes: 0, toasts: [] };
// Chainable element stub: records ids, click handlers and children so a test can find a button by id.
const sharedValues: Record<string, unknown> = {};
const ui = { calls: [] as any[], tree: null as any, shared: "main" as string };
function el(): any {
  const node: any = { id: null, click: null, kids: [] };
  const self: any = new Proxy(node, {
    get(n, k: string) {
      if (k === "__node") return n;
      return (...a: any[]) => {
        if (k === "id") n.id = a[0];
        else if (k === "on_click") n.click = a[0];
        else if (k === "child") n.kids.push(a[0]);
        else if (k === "children") n.kids.push(...a[0]);
        else if (k === "when") { if (a[0]) a[1](self); }
        return self;
      };
    },
  });
  return self;
}
const found = (e: any, id: string): any => {
  const n = e?.__node;
  if (!n) return null;
  if (n.id === id) return n;
  for (const k of n.kids) { const r = found(k, id); if (r) return r; }
  return null;
};
const values: Record<string, any> = {
  tr, View: class {},
  div: el, h_flex: el, v_flex: el, svg: el,
  layout: () => ui.tree,
  shared: (key: string) => (key === "layout" ? ui.shared : sharedValues[key] ?? null),
  setShared: (key: string, v: unknown) => { sharedValues[key] = v; },
  run: (...a: any[]) => { ui.calls.push(a); },
  exec: (...a: any[]) => { ui.calls.push(["exec", ...a]); return Promise.resolve(null); },
  window: () => ({ platform: "linux" }),
  updates: () => ({ app: { version: "1.0.0", ready: null, updates: true }, core: { version: "1", managed: null, ready: null, updating: false, error: null }, checked: null, checks: true, checking: false, fork: { repo: "Jul879n/sunna", latest: "1.1.0", url: "https://example.test/r", newer: true, notes: null, error: null } }),
  keys: () => disk.keys,
  appHome: () => "/home",
  notify: (n: any) => { disk.toasts.push(n); },
  readTextFile: async () => {
    if (disk.readFail) throw new Error(disk.readFail);
    return disk.raw;
  },
  writeTextFile: async (_path: string, text: string, expected: string | null) => {
    disk.writes++;
    if (disk.fail) throw new Error(disk.fail);
    if (expected !== disk.raw) throw new Error("changed since read");
    disk.raw = text;
  },
};
// themes.js, as main.js imports it: the same stubs, an in-memory fs.
const files = new Map<string, string>();
const fsStub = { exists: async (p: string) => files.has(p), unlink: async (p: string) => { files.delete(p); }, readdir: async () => [] };
const themeList: any[] = [];
const colors = () => new Proxy({}, { get: () => "#112233" });
values.themes = () => themeList;
values.settings = () => ({ theme: "dark-a", theme_light: "light-a", theme_dark: "dark-a" });
values.layouts = () => [];
values.InputState = { new: (o: any = {}) => ({ on() {}, value: () => o.value ?? "", set_value() {} }) };
values.Input = { new: el };
values.home = () => "/home";
Object.assign(values, themeImport, { fs: fsStub });
Object.assign(values, await loadModule(new URL("../defaults/frame/settings/fsx.js", import.meta.url), values));
Object.assign(values, await loadModule(new URL("../defaults/frame/settings/sidebar.js", import.meta.url), values));
Object.assign(values, await loadModule(new URL("../defaults/frame/settings/themes.js", import.meta.url), values));
const names = [...source.matchAll(/^import \{([^}]*)\}/gm)].flatMap(m => m[1].split(",").map(n => n.trim()));
const args = names.map(name => values[name] ?? (() => undefined));
const Settings = new Function(...names, source.replace(/^import .*;\n/gm, "").replace("export default class Settings", "return class Settings"))(...args);

/** Runs `edit` through the Settings window's change() on `raw`; the toasts it posts. */
async function change(raw: string | null, edit: (q: Record<string, any>) => void, fail?: string, readFail?: string) {
  Object.assign(disk, { raw, writes: 0, fail, readFail, toasts: [] });
  const view = new Settings();
  let done: () => void;
  const finished = new Promise<void>((resolve) => (done = resolve));
  view.change(edit, { spawn: (f: (cx: any) => Promise<void>) => f({ notify: () => done() }) });
  await finished;
  return { view, toasts: disk.toasts };
}

test("a change writes settings.json, keeping what else it holds", async () => {
  const { view, toasts } = await change('{"font_size": 15}', (q) => (q.dock_bounce = true));
  expect(JSON.parse(disk.raw!)).toEqual({ font_size: 15, dock_bounce: true });
  expect(view.person).toEqual({ font_size: 15, dock_bounce: true });
  expect(toasts).toEqual([]);
});

test("a broken settings.json is never written over, and says so", async () => {
  const broken = '{"font_size": 15, "theme": "x",}';
  const { toasts } = await change(broken, (q) => (q.dock_bounce = true));
  expect(disk.raw).toBe(broken);
  expect(disk.writes).toBe(0);
  expect(toasts).toEqual([{ title: "settings.json has errors: fix it first", level: "error" }]);
});

test("settings.json that isn't an object counts as broken", async () => {
  for (const raw of ["null", "[]", "3"]) {
    const { toasts } = await change(raw, (q) => (q.dock_bounce = true));
    expect(disk.raw).toBe(raw);
    expect(disk.writes).toBe(0);
    expect(toasts).toEqual([{ title: "settings.json has errors: fix it first", level: "error" }]);
  }
});

test("a write that keeps failing is reported, not dropped", async () => {
  const { toasts } = await change("{}", (q) => (q.dock_bounce = true), "read-only file system");
  expect(disk.writes).toBe(3);
  expect(disk.raw).toBe("{}");
  expect(toasts).toEqual([{ title: "Not saved to settings.json", body: "read-only file system", level: "error" }]);
});

test("a settings.json that can't be read, or an edit that throws, says why", async () => {
  const unread = await change("{}", (q) => (q.dock_bounce = true), undefined, "permission denied");
  expect(disk.writes).toBe(0);
  expect(unread.toasts).toEqual([{ title: "Cannot read settings.json", body: "permission denied", level: "error" }]);
  const thrown = await change("{}", () => { throw new Error("no such section"); });
  expect(disk.writes).toBe(0);
  expect(thrown.toasts).toEqual([{ title: "Cannot read settings.json", body: "no such section", level: "error" }]);
});

/** Presses `keystroke` while the shortcut `old` of `section` is being changed; the toasts. */
async function press(raw: string, keys: any[], capturing: any, e: { key: string; keystroke: string }) {
  Object.assign(disk, { raw, writes: 0, fail: undefined, readFail: undefined, toasts: [], keys });
  const view = new Settings();
  view.person = {};
  view.capturing = capturing;
  view.capture(e, { spawn: (f: (cx: any) => Promise<void>) => f({ notify() {} }) });
  await new Promise((r) => setTimeout(r, 30));
  return view;
}

test("a shortcut moves to the keys pressed, in the same section", async () => {
  const raw = '{"global":{"alt-1":"space.select:1","alt-2":"space.select:2"}}';
  const list = [{ section: "global", keys: "alt-1", action: "space.select:1" }, { section: "global", keys: "alt-2", action: "space.select:2" }];
  const view = await press(raw, list, { section: "global", action: "space.select:1", old: "alt-1" }, { key: "y", keystroke: "ctrl-shift-y" });
  expect(JSON.parse(disk.raw!)).toEqual({ global: { "alt-2": "space.select:2", "ctrl-shift-y": "space.select:1" } });
  expect(view.capturing).toBeNull();
});

test("keys already bound in the section are refused, a bare modifier or Escape change nothing", async () => {
  const raw = '{"global":{"alt-1":"space.select:1","alt-2":"space.select:2"}}';
  const list = [{ section: "global", keys: "alt-1", action: "space.select:1" }, { section: "global", keys: "alt-2", action: "space.select:2" }];
  const capturing = () => ({ section: "global", action: "space.select:1", old: "alt-1" });
  await press(raw, list, capturing(), { key: "x", keystroke: "alt-2" });
  expect(disk.toasts).toEqual([{ title: "Already used by space.select:2", level: "error" }]);
  expect(disk.writes).toBe(0);
  const bare = await press(raw, list, capturing(), { key: "shift", keystroke: "shift" });
  expect(bare.capturing).not.toBeNull();
  const esc = await press(raw, list, capturing(), { key: "escape", keystroke: "escape" });
  expect(esc.capturing).toBeNull();
  expect(disk.writes).toBe(0);
});

const block = (id: string, extra: any = {}) => ({ id, type: "block", ...extra });
const tree = () => ({
  id: "root", type: "row", children: [
    { id: "sidebar", type: "column", children: [block("brand"), block("spaces")] },
    { id: "work", type: "column", children: [block("tabs"), { id: "work-body", type: "row", children: [block("main")] }, block("status")] },
  ],
});
function design() {
  ui.tree = tree();
  ui.calls = [];
  const out = new Settings().design({});
  return { click: (id: string) => { const n = found(out, id); expect(n).not.toBeNull(); n.click(); }, has: (id: string) => found(out, id) !== null };
}

test("layout arrows move a block before its previous sibling or after its next one", () => {
  const d = design();
  d.click("st-ly-a-sidebar");
  d.click("st-ly-b-status");
  expect(ui.calls).toEqual([["layout.move", { id: "sidebar", after: "work" }], ["layout.move", { id: "status", before: "work-body" }]]);
  expect(d.has("st-ly-b-sidebar")).toBe(false);
  expect(d.has("st-ly-a-work")).toBe(false);
  expect(d.has("st-ly-a-status")).toBe(false);
});

test("layout toggles hide or show a block, except the terminals", () => {
  const d = design();
  d.click("st-ly-t-tabs");
  expect(ui.calls).toEqual([["layout.toggle", { id: "tabs" }]]);
  expect(d.has("st-ly-t-main")).toBe(false);
});

test("a newer fork release shows a button that opens it", () => {
  const view = new Settings();
  const tree = view.updates({ accent: "a", dim: "d", foreground: "f", border: "b" });
  const btn = found(tree, "st-fork-open");
  expect(btn).not.toBeNull();
  btn.click({}, { notify() {} });
  expect(ui.calls.at(-1)).toEqual(["exec", "xdg-open", ["https://example.test/r"]]);
});


const cx = () => ({ notify() {}, spawn: (f: (cx: any) => any) => f({ notify() {} }), stop_propagation() {} });
const palette = new Proxy({}, { get: () => "#000000" });

test("the theme section draws a card per theme and a click applies it", () => {
  themeList.splice(0, themeList.length,
    { name: "dark-a", light: false, colors: colors(), style: {}, shipped: true },
    { name: "dark-b", light: false, colors: colors(), style: { radius: 0 }, shipped: false });
  ui.calls = [];
  const out = new Settings().themePicker(palette, false);
  expect(found(out, "st-t-dark-a")).not.toBeNull();
  expect(found(out, "st-t-dark-b")).not.toBeNull();
  found(out, "st-t-dark-b").click({}, cx());
  expect(ui.calls).toContainEqual(["app.theme", { name: "dark-b" }]);
});

test("a new theme from the current one saves 16 ansi colors and its style; an invalid hex saves nothing", async () => {
  themeList.splice(0, themeList.length, { name: "dark-a", light: false, colors: colors(), style: { radius: 4 }, shipped: true });
  Object.assign(disk, { raw: null, writes: 0, fail: undefined, toasts: [] });
  const view = new Settings();
  view.importing = false;
  view.editing = null;
  view.importPanel = null;
  const tools = () => view.themeTools(palette, cx());
  found(tools(), "st-theme-new").click({}, cx());
  expect(view.editing).not.toBeNull();
  expect(view.editing.name).toBe("dark-a-mio");
  view.editing.draft.background = "nope";
  found(tools(), "st-te-save").click({}, cx());
  await new Promise((r) => setTimeout(r, 10));
  expect(disk.writes).toBe(0);
  view.editing.draft.background = "#102030";
  ui.calls = [];
  found(tools(), "st-te-save").click({}, cx());
  await new Promise((r) => setTimeout(r, 10));
  const saved = JSON.parse(disk.raw!);
  expect(saved.ansi).toHaveLength(16);
  expect(saved.style).toEqual({ radius: 4 });
  expect(saved.background).toBe("#102030");
  expect(ui.calls).toContainEqual(["app.theme", { name: "dark-a-mio" }]);
});

test("adding a mod's panel on a Herdr layout copies it first, then adds the child once the copy is shown", () => {
  const withBody = () => ({ id: "root", type: "row", children: [{ id: "panel-body", type: "stack", active: "files", children: [block("files"), block("changes")] }] });
  ui.tree = withBody();
  ui.shared = "main";
  ui.calls = [];
  const view = new Settings();
  view.sidebar = new (values.SidebarEditor)(view, ["main", "terminals", "win98"]);
  view.sidebar.info = { demo: { title: "Demo" } };
  const out = view.design(palette, cx());
  found(out, "st-sb-add-demo").click({}, cx());
  expect(ui.calls).toEqual([["layout.clone", { name: "main", as: "my-main" }]]);
  ui.shared = "my-main";
  ui.calls = [];
  view.design(palette, cx());
  const set = ui.calls.find((c) => c[0] === "layout.set")!;
  const body = (function find(n: any): any { return n.id === "panel-body" ? n : n.children?.map(find).find(Boolean); })(set[1].layout);
  expect(body.children.map((c: any) => c.id)).toEqual(["files", "changes", "panel-demo"]);
  expect(body.children[2].content).toEqual({ mod: "demo" });
  expect(sharedValues["workspace.panels"]).toContain("panel-demo");
  ui.shared = "main";
});
