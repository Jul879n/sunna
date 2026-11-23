import { expect, test } from "bun:test";
import { loadModule } from "./mod-module";
import { memfs } from "./memfs";
import * as themeImport from "../defaults/frame/settings/theme-import.js";

const home = "/home";
const execCalls: any[] = [];
let execReply: (cmd: string, args: string[]) => any = () => ({ code: 0, stdout: "", stderr: "" });

async function load(tree: Record<string, string>) {
  const fs = memfs(tree);
  const values: Record<string, any> = {
    fs, appHome: () => home, home: () => home,
    exec: async (cmd: string, args: string[]) => { execCalls.push([cmd, args]); return execReply(cmd, args); },
    ...themeImport,
  };
  const fsx = await loadModule(new URL("../defaults/frame/settings/fsx.js", import.meta.url), values);
  Object.assign(values, fsx);
  const store = await loadModule(new URL("../defaults/frame/settings/store.js", import.meta.url), values);
  return { fs, store };
}

const theme = JSON.stringify({ background: "#000000", foreground: "#ffffff", selection: "#111111", ansi: Array(16).fill("#000000") });

test("parseSource tells GitHub shorthand, git URLs and folders apart", async () => {
  const { store } = await load({});
  expect(store.parseSource("acme/tools@v2")).toMatchObject({ kind: "github", url: "https://github.com/acme/tools.git", ref: "v2", label: "acme/tools" });
  expect(store.parseSource("https://example.com/a/b.git")).toMatchObject({ kind: "git", ref: null });
  expect(store.parseSource("git@github.com:a/b.git").kind).toBe("git");
  expect(store.parseSource("/tmp/mod-src")).toMatchObject({ kind: "folder", path: "/tmp/mod-src" });
  expect(store.parseSource("~/mods/x").kind).toBe("folder");
  expect(() => store.parseSource("just words")).toThrow("settings.mods_bad_source");
});

test("scanSource finds mods and themes; nothing found is an error", async () => {
  const { store } = await load({ "/src/mods/a/main.js": "x", "/src/themes/t.json": theme, "/src/themes/bad.json": '{"background":"#000000"}' });
  const items = await store.scanSource("/src", "repo");
  expect(items.map((i: any) => [i.type, i.name])).toEqual([["mod", "a"], ["theme", "t"]]);
  const empty = await load({ "/empty/readme.md": "hi" });
  await expect(empty.store.scanSource("/empty")).rejects.toThrow("settings.mods_nothing_found");
});

test("an existing destination is flagged and never overwritten; an install records its source", async () => {
  const { fs, store } = await load({ "/src/mods/a/main.js": "new", "/src/mods/b/main.js": "b", [`${home}/mods/a/main.js`]: "mine" });
  const items = await store.withExisting(await store.scanSource("/src"));
  expect(items.map((i: any) => [i.name, i.exists])).toEqual([["a", true], ["b", false]]);
  await expect(store.installItems([items[0]], { source: "acme/x", commit: "abc" })).rejects.toThrow("settings.mods_exists");
  expect(fs.files.get(`${home}/mods/a/main.js`)).toBe("mine");
  await store.installItems([items[1]], { source: "acme/x", ref: null, commit: "abc123" });
  expect(fs.files.get(`${home}/mods/b/main.js`)).toBe("b");
  expect(JSON.parse(fs.files.get(`${home}/mods/b/.herdr-install.json`)!)).toMatchObject({ source: "acme/x", commit: "abc123" });
});

test("the catalog falls back to gh when fetch fails", async () => {
  const { store } = await load({});
  const store2: Record<string, string> = {};
  (globalThis as any).localStorage = { getItem: (k: string) => store2[k] ?? null, setItem: (k: string, v: string) => { store2[k] = v; } };
  (globalThis as any).fetch = async () => { throw new Error("offline"); };
  execReply = (cmd) => cmd === "gh"
    ? { code: 0, stdout: JSON.stringify({ items: [{ full_name: "a/b", description: "d", stargazers_count: 3, pushed_at: "2026-01-01T00:00:00Z", html_url: "https://github.com/a/b", extra: 1 }] }), stderr: "" }
    : { code: 1, stdout: "", stderr: "" };
  const list = await store.catalog("herdr-mod");
  expect(list).toEqual([{ full_name: "a/b", description: "d", stargazers_count: 3, pushed_at: "2026-01-01T00:00:00Z", html_url: "https://github.com/a/b" }]);
  expect(execCalls.some(([c]) => c === "gh")).toBe(true);
  expect(JSON.parse(store2["catalog:herdr-mod"]).items).toHaveLength(1);
});
