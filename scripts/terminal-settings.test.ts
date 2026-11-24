import { expect, test, beforeEach } from "bun:test";
import { tr } from "./tr-stub";
import { readFile } from "node:fs/promises";
import { loadModule } from "./mod-module";

// The Orca-inspired settings: the Settings page writes shared() keys, and the mod that owns each one reads them.
const url = (p: string) => new URL(`../${p}`, import.meta.url);
const read = (p: string) => readFile(url(p), "utf8");

// Chainable element stub: records ids, click handlers, text and children.
function el(): any {
  const node: any = { id: null, click: null, down: null, kids: [] };
  const self: any = new Proxy(node, {
    get(n, k: string | symbol) {
      if (k === "__node") return n;
      if (typeof k === "symbol") return undefined;
      return (...a: any[]) => {
        if (k === "id") n.id = a[0];
        else if (k === "on_click") n.click = a[0];
        else if (k === "on_mouse_down") n.down = a[1];
        else if (k === "child") n.kids.push(a[0]);
        else if (k === "children") n.kids.push(...a[0]);
        else if (k === "when") { if (a[0]) a[1](self); }
        return self;
      };
    },
  });
  return self;
}
const nodeOf = (e: any) => e?.__node;
const find = (e: any, id: string): any => {
  const n = nodeOf(e);
  if (!n) return null;
  if (n.id === id) return n;
  for (const k of n.kids) { const r = find(k, id); if (r) return r; }
  return null;
};
const ids = (e: any): string[] => { const n = nodeOf(e); return n ? [n.id, ...n.kids.flatMap(ids)].filter(Boolean) : []; };
const text = (e: any): string => (typeof e === "string" ? e : nodeOf(e) ? nodeOf(e).kids.map(text).join(" ") : "");

const shared: Record<string, unknown> = {};
const calls: any[] = [];
const sent: any[] = [];
const state: { disk: string | null } = { disk: null };
const palette = new Proxy({}, { get: () => "#000000" });
const cx = () => ({ notify() {}, spawn: (f: (cx: any) => any) => f({ notify() {} }), stop_propagation() {} });
const base: Record<string, any> = {
  tr, View: class {},
  div: el, h_flex: el, v_flex: el, svg: el, image: el,
  shared: (key: string) => shared[key] ?? null,
  setShared: (key: string, v: unknown) => { shared[key] = v; },
  run: (...a: any[]) => { calls.push(a); },
  window: () => ({ platform: "macos", active: true }),
  style: () => ({ radius: 6 }),
  theme: () => palette,
  pickFolder: async () => "/picked",
  InputState: { new: (o: any = {}) => { const h: Record<string, any> = {}; let value = o.value ?? ""; return { on: (n: string, f: any) => { h[n] = f; }, value: () => value, set_value: (v: string) => { value = v; }, h, type: (v: string) => { value = v; h.change?.({}, cx()); } }; } },
  Input: { new: el },
  appHome: () => "/home",
  notify: (n: any) => { sent.push(n); },
  readTextFile: async () => state.disk,
  writeTextFile: async (_p: string, t: string) => { state.disk = t; },
};

beforeEach(() => { for (const k of Object.keys(shared)) delete shared[k]; calls.length = 0; sent.length = 0; });

async function mod(path: string, exports: string[], extra: Record<string, any> = {}) {
  const src = await read(path);
  const imports = [...src.matchAll(/^import \{([^}]*)\}/gm)].flatMap((m) => m[1].split(",").map((n) => n.trim()).filter(Boolean));
  const values = { ...base, spaces: () => [], panes: () => [], layout: () => null, exec: async () => ({ code: 1, stdout: "", stderr: "" }), ...extra };
  const body = src.replace(/^import .*;\n/gm, "").replace(/^export default class (\w+)/m, "class $1").replace(/^export /gm, "");
  return new Function(...imports, `${body}\nreturn { ${exports.join(", ")} };`)(...imports.map((n) => values[n] ?? (() => undefined)));
}

test("files: a file opens in the native editor tab, on its machine", async () => {
  const runs: any[] = [];
  const { Files } = await mod("defaults/mods/files/main.js", ["Files"], { run: (...a: any[]) => runs.push(a), theme: () => palette, focused: () => null });
  Files.prototype.show.call({ root: "/r", on: null }, "src/a.ts");
  Files.prototype.show.call({ root: "/r", on: "vm" }, "b.md");
  Files.prototype.show.call({ root: null, on: null }, "c.md");
  expect(runs).toEqual([["file.open", { path: "/r/src/a.ts", on: "" }], ["file.open", { path: "/r/b.md", on: "vm" }]]);
});
