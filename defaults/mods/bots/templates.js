// Starting points for a bot: its SOUL.md (purpose first, then instructions) and its triggers.
export const CATEGORIES = ["releases", "support", "planning", "personal"];

// `category` groups them in the gallery. Schedules and GitHub triggers start switched off (see `startsOff`):
// the person turns them on once the bot is set up. Names and "what it does first" are locale keys
// `bots.template_<id>` and `bots.first_<id>` (id with `-` as `_`).
export const TEMPLATES = {
  "pr-review": {
    category: "releases",
    soul: "Revisas PRs donde te piden revisión.\n\nLee el diff con `gh pr diff`, comenta problemas concretos con `gh pr review --comment`; nunca apruebes ni hagas merge.\n",
    triggers: [{ type: "github", query: "is:pr is:open review-requested:@me", every_minutes: 10 }],
  },
  "ci-failures": {
    category: "releases",
    soul: "Vigilas los PRs propios con CI en rojo.\n\nPara cada PR abierto con checks fallidos lee el log con `gh run view --log-failed`, explica la causa probable en un comentario con `gh pr comment` y propone el arreglo; no hagas push.\n",
    triggers: [{ type: "github", query: "is:pr is:open author:@me status:failure", every_minutes: 15 }],
  },
  "release-notes": {
    category: "releases",
    soul: "Redactas las notas de la próxima versión.\n\nLee los commits y PRs desde el último tag (`git log` y `gh pr list --state merged`) y escribe `NOTAS.md` con secciones Nuevo y Corregido, una línea por cambio, en lenguaje de usuario.\n",
    triggers: [{ type: "manual" }],
  },
  "issue-triage": {
    category: "support",
    soul: "Clasificas los issues nuevos.\n\nPara cada issue sin etiquetas propone etiquetas y prioridad con `gh issue comment`; no cierres issues.\n",
    triggers: [{ type: "github", query: "is:issue is:open no:label", every_minutes: 10 }],
  },
  "docs-sync": {
    category: "planning",
    soul: "Mantienes la documentación al día.\n\nCada mañana compara los cambios recientes del código con README y docs, y anota en `DOCS-PENDIENTES.md` lo que quedó desactualizado; no edites la documentación.\n",
    triggers: [{ type: "daily", at: "09:00" }],
  },
  "daily-summary": {
    category: "planning",
    soul: "Cada mañana resumes la actividad del repo.\n\nEscribe en `RESUMEN.md` los commits de ayer, los PRs abiertos y los issues nuevos.\n",
    triggers: [{ type: "daily", at: "09:00" }],
  },
  "day-plan": {
    category: "personal",
    soul: "Preparas mi plan del día.\n\nLee mis PRs abiertos, revisiones pendientes e issues asignados con `gh` y escribe `HOY.md` con las 3 prioridades y lo que está bloqueado.\n",
    triggers: [{ type: "daily", at: "08:30" }],
  },
  blank: {
    category: "personal",
    soul: "Describe aquí el propósito del bot.\n",
    triggers: [{ type: "manual" }],
  },
};

/** A template with a schedule or GitHub trigger starts switched off; a manual one is ready. */
export const startsOff = (tpl) => tpl.triggers.some((x) => x.type !== "manual");

/** `[id, template]` pairs matching the search text (name or what it does first) and the category ("all" for every one). */
export function filterTemplates(query, category, label) {
  const q = (query ?? "").trim().toLowerCase();
  return Object.entries(TEMPLATES).filter(([id, tpl]) =>
    (category === "all" || tpl.category === category) && (!q || label(id).toLowerCase().includes(q)));
}

export const NAME = /^[a-z0-9-]+$/;
const MAX_RUNS = 100;

/** The first paragraph of SOUL.md. */
export const purpose = (soul) => (soul ?? "").trim().split(/\n\s*\n/)[0].replace(/\s+/g, " ");

/** Runs started today (local time). */
export function runsToday(runs, now) {
  const day = new Date(now).toDateString();
  // A skipped run (its precheck said no) never started an agent: it does not count against the cap.
  return runs.filter((r) => r.status !== "skipped" && new Date(r.at).toDateString() === day).length;
}

const lastOf = (runs, type) => runs.filter((r) => r.trigger === type).map((r) => Date.parse(r.at)).sort((a, b) => b - a)[0] ?? 0;

/** The triggers of `bot` that fire at `now` (ms), given its history. github fires through its own poll. */
export function dueTriggers(bot, state, now) {
  const runs = state.runs ?? [], out = [];
  for (const trg of bot.triggers ?? []) {
    if (trg.type === "interval" && now - lastOf(runs, "interval") >= trg.minutes * 60000) out.push(trg);
    if (trg.type === "daily") {
      const [h, m] = String(trg.at).split(":").map(Number), at = new Date(now);
      at.setHours(h || 0, m || 0, 0, 0);
      // `days` (0 = Sunday) limits it to some weekdays; `grace_minutes` drops a slot that was missed by longer than that.
      const dayOk = !Array.isArray(trg.days) || trg.days.length === 0 || trg.days.includes(at.getDay());
      const late = trg.grace_minutes > 0 && now - at.getTime() > trg.grace_minutes * 60000;
      if (dayOk && !late && now >= at.getTime() && lastOf(runs, "daily") < at.getTime()) out.push(trg);
    }
    if (trg.type === "github" && now - Date.parse(state.last_poll?.github ?? 0) >= trg.every_minutes * 60000) out.push(trg);
  }
  return out;
}

/** The history with a new run added, capped. */
export const withRun = (state, run) => ({ ...state, runs: [...(state.runs ?? []), run].slice(-MAX_RUNS) });

/** When `bot` fires next (ms), or null for a manual one. github waits its poll interval. */
export function nextRunAt(bot, state, now) {
  const runs = state.runs ?? [], times = [];
  for (const trg of bot.triggers ?? []) {
    if (trg.type === "interval") times.push(Math.max(now, (lastOf(runs, "interval") || now) + trg.minutes * 60000));
    if (trg.type === "github") times.push(Math.max(now, (Date.parse(state.last_poll?.github ?? 0) || now) + trg.every_minutes * 60000));
    if (trg.type === "daily") {
      const [h, m] = String(trg.at).split(":").map(Number), at = new Date(now);
      at.setHours(h || 0, m || 0, 0, 0);
      const ok = (d) => !Array.isArray(trg.days) || trg.days.length === 0 || trg.days.includes(d.getDay());
      if (at.getTime() <= now && lastOf(runs, "daily") >= at.getTime()) at.setDate(at.getDate() + 1);
      for (let i = 0; i < 7 && !ok(at); i++) at.setDate(at.getDate() + 1);
      times.push(Math.max(now, at.getTime()));
    }
  }
  return times.length ? Math.min(...times) : null;
}

/** How far `ms` is from `now`: `["now"|"min"|"hour"|"day", n]`, and whether it is ahead. */
export function relTime(ms, now) {
  const diff = Math.abs(ms - now), min = Math.round(diff / 60000);
  const [unit, n] = min < 1 ? ["now", 0] : min < 60 ? ["min", min] : min < 1440 ? ["hour", Math.round(min / 60)] : ["day", Math.round(min / 1440)];
  return { unit, n, ahead: ms > now };
}

/** The one trigger worth naming on a card: `{ type, at?, minutes? }` or null for a manual bot. */
export function scheduleOf(bot) {
  const trg = (bot.triggers ?? bot.config?.triggers ?? []).find((x) => x.type === "interval" || x.type === "daily" || x.type === "github");
  return trg ? { type: trg.type, at: trg.at, minutes: trg.minutes ?? trg.every_minutes } : null;
}
