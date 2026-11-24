import { test, expect } from "bun:test";
import { tr } from "./tr-stub";
import { readFile } from "node:fs/promises";

const source = (await readFile(new URL("../defaults/mods/worktree/main.js", import.meta.url), "utf8"))
  .replace(/^import .*;\n/gm, "").replace("export default class Worktree", "class Worktree");

// Execute conditional builders and retain text, without native GPUI layout.
function element() {
  const node = { id: "", children: [] as any[] };
  const el: any = new Proxy(node, { get: (_, method) => {
    if (method === "node") return node;
    return (...args: any[]) => {
      if (method === "id") node.id = args[0];
      if (method === "child") node.children.push(args[0]?.node ?? args[0]);
      if (method === "children") node.children.push(...args[0].map((child: any) => child?.node ?? child));
      if (method === "when" && args[0]) args[1](el);
      return el;
    };
  } });
  return el;
}
function textOf(node: any): string {
  return typeof node === "string" ? node : (node?.children ?? []).map(textOf).join(" ");
}
function rowsOf(node: any): any[] {
  return node?.id?.match(/^wt-\d+$/) ? [node] : (node?.children ?? []).flatMap(rowsOf);
}

function harness(focusedMachine: string | null = null, raw: string | null = null) {
  let spaceList: any[] = [];
  let paneList: any[] = [];
  const reads = { spaces: 0, panes: 0 };
  const writes: unknown[] = [];
  const destinations: (string | null)[] = [];
  const executions: any[] = [];
  const overlays: any[] = [];
  let current: any = { overlay: "overlay" };
  let liveAgents: any[] = [];
  let reply: ((command: string, args: string[], options: any) => any) | null = null;
  const globals = {
    agents: () => liveAgents,
    View: class {}, div: element, svg: element, v_flex: element, h_flex: element,
    tr, Input: { new: element }, Scrollbar: { vertical: element }, theme: () => ({}),
    appHome: () => "/app", readTextFile: async () => raw,
    spaces: () => { reads.spaces++; return structuredClone(spaceList); },
    panes: () => { reads.panes++; return structuredClone(paneList); },
    props: () => current, closeOverlay: () => {}, openOverlay: (overlay: any) => { overlays.push(overlay); },
    run: (command: string, args: any) => {
      writes.push([command, args]);
      if (command === "space.new") destinations.push(args.machine === "" ? null : args.machine ?? focusedMachine);
    },
    exec: async (command: string, args: string[], options: any) => {
      executions.push({ command, args, ...options });
      return (reply && reply(command, args, options)) ?? { code: 0, stdout: "", stderr: "" };
    },
  };
  const { Worktree, root } = new Function(...Object.keys(globals), `${source}; return { Worktree, root };`)(...Object.values(globals));
  const worktree = new Worktree();
  Object.assign(worktree, { opening: true, text: "", at: 0, repo: { name: "repo" }, userHome: "/home/user", root: "/home/user/.herdr/worktrees" });
  return { worktree, root, reads, writes, destinations, executions, overlays, setAgents: (a: any[]) => { liveAgents = a; }, setReply: (f: typeof reply) => { reply = f; }, setProps: (next: any) => { current = { overlay: "overlay", ...next }; }, snapshot: (spaces: unknown[], panes: unknown[]) => { spaceList = spaces; paneList = panes; } };
}

test("open render reads spaces and panes once and refreshes both on the next render", () => {
  const { worktree, reads, snapshot } = harness();
  worktree.list = Array.from({ length: 80 }, (_, i) => ({ path: `/repo/wt-${i}`, branch: `branch-${i}`, at: 0 }));
  const spaces = worktree.list.map((c: any, i: number) => ({ id: `s${i}`, tabs: [{ id: `tab${i}` }] }));
  const panes = worktree.list.map((c: any, i: number) => ({ tab: `tab${i}`, cwd: `${c.path}/src` }));
  snapshot(spaces, panes);
  const first = rowsOf(worktree.render({}).node);
  expect(reads).toEqual({ spaces: 1, panes: 1 });
  expect(first).toHaveLength(80);
  for (let i = 0; i < first.length; i++) {
    expect(textOf(first[i])).toContain(`branch-${i}`);
    expect(textOf(first[i])).toContain(" open ");
  }
  snapshot(spaces.slice(0, -1), panes.map((p: any, i: number) => i === 0 ? { ...p, cwd: "/elsewhere" } : p));
  const second = rowsOf(worktree.render({}).node);
  expect(reads).toEqual({ spaces: 2, panes: 2 });
  expect(textOf(second[0])).not.toContain(" open ");
  expect(textOf(second[79])).not.toContain(" open ");
  expect(textOf(second[1])).toContain(" open ");
});

test("open clicks read fresh snapshots once and select an existing space or create one", () => {
  const { worktree, reads, writes, snapshot } = harness();
  const checkout = { path: "/repo/worktree", branch: "branch", at: 0 };
  worktree.list = [checkout];
  snapshot([], []);
  worktree.render({});
  snapshot([{ id: "existing", tabs: [{ id: "tab" }] }], [{ tab: "tab", cwd: checkout.path }]);
  worktree.open(checkout);
  expect(reads).toEqual({ spaces: 2, panes: 2 });
  expect(writes).toEqual([["space.select", { id: "existing" }]]);
  snapshot([], []);
  worktree.open(checkout);
  expect(reads).toEqual({ spaces: 3, panes: 3 });
  expect(writes[1]).toEqual(["space.new", { folder: checkout.path, machine: "" }]);
  worktree.open(undefined);
  expect(reads).toEqual({ spaces: 3, panes: 3 });
});

for (const on of [undefined, null, "server"]) {
  for (const focused of [null, "server"]) {
    test(`create and open stay on ${on ?? "local"} while ${focused ?? "local"} is focused`, async () => {
      const { worktree, destinations, executions } = harness(focused);
      Object.assign(worktree, {
        on, text: "feature", repo: { name: "repo", main: "/repo", top: "/repo" }, list: [],
      });
      expect(await worktree.create()).toBe("");
      expect(executions).toHaveLength(2);
      expect(executions.every(e => (e.on ?? null) === (on ?? null))).toBe(true);
      expect(executions[1].args).toEqual(["worktree", "add", "/home/user/.herdr/worktrees/repo/feature", "feature"]);
      worktree.open({ path: "/repo/existing" });
      expect(destinations).toEqual([on ?? null, on ?? null]);
    });
  }
}

for (const [branch, folder] of [
  ["음악수행", "음악수행"], ["쪼지학", "쪼지학"],
  ["हिन्दी", "हिन्दी"], ["हिंदी", "हिंदी"],
  ["का", "का"], ["कि", "कि"],
  ["বাংলা", "বাংলা"],
  ["feature/हिन्दी", "feature-हिन्दी"],
  ["feature/음악수행-１２٣", "feature-음악수행-１２٣"],
  ["음악수행".normalize("NFD"), "음악수행"],
  ["feature/ASCII_1.2--fix", "feature-ASCII_1.2--fix"],
]) {
  test(`branch ${branch} uses folder ${folder}`, async () => {
    const { worktree, executions, writes } = harness();
    Object.assign(worktree, { text: branch, repo: { name: "repo", main: "/repo", top: "/repo" } });
    expect(await worktree.create()).toBe("");
    const path = `/home/user/.herdr/worktrees/repo/${folder}`;
    expect(executions[1].args).toEqual(["worktree", "add", path, branch]);
    expect(writes[0][1].folder).toBe(path);
  });
}

test("symbol-only branches get stable distinct non-empty folders", async () => {
  const folders: string[] = [];
  for (const branch of ["🎵", "🎶", "🎵"]) {
    const { worktree, executions } = harness();
    Object.assign(worktree, { text: branch, repo: { name: "repo", main: "/repo", top: "/repo" } });
    expect(await worktree.create()).toBe("");
    const path = executions[1].args[2];
    expect(path).toMatch(/\/repo\/branch-[0-9a-f]{8}$/);
    folders.push(path);
  }
  expect(folders[0]).toBe(folders[2]);
  expect(folders[0]).not.toBe(folders[1]);
});

test("space matching keeps deepest checkout, machine, live-folder and fallback rules", () => {
  const { worktree, reads, writes, snapshot } = harness();
  const paths = ["C:/repo", "C:/repo/nested", "C:/repo/fallback", "C:/repo-other"];
  worktree.list = paths.map(path => ({ path }));
  snapshot([
    { id: "remote", machine: "server", folder: paths[0] },
    { id: "nested", folder: paths[0], tabs: [{ id: "tab" }] },
    { id: "fallback", folder: paths[2] },
    { id: "other", folder: paths[3] },
  ], [
    { tab: "tab" },
    { tab: "tab", cwd: "C:\\repo\\nested\\src" },
    { tab: "tab", cwd: paths[0] },
  ]);
  for (const checkout of worktree.list) worktree.open(checkout);
  expect(writes).toEqual([
    ["space.new", { folder: paths[0], machine: "" }],
    ["space.select", { id: "nested" }],
    ["space.select", { id: "fallback" }],
    ["space.select", { id: "other" }],
  ]);
  worktree.on = "server";
  worktree.open(worktree.list[0]);
  expect(writes[4]).toEqual(["space.select", { id: "remote" }]);
  expect(reads).toEqual({ spaces: 5, panes: 5 });
});

test("configured roots drive both the checkout and its space, expanding the target home", async () => {
  for (const [directory, userHome, on, expected] of [
    [undefined, "/home/user", null, "/home/user/.herdr/worktrees"],
    ["~/Developer/worktrees", "/home/remote", "server", "/home/remote/Developer/worktrees"],
    ["/srv/worktrees/", "/home/remote", "server", "/srv/worktrees"],
    ["C:\\Developer\\Work Trees\\", "C:\\Users\\user", null, "C:/Developer/Work Trees"],
    ["~\\Developer\\worktrees", "C:\\Users\\user", "server", "C:/Users/user/Developer/worktrees"],
    ["~", "/home/user", null, "/home/user"],
    ["/", "/home/user", null, ""],
  ] as const) {
    const { worktree, root, executions, writes } = harness(null, JSON.stringify({ worktrees: { directory } }));
    Object.assign(worktree, { opening: false, on, userHome, text: "feature/new", repo: { name: "repo", main: "/repo", top: "/repo" }, root: await root(userHome) });
    const folder = `${expected}/repo/feature-new`;
    expect(await worktree.create()).toBe("");
    expect(executions[1].args).toEqual(["worktree", "add", folder, "feature/new"]);
    expect(executions[1].on).toBe(on);
    expect(writes).toEqual([["space.new", { folder, machine: on ?? "", tab: undefined, argv: undefined }]]);
    expect(textOf(worktree.render({}).node)).toContain(`${expected.replace(userHome.replace(/\\/g, "/"), "~")}/repo/`);
  }
});

test("missing settings keep the old location, while invalid directories cannot create elsewhere", async () => {
  expect(await harness().root("/home/user")).toBe("/home/user/.herdr/worktrees");
  for (const directory of ["", "   ", 42, {}, "relative/worktrees", "C:relative"]) {
    const { root } = harness(null, JSON.stringify({ worktrees: { directory } }));
    await expect(root("/home/user")).rejects.toThrow("worktrees.directory");
  }
  await expect(harness(null, "{").root("/home/user")).rejects.toThrow();
});

function nodesOf(node: any, pick: (n: any) => boolean): any[] {
  return [...(pick(node) ? [node] : []), ...(node?.children ?? []).flatMap((c: any) => nodesOf(c, pick))];
}

test("each linked checkout has a remove button and the main one has none", () => {
  const { worktree, snapshot } = harness();
  worktree.list = [{ path: "/repo", branch: "main", main: true, at: 0 }, { path: "/repo/wt", branch: "feat", at: 0 }];
  snapshot([], []);
  const tree = worktree.render({}).node;
  const ids = nodesOf(tree, (n) => /^wt-remove-\d+$/.test(n.id ?? "")).map((n) => n.id);
  expect(ids).toEqual(["wt-remove-1"]);
});

test("a row's remove opens the remove dialog for that checkout, with its space when it has one", () => {
  const { worktree, overlays, snapshot } = harness();
  const checkout = { path: "/repo/wt", branch: "feat", at: 0 };
  worktree.list = [checkout];
  snapshot([{ id: "s1", tabs: [{ id: "t1" }] }], [{ tab: "t1", cwd: checkout.path }]);
  worktree.removeRow(checkout);
  expect(overlays).toHaveLength(1);
  expect(overlays[0]).toMatchObject({ mod: "worktree", kind: "dialog", props: { action: "remove", space: "s1", folder: "/repo/wt", name: "feat" } });
  snapshot([], []);
  worktree.removeRow(checkout);
  expect(overlays[1].props.space).toBeUndefined();
});

test("removing a checkout with no space open deletes it without closing any space", async () => {
  const { worktree, writes, executions, setProps } = harness();
  setProps({ action: "remove", folder: "/repo/wt", name: "feat" });
  Object.assign(worktree, { opening: false, repo: { name: "repo", main: "/repo", top: "/repo/wt", linked: true }, dirty: false, unpushed: [] });
  expect(await worktree.remove()).toBe("");
  expect(writes.some(([command]: any) => command === "space.close")).toBe(false);
  expect(executions.at(-1).args).toEqual(["worktree", "remove", "/repo/wt"]);
});

test("removing a checkout that has a space closes that space first", async () => {
  const { worktree, writes, snapshot, setProps } = harness();
  snapshot([{ id: "s1", tabs: [] }], []);
  setProps({ action: "remove", space: "s1" });
  Object.assign(worktree, { opening: false, repo: { name: "repo", main: "/repo", top: "/repo/wt", linked: true }, dirty: false, unpushed: [] });
  expect(await worktree.remove()).toBe("");
  expect(writes[0]).toEqual(["space.close", { id: "s1", confirmed: true }]);
});

test("a repo with only its main checkout says what worktrees are for and offers a new one", () => {
  const { worktree, overlays, snapshot, setProps } = harness();
  worktree.list = [{ path: "/repo", branch: "main", main: true, at: 0 }];
  worktree.folder = "/repo";
  setProps({ space: "s1" });
  snapshot([], []);
  const tree = worktree.render({}).node;
  expect(nodesOf(tree, (n) => n.id === "wt-empty")).toHaveLength(1);
  worktree.text = "xyz";
  expect(nodesOf(worktree.render({}).node, (n) => n.id === "wt-empty")).toHaveLength(0);
  worktree.newFromList();
  expect(overlays[0]).toMatchObject({ mod: "worktree", props: { action: "new", space: "s1", folder: "/repo" } });
  worktree.list = [{ path: "/repo", branch: "main", main: true, at: 0 }, { path: "/repo/wt", branch: "feat", at: 0 }];
  worktree.text = "";
  expect(nodesOf(worktree.render({}).node, (n) => n.id === "wt-empty")).toHaveLength(0);
});

test("cleanup lists each linked checkout with its evidence and preselects merged, clean, idle ones", async () => {
  const { worktree, snapshot, setAgents, setReply, setProps } = harness();
  setProps({ action: "cleanup" });
  snapshot([{ id: "s2", tabs: [{ id: "t2" }] }], [{ tab: "t2", cwd: "/repo/busy" }]);
  setAgents([{ space: "s2", status: "working" }]);
  setReply((_c, args, o) => {
    if (args[0] === "branch") return { code: 0, stdout: "  main\n+ merged-clean\n  merged-dirty\n", stderr: "" };
    if (args.includes("status")) return { code: 0, stdout: o.cwd === "/repo/merged-dirty" ? " M a.ts\n" : "", stderr: "" };
    return null;
  });
  Object.assign(worktree, { opening: false, cleaning: true, repo: { name: "repo", main: "/repo", top: "/repo", linked: false } });
  worktree.list = [
    { path: "/repo", branch: "main", main: true, at: 9 }, { path: "/repo/merged-clean", branch: "merged-clean", at: 5 },
    { path: "/repo/merged-dirty", branch: "merged-dirty", at: 4 }, { path: "/repo/busy", branch: "busy", at: 3 }, { path: "/repo/open", branch: "open", at: 2 },
  ];
  const c = await worktree.inspect(worktree.repo);
  expect(c.map((x: any) => [x.c.branch, x.merged, x.dirty, x.busy])).toEqual([
    ["merged-clean", true, false, false], ["merged-dirty", true, true, false], ["busy", false, false, true], ["open", false, false, false],
  ]);
});

test("cleanup removes only what is picked and safe, never forces, and needs a second click", async () => {
  const { worktree, writes, executions, setProps } = harness();
  setProps({ action: "cleanup" });
  const mk = (branch: string, over: any = {}) => ({ c: { path: `/repo/${branch}`, branch }, dirty: false, busy: false, merged: true, space: null, ...over });
  Object.assign(worktree, {
    opening: false, cleaning: true, repo: { name: "repo", main: "/repo", top: "/repo", linked: false },
    cands: [mk("a", { space: "s1" }), mk("b", { dirty: true }), mk("c", { busy: true }), mk("d")],
    sel: new Set(["/repo/a", "/repo/b", "/repo/c"]),
  });
  worktree.go({ spawn() {}, notify() {} });
  expect(worktree.armed).toBe(true); // the first click only arms it
  expect(executions).toHaveLength(0);
  expect(await worktree.cleanup()).toBe("");
  const removed = executions.filter((e: any) => e.args[0] === "worktree").map((e: any) => e.args);
  expect(removed).toEqual([["worktree", "remove", "/repo/a"]]); // b is dirty, c has an agent, d was not picked
  expect(writes).toEqual([["space.close", { id: "s1", confirmed: true }]]);
  worktree.toggle("/repo/d", { notify() {} });
  expect(worktree.armed).toBe(false);
});
