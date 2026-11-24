// The agents panel's logic: filters, counts and the activity feed. No UI here.

export const STATUSES = ["all", "blocked", "working", "done", "idle"];
export const MAX_EVENTS = 100;

/** The agents that pass the filter `{ status, app, query }` ("all" or "" for no limit). */
export function filterAgents(list, f = {}) {
  const q = (f.query ?? "").trim().toLowerCase();
  return list.filter((a) =>
    (!f.status || f.status === "all" || a.status === f.status) &&
    (!f.app || f.app === "all" || (a.program?.name ?? a.app ?? "") === f.app) &&
    (!q || `${a.space_name} ${a.tab_name} ${a.title ?? ""} ${a.program?.name ?? ""}`.toLowerCase().includes(q)));
}

/** How many agents sit in each status, for the filter chips. */
export function counts(list) {
  const out = { all: list.length, blocked: 0, working: 0, done: 0, idle: 0 };
  for (const a of list) if (a.status in out) out[a.status]++;
  return out;
}

/** The distinct agent programs present, by name. */
export const apps = (list) => [...new Set(list.map((a) => a.program?.name ?? a.app).filter(Boolean))].sort();

/** The event a change of status is news for: "blocked" (needs you) or "done" (finished working), else null. */
export function eventFor(before, after) {
  if (!before || before === after) return null;
  if (after === "blocked") return "blocked";
  if (before === "working" && (after === "idle" || after === "done")) return "done";
  return null;
}

/** The feed with `event` added on top, capped. */
export const withEvent = (events, event) => [event, ...(Array.isArray(events) ? events : [])].slice(0, MAX_EVENTS);

/** Events newer than the moment the feed was last looked at. */
export const unread = (events, seenAt) => (Array.isArray(events) ? events : []).filter((e) => e.at > (seenAt ?? 0)).length;
