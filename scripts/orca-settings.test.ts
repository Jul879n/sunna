import { expect, test, beforeEach } from "bun:test";
import { tr } from "./tr-stub";
import { readFile } from "node:fs/promises";
import { loadModule } from "./mod-module";

// The Orca-inspired settings: the (native) Settings page writes shared() keys, and the mod that owns each one reads them.
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
  notify() {},
  readTextFile: async () => state.disk,
  writeTextFile: async (_p: string, t: string) => { state.disk = t; },
};

beforeEach(() => { for (const k of Object.keys(shared)) delete shared[k]; calls.length = 0; state.disk = null; });

// ---- The mods that read them ---------------------------------------------------------------------
async function mod(path: string, exports: string[], extra: Record<string, any> = {}) {
  const src = await read(path);
  const imports = [...src.matchAll(/^import \{([^}]*)\}/gm)].flatMap((m) => m[1].split(",").map((n) => n.trim()).filter(Boolean));
  const values = { ...base, spaces: () => [], panes: () => [], agents: () => [], machines: () => [], layout: () => null, exec: async () => ({ code: 1, stdout: "", stderr: "" }), ...extra };
  const body = src.replace(/^import .*;\n/gm, "").replace(/^export default class (\w+)/m, "class $1").replace(/^export /gm, "");
  return new Function(...imports, `${body}\nreturn { ${exports.join(", ")} };`)(...imports.map((n) => values[n] ?? (() => undefined)));
}

test("spaces: the card, the branch name and the sidebar buttons follow the settings", async () => {
  const { Spaces } = await mod("defaults/mods/spaces/main.js", ["Spaces"]);
  const space = { id: "s1", name: "api", folder: "/work/api", active: false, status: "idle", tabs: [{ id: "t1", changed_at: Date.now() - 3 * 3600_000 }] };
  const git = { branch: "feat/x", ahead: 2, behind: 0, dirty: true, linked: false, repo: "r", name: "api" };
  const draw = (extra: any = {}) => Spaces.prototype.row.call({ pressed: null, over: null, ...extra }, { w: space, i: 0, g: git, tree: null }, palette);
  const row = draw();
  expect(ids(row)).toContain("space-git-s1");
  expect(text(row)).toContain("3h");
  shared["space.card"] = { git: false };
  const bare = draw();
  expect(ids(bare)).not.toContain("space-git-s1");
  expect(text(bare)).toContain("3h");
  shared["space.card"] = { git: false, ago: false };
  expect(text(draw())).not.toContain("3h");
  expect(text(draw())).toContain("api");
  shared["space.card"] = { git: false };
  shared["space.branch_names"] = true;
  expect(text(draw())).toContain("feat/x");
  shared["space.branch_names"] = false;
  expect(text(draw())).not.toContain("feat/x");
  // top buttons
  const topOf = () => Spaces.prototype.top.call({ botsOpen: true, bots: [], cx: null }, palette);
  expect(ids(topOf())).toEqual(expect.arrayContaining(["side-search", "side-bots"]));
  shared["sidebar.buttons"] = { search: false };
  expect(ids(topOf())).not.toContain("side-search");
  expect(ids(topOf())).toContain("side-bots");
  shared["sidebar.buttons"] = { bots: false };
  expect(ids(topOf())).not.toContain("side-bots");
  expect(ids(topOf())).not.toContain("side-bot-card-new");
});

test("spaces: agents under a space can be hidden", async () => {
  const live = [{ space: "s1", address: "a1", pane: "p", tab: "t1", status: "working", title: "Fix", program: { name: "claude" }, focused: false }];
  const { Spaces } = await mod("defaults/mods/spaces/main.js", ["Spaces"], { agents: () => live });
  const r = { w: { id: "s1", name: "api", folder: "/w/api", tabs: [{ id: "t1" }] }, i: 0, g: null, tree: null };
  const draw = () => Spaces.prototype.withAgents.call({ pressed: null, over: null, row: Spaces.prototype.row }, r, palette);
  expect(ids(draw())).toContain("space-agent-a1");
  shared["space.card"] = { agents: false };
  expect(ids(draw())).not.toContain("space-agent-a1");
});

test("status bar: each item and the count format follow the settings", async () => {
  const live = [{ status: "working" }, { status: "idle" }, { status: "blocked" }];
  const { WorkspaceStatus } = await mod("defaults/mods/workspace-status/main.js", ["WorkspaceStatus"], { spaces: () => [{ active: true, branch: "main" }], agents: () => live, keys: () => [], settings: () => ({}) });
  const draw = () => WorkspaceStatus.prototype.render.call({ awake: () => el().id("awake"), hint: () => "hint" });
  expect(text(draw())).toContain("3 agents");
  expect(ids(draw())).toEqual(expect.arrayContaining(["awake", "palette"]));
  shared["status.format"] = "working";
  expect(text(draw())).toContain("1 working");
  shared["status.items"] = { agents: false, hint: false, awake: false, branch: false, machine: false };
  const bare = draw();
  expect(text(bare)).not.toContain("working");
  expect(text(bare)).not.toContain("main");
  expect(ids(bare)).not.toContain("palette");
  expect(ids(bare)).not.toContain("awake");
});

test("tabs: the wheel walks the tabs by recent use when asked, by position otherwise", async () => {
  const { Tabs } = await mod("defaults/mods/tabs/main.js", ["Tabs"]);
  const make = () => Object.assign(new Tabs(), { recent: ["b", "a", "c"], ids: ["a", "b", "c", "d"], cycle: null });
  const strip = make();
  strip.go("tab.next");
  expect(calls.at(-1)).toEqual(["tab.next"]);
  shared["tabs.order"] = "recent";
  const t = make();
  t.go("tab.next");
  expect(calls.at(-1)).toEqual(["tab.select", { id: "a" }]);
  t.go("tab.next");
  expect(calls.at(-1)).toEqual(["tab.select", { id: "c" }]);
  t.go("tab.next");
  expect(calls.at(-1)).toEqual(["tab.select", { id: "d" }]); // never used: after the used ones
  const n = calls.length;
  t.go("tab.next");
  expect(calls.length).toBe(n); // the end
  t.go("tab.prev");
  expect(calls.at(-1)).toEqual(["tab.select", { id: "c" }]);
  // a new gesture starts from the tab in front
  t.cycle.at -= 5000;
  const before = calls.length;
  t.go("tab.prev");
  expect(calls.length).toBe(before); // nothing newer than the tab in front
});

test("files: ignored files are listed or left out, and a double click can be asked for", async () => {
  const listing = ["d", "src", "f", "a.ts", "d", "node_modules", "f", "debug.log"].join("\0") + "\0";
  const asked: any[] = [];
  const exec = async (cmd: string, args: string[], opts: any) => {
    asked.push([cmd, args, opts]);
    if (cmd === "sh") return { code: 0, stdout: listing, stderr: "" };
    if (cmd === "git" && args[0] === "check-ignore") return { code: 0, stdout: "node_modules\0debug.log\0", stderr: "" };
    return { code: 1, stdout: "", stderr: "" };
  };
  const { Files } = await mod("defaults/mods/files/main.js", ["Files"], { exec, focused: () => null, settings: () => ({}) });
  const load = async () => {
    const self: any = { entries: new Map(), folders: new Set(), current: () => true, limited: false };
    await Files.prototype.loadFolder.call(self, "", { on: null, cwd: "/repo" });
    return self.entries.get("").map((e: any) => e.name);
  };
  expect(await load()).toEqual(["node_modules", "src", "a.ts", "debug.log"]);
  expect(asked.some((a) => a[1][0] === "check-ignore")).toBe(false);
  shared["files.ignored"] = false;
  expect(await load()).toEqual(["src", "a.ts"]);
  const ask = asked.find((a) => a[1][0] === "check-ignore");
  expect(ask[1]).toEqual(["check-ignore", "-z", "--", "src", "a.ts", "node_modules", "debug.log"]);
  expect(ask[2]).toEqual({ cwd: "/repo", on: null });
  // double click: a click only picks
  const self: any = { entries: new Map([["", [{ name: "a.ts", dir: false }]]]), folders: new Set(), marks: {}, root: "/repo", shown: [], picked: null };
  self.show = (p: string) => self.shown.push(p);
  const rowOf = () => Files.prototype.rows.call(self, "", 0, [], palette)[0];
  rowOf().__node.click?.({}, cx());
  expect(self.shown).toEqual(["a.ts"]);
  shared["files.open"] = "double";
  const row = rowOf().__node;
  expect(row.click).toBeNull();
  row.down({ click_count: 1 }, cx());
  expect(self.shown).toEqual(["a.ts"]);
  expect(self.picked).toBe("a.ts");
  row.down({ click_count: 2 }, cx());
  expect(self.shown).toEqual(["a.ts", "a.ts"]);
});

test("changes: a compare base picks the merge-base, and the groups order the files", async () => {
  const log: string[][] = [];
  const answers: Record<string, any> = {
    "symbolic-ref --short refs/remotes/origin/HEAD": { code: 0, stdout: "origin/trunk\n" },
    "rev-parse --verify --quiet origin/trunk": { code: 0, stdout: "x" },
    "rev-parse --verify --quiet @{u}": { code: 0, stdout: "x" },
    "merge-base HEAD origin/trunk": { code: 0, stdout: "abc123\n" },
    "merge-base HEAD @{u}": { code: 0, stdout: "def456\n" },
  };
  const git = async (args: string[]) => { log.push(args); return answers[args.join(" ")] ?? { code: 1, stdout: "", stderr: "" }; };
  const { mergeBase, Changes } = await mod("defaults/mods/changes/main.js", ["mergeBase", "Changes"], { focused: () => null, settings: () => ({}), InputState: base.InputState, Scrollbar: el, v_virtual_list: el });
  expect(await mergeBase(git, "default")).toEqual({ ref: "origin/trunk", commit: "abc123" });
  expect(await mergeBase(git, "upstream")).toEqual({ ref: "upstream", commit: "def456" });
  expect(await mergeBase(async () => ({ code: 1, stdout: "", stderr: "" }), "default")).toBeNull();
  const files = [{ path: "a", status: "M" }, { path: "b", status: "A" }, { path: "c", status: "D" }, { path: "d", status: "M" }];
  const rows = () => Changes.prototype.groupRows.call({ files }).map((r: any) => r.head ? `${r.head}${r.n}` : r.path);
  expect(rows()).toEqual(["a", "b", "c", "d"]);
  shared["changes.groups"] = "status";
  expect(rows()).toEqual(["M2", "a", "d", "A1", "b", "D1", "c"]);
  shared["changes.groups"] = "new_first";
  expect(rows()).toEqual(["A1", "b", "M2", "a", "d", "D1", "c"]);
});

test("worktree: prefix, template and an existing branch decide the branch name", async () => {
  const known = new Set<string>();
  const exec = async (_c: string, args: string[]) => {
    const line = args.join(" ");
    if (line.startsWith("show-ref --verify --quiet refs/heads/")) return { code: known.has(args[3].slice(11)) ? 0 : 1, stdout: "", stderr: "" };
    if (line === "config user.name") return { code: 0, stdout: "Ana María Pérez\n", stderr: "" };
    return { code: 1, stdout: "", stderr: "" };
  };
  const { branchName, Worktree } = await mod("defaults/mods/worktree/main.js", ["branchName", "Worktree"], { exec });
  const repo = { main: "/r", top: "/r", name: "api" };
  expect(await branchName("login", repo, null)).toBe("login");
  shared["worktree.prefix_mode"] = "custom";
  shared["worktree.prefix"] = "feat";
  expect(await branchName("login", repo, null)).toBe("feat/login");
  expect(await branchName("feat/login", repo, null)).toBe("feat/login");
  shared["worktree.prefix"] = "wip-";
  expect(await branchName("x", repo, null)).toBe("wip-x");
  shared["worktree.prefix_mode"] = "git";
  expect(await branchName("login", repo, null)).toBe("ana-maría-pérez/login");
  shared["worktree.prefix_mode"] = "custom";
  shared["worktree.prefix"] = "u";
  shared["worktree.template"] = "{repo}-{name}";
  expect(await branchName("login", repo, null)).toBe("u/api-login");
  shared["worktree.template"] = "{date}";
  expect(await branchName("login", repo, null)).toMatch(/^u\/\d{4}-\d\d-\d\d-login$/);
  known.add("login");
  expect(await branchName("login", repo, null)).toBe("login"); // exists: as typed
  // external worktrees
  const list = [{ path: "/h/.herdr/worktrees/api/a", main: false }, { path: "/elsewhere/b", main: false }, { path: "/r", main: true }];
  const view = Object.assign(Object.create(Worktree.prototype), { opening: true, root: "/h/.herdr/worktrees" });
  expect(view.outside(list)).toHaveLength(3);
  shared["worktree.external"] = false;
  expect(view.outside(list).map((c: any) => c.path)).toEqual(["/h/.herdr/worktrees/api/a", "/r"]);
});

test("worktree: a new worktree lands in the folder per project or straight in the folder, and can start from origin", async () => {
  const runs: string[][] = [];
  const exec = async (_c: string, args: string[]) => {
    runs.push(args);
    const line = args.join(" ");
    if (line === "symbolic-ref --short refs/remotes/origin/HEAD") return { code: 0, stdout: "origin/main\n", stderr: "" };
    return { code: line.startsWith("show-ref") ? 1 : 0, stdout: "", stderr: "" };
  };
  const { Worktree } = await mod("defaults/mods/worktree/main.js", ["Worktree"], { exec, props: () => ({}) });
  const make = () => Object.assign(Object.create(Worktree.prototype), { text: "Fix Bug", root: "/wt", repo: { main: "/r", top: "/r", name: "api" }, on: null, props: {} });
  const add = () => runs.find((a) => a[0] === "worktree");
  await make().create();
  expect(add()).toEqual(["worktree", "add", "-b", "Fix Bug", "/wt/api/Fix-Bug", "HEAD"]);
  runs.length = 0;
  shared["worktree.nest"] = false;
  shared["worktree.fresh"] = true;
  await make().create();
  expect(add()).toEqual(["worktree", "add", "-b", "Fix Bug", "/wt/Fix-Bug", "origin/main"]);
  expect(runs.some((a) => a.join(" ") === "fetch origin main")).toBe(true);
  expect(runs.some((a) => a.join(" ") === "fetch origin main:main")).toBe(true);
});

test("menu: the apps from Settings add an Open in item to local projects and spaces only", async () => {
  const opened: any[] = [];
  let current: any = {};
  const { items, Menu } = await mod("defaults/mods/menu/main.js", ["items", "Menu"], { terminals: () => [], exec: async (...a: any[]) => { opened.push(a); }, closeOverlay() {}, openOverlay() {}, props: () => current, keys: () => [] });
  const project = (machine: string | null) => ({ tab: "project", overlay: "o", target: { id: "s", key: "k", folder: "/work/api", machine, repo: false } });
  expect(items(project(null), null).map((i: any) => i[1])).not.toContain("open_in");
  shared["menu.open_in"] = ["Cursor", "Zed"];
  expect(items(project(null), null).filter((i: any) => i[1] === "open_in").map((i: any) => i[0])).toEqual(["Open in Cursor", "Open in Zed"]);
  expect(items(project("box"), null).map((i: any) => i[1])).not.toContain("open_in");
  current = project(null);
  Menu.prototype.choose.call({ git: null }, items(project(null), null).findIndex((i: any) => i[1] === "open_in" && i[2].app === "Zed"), cx());
  expect(opened.at(-1)).toEqual(["open", ["-a", "Zed", "/work/api"]]);
});
