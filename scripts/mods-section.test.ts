import { expect, test } from "bun:test";
import { readdir } from "node:fs/promises";
import { tr } from "./tr-stub";
import { loadModule } from "./mod-module";
import { memfs } from "./memfs";
import * as themeImport from "../defaults/frame/settings/theme-import.js";

// Proxy whose methods return itself (so chains keep recording on the same node).
function node(): any {
  const n: any = { id: null, click: null, kids: [], label: null };
  const self: any = new Proxy(n, {
    get(t, k: string) {
      if (k === "__node") return t;
      return (...a: any[]) => {
        if (k === "id") t.id = a[0];
        else if (k === "on_click") t.click = a[0];
        else if (k === "child") t.kids.push(a[0]);
        else if (k === "children") t.kids.push(...a[0]);
        else if (k === "when") { if (a[0]) a[1](self); }
        return self;
      };
    },
  });
  n.self = self;
  return self;
}
const walk = (e: any, f: (n: any) => void) => { const n = e?.__node; if (!n) return; f(n); n.kids.forEach((k: any) => walk(k, f)); };
const ids = (e: any) => { const out: string[] = []; walk(e, (n) => n.id && out.push(n.id)); return out; };
const byId = (e: any, id: string) => { let r: any = null; walk(e, (n) => { if (n.id === id) r = n.self; }); return r; };

const home = "/home";
const calls: any[] = [];
const toasts: any[] = [];
let repoTree: string[] = ["main.js"];
// The gallery asks GitHub what a repo holds; the tests answer from `repoTree`.
(globalThis as any).fetch = async () => ({ status: 200, json: async () => ({ tree: repoTree.map((path) => ({ path })) }) });
function setup(tree: Record<string, string>) {
  const fs = memfs(tree);
  const values: Record<string, any> = {
    tr, fs, div: node, v_flex: node, h_flex: node, svg: node, image: node, appHome: () => home,
    run: (...a: any[]) => calls.push(a), exec: async () => ({ code: 0, stdout: "", stderr: "" }),
    notify: (n: any) => toasts.push(n), window: () => ({ platform: "linux" }), pickFolder: async () => null, home: () => home,
    InputState: { new: () => ({ on() {}, value: () => "", set_value() {} }) }, Input: { new: node },
    ...themeImport,
  };
  return loadModule(new URL("../defaults/frame/settings/fsx.js", import.meta.url), values).then(async (fsx) => {
    Object.assign(values, fsx);
    Object.assign(values, await loadModule(new URL("../defaults/frame/settings/store.js", import.meta.url), values));
    const mods = await loadModule(new URL("../defaults/frame/settings/mods.js", import.meta.url), values);
    const host = { button: (_t: any, id: string, _label: string, onClick: any) => node().id(id).on_click(onClick), segmented: () => node() };
    const section = new mods.ModsSection(host);
    const pending: Promise<unknown>[] = [];
    const cx = { notify() {}, spawn: (f: any) => pending.push(f({ notify() {} })) };
    await section.reload();
    const settle = async () => { while (pending.length) await pending.shift(); };
    return { fs, fsx, mods, section, cx, settle };
  });
}

const palette = new Proxy({}, { get: () => "#000000" });
const tree = () => ({
  [`${home}/mods/hola/main.js`]: "export default class A {}",
  [`${home}/mods/hola/mod.json`]: '{"place":"none","name":"Hola","description":"Prueba","version":"1.0"}',
  [`${home}/mods/tabs/main.js`]: "export default class T {}",
});

test("SHIPPED_MODS equals the folders of defaults/mods", async () => {
  const { mods } = await setup({});
  const real = (await readdir(new URL("../defaults/mods", import.meta.url))).filter((n) => !n.startsWith(".")).sort();
  expect([...mods.SHIPPED_MODS].sort()).toEqual(real);
});

test("turning a personal mod off moves it to mod-store/disabled, and on brings it back", async () => {
  const { fs, section, cx, settle } = await setup(tree());
  let out = section.render(palette, cx);
  const off = ids(byId(out, "st-mod-hola")).find((i) => i.startsWith("st-mod-dis-"))!;
  expect(off).toBeDefined();
  byId(out, off).__node.click({}, cx);
  await settle();
  expect(fs.files.has(`${home}/mod-store/disabled/hola/main.js`)).toBe(true);
  expect(fs.files.has(`${home}/mods/hola/main.js`)).toBe(false);
  out = section.render(palette, cx);
  const on = ids(byId(out, "st-mod-hola")).find((i) => i.startsWith("st-mod-en-"))!;
  byId(out, on).__node.click({}, cx);
  await settle();
  expect(fs.files.has(`${home}/mods/hola/main.js`)).toBe(true);
  expect(fs.files.has(`${home}/mod-store/disabled/hola/main.js`)).toBe(false);
});

test("an included mod can only be duplicated", async () => {
  const { section, cx } = await setup(tree());
  const card = ids(byId(section.render(palette, cx), "st-mod-tabs"));
  expect(card.some((i) => i.startsWith("st-mod-dis-"))).toBe(false);
  expect(card.some((i) => i.startsWith("st-mod-del-"))).toBe(false);
  expect(card.some((i) => i.startsWith("st-mod-dup-"))).toBe(true);
});

test("deleting needs two clicks", async () => {
  const { fs, section, cx, settle } = await setup(tree());
  const del = () => ids(byId(section.render(palette, cx), "st-mod-hola")).find((i) => i.startsWith("st-mod-del-"))!;
  byId(section.render(palette, cx), del()).__node.click({}, cx);
  expect(fs.files.has(`${home}/mods/hola/main.js`)).toBe(true);
  byId(section.render(palette, cx), del()).__node.click({}, cx);
  await settle();
  expect(fs.files.has(`${home}/mods/hola/main.js`)).toBe(false);
});

test("scanCapabilities reads what the code can do", async () => {
  const { fsx } = await setup({
    "/m/a/main.js": 'const r = await exec("git", ["status"]);',
    "/m/b/main.js": 'fetch("https://x")',
    "/m/c/main.js": 'import * as fs from "fs/promises";',
    "/m/d/main.js": "export default class D {}",
  });
  expect(await fsx.scanCapabilities("/m/a")).toEqual({ programs: true, network: false, files: false });
  expect(await fsx.scanCapabilities("/m/b")).toEqual({ programs: false, network: true, files: false });
  expect((await fsx.scanCapabilities("/m/c")).files).toBe(true);
  expect(await fsx.scanCapabilities("/m/d")).toEqual({ programs: false, network: false, files: false });
});

test("clicking a gallery card opens its page, with Install and Back", async () => {
  const { section, cx } = await setup(tree());
  section.tab = "explore";
  section.found["herdr-mod"] = [{ full_name: "a/b", description: "d", stargazers_count: 3, pushed_at: new Date().toISOString(), html_url: "https://github.com/a/b" }];
  expect(ids(section.render(palette, cx))).toContain("st-cat-0");
  byId(section.render(palette, cx), "st-cat-0").__node.click({}, cx);
  const page = ids(section.render(palette, cx));
  expect(page).toContain("st-detail-install");
  expect(page).toContain("st-detail-github");
  expect(page).not.toContain("st-cat-0");
  byId(section.render(palette, cx), "st-detail-back").__node.click({}, cx);
  expect(ids(section.render(palette, cx))).toContain("st-cat-0");
});

test("installing from a gallery page that holds no mod stays on the page and says why", async () => {
  const { section, cx, settle } = await setup(tree());
  section.tab = "explore";
  section.found["herdr-mod"] = [{ full_name: "a/b", description: "d", stargazers_count: 3, pushed_at: new Date().toISOString(), html_url: "https://github.com/a/b" }];
  byId(section.render(palette, cx), "st-cat-0").__node.click({}, cx);
  byId(section.render(palette, cx), "st-detail-install").__node.click({}, cx);
  await settle();
  expect(section.detail).not.toBeNull();
  expect(section.tab).toBe("explore");
  expect(section.installError).toContain("No mods, themes or layouts found");
  // Going back clears it.
  byId(section.render(palette, cx), "st-detail-back").__node.click({}, cx);
  expect(section.installError).toBeNull();
});

test("a gallery repo with nothing installable is flagged and Install does nothing", async () => {
  repoTree = ["README.md", "src/lib.rs"];
  try {
    const { section, cx, settle } = await setup(tree());
    section.tab = "explore";
    section.found["herdr-mod"] = [{ full_name: "a/b", description: "d", stargazers_count: 3, pushed_at: new Date().toISOString(), html_url: "https://github.com/a/b" }];
    byId(section.render(palette, cx), "st-cat-0").__node.click({}, cx);
    section.render(palette, cx);
    await settle();
    expect(section.compat["a/b"]).toBe(false);
    byId(section.render(palette, cx), "st-detail-install").__node.click({}, cx);
    await settle();
    expect(section.busy).toBe(false);
    expect(section.installError).toBeNull();
  } finally {
    repoTree = ["main.js"];
  }
});
