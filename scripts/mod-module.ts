import { readFile } from "node:fs/promises";

/**
 * Evaluates a mod file the way the tests evaluate main.js: its imports become parameters
 * (`values[name]`, or a no-op), and its `export`ed names come back as an object.
 */
export async function loadModule(url: URL | string, values: Record<string, any>): Promise<Record<string, any>> {
  const source = await readFile(url, "utf8");
  const names = [...source.matchAll(/^import \{([^}]*)\}/gm)].flatMap((m) => m[1].split(",").map((n) => n.trim()).filter(Boolean));
  const stars = [...source.matchAll(/^import \* as (\w+)/gm)].map((m) => m[1]);
  const exported = [...source.matchAll(/^export (?:async )?(?:function|class|const) (\w+)/gm)].map((m) => m[1]);
  const body = source.replace(/^import .*;\n/gm, "").replace(/^export /gm, "") + `\nreturn { ${exported.join(", ")} };`;
  return new Function(...names, ...stars, body)(
    ...names.map((n) => values[n] ?? (() => undefined)),
    ...stars.map((n) => values[n]),
  );
}
