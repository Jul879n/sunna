@AGENTS.md

## Orca como referencia

Orca (`stablyai/orca`, licencia MIT, instalada en `/Applications/Orca.app`) es la referencia de producto y UX de Sunna. Antes de diseñar o pulir una función (barra lateral, proyectos, worktrees, bots, plantillas, temas, mods), revisar cómo la resuelve Orca: su código en GitHub (`gh api repos/stablyai/orca/...`, árbol en `src/renderer/src/components/`) o su bundle instalado (`app.asar.unpacked/out/renderer/assets`). Replicar la idea y el flujo, no copiar código literal; adaptar a mods JS y al núcleo pequeño de Sunna. Lo que Orca hace mejor en una función ya existente en Sunna también se pule igual.
