import { expect, test } from "bun:test";
import { loadModule } from "./mod-module";

const m = await loadModule(new URL("../defaults/mods/activity/model.js", import.meta.url), {});
const agent = (over: any) => ({ address: "t1", space_name: "api", tab_name: "main", title: "", status: "idle", program: { name: "claude" }, ...over });

test("agents filter by status, program and text, and chips count each status", () => {
  const list = [agent({ address: "a", status: "blocked" }), agent({ address: "b", status: "working", program: { name: "codex" } }), agent({ address: "c", space_name: "web" })];
  expect(m.filterAgents(list, { status: "blocked" }).map((a: any) => a.address)).toEqual(["a"]);
  expect(m.filterAgents(list, { app: "codex" }).map((a: any) => a.address)).toEqual(["b"]);
  expect(m.filterAgents(list, { query: "WEB" }).map((a: any) => a.address)).toEqual(["c"]);
  expect(m.filterAgents(list, { status: "all", app: "all", query: "" })).toHaveLength(3);
  expect(m.counts(list)).toEqual({ all: 3, blocked: 1, working: 1, done: 0, idle: 1 });
  expect(m.apps(list)).toEqual(["claude", "codex"]);
});

test("only needing you or stopping work is an event", () => {
  expect(m.eventFor(undefined, "blocked")).toBeNull();
  expect(m.eventFor("working", "blocked")).toBe("blocked");
  expect(m.eventFor("working", "idle")).toBe("done");
  expect(m.eventFor("idle", "working")).toBeNull();
  expect(m.eventFor("blocked", "blocked")).toBeNull();
});

test("the feed keeps the newest 100 and counts what is newer than the last look", () => {
  let feed: any[] = [];
  for (let i = 1; i <= 105; i++) feed = m.withEvent(feed, { kind: "done", address: "a", name: "x", at: i });
  expect(feed).toHaveLength(100);
  expect(feed[0].at).toBe(105);
  expect(m.unread(feed, 100)).toBe(5);
  expect(m.unread(feed, undefined)).toBe(100);
  expect(m.unread(undefined, 0)).toBe(0);
});
