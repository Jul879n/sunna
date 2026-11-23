// An in-memory `fs/promises`, for the mods that copy, move and read folders.
export function memfs(initial: Record<string, string> = {}) {
  const files = new Map<string, string>(Object.entries(initial));
  const dirs = new Set<string>();
  const parent = (p: string) => p.slice(0, p.lastIndexOf("/"));
  const addDirs = (p: string) => { for (let d = parent(p); d && !dirs.has(d); d = parent(d)) dirs.add(d); };
  for (const p of files.keys()) addDirs(p);
  const enoent = (p: string) => Object.assign(new Error(`ENOENT: ${p}`), { code: "ENOENT" });
  const children = (dir: string) => {
    const out = new Map<string, boolean>();
    const prefix = dir + "/";
    for (const p of files.keys()) if (p.startsWith(prefix)) { const rest = p.slice(prefix.length); const i = rest.indexOf("/"); out.set(i < 0 ? rest : rest.slice(0, i), i >= 0); }
    for (const d of dirs) if (d.startsWith(prefix)) { const rest = d.slice(prefix.length); const i = rest.indexOf("/"); out.set(i < 0 ? rest : rest.slice(0, i), true); }
    return out;
  };
  const api = {
    files, dirs,
    exists: async (p: string) => files.has(p) || dirs.has(p),
    mkdir: async (p: string) => { dirs.add(p); addDirs(p); },
    readdir: async (p: string, o?: { withFileTypes: boolean }) => {
      if (!dirs.has(p)) throw enoent(p);
      const list = [...children(p)].sort(([a], [b]) => a.localeCompare(b));
      return o?.withFileTypes ? list.map(([name, dir]) => ({ name, isDirectory: () => dir })) : list.map(([n]) => n);
    },
    readFile: async (p: string, enc?: string) => { if (!files.has(p)) throw enoent(p); const s = files.get(p)!; return enc ? s : new TextEncoder().encode(s); },
    writeFile: async (p: string, c: string | Uint8Array) => { files.set(p, typeof c === "string" ? c : new TextDecoder().decode(c)); addDirs(p); },
    unlink: async (p: string) => { if (!files.delete(p)) throw enoent(p); },
    rmdir: async (p: string) => { if (children(p).size) throw new Error(`not empty: ${p}`); dirs.delete(p); },
  };
  return api;
}
