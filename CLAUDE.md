@AGENTS.md

## Orca como referencia

Orca (`stablyai/orca`, licencia MIT, instalada en `/Applications/Orca.app`) es la referencia de producto y UX de Sunna. Antes de diseñar o pulir una función (barra lateral, proyectos, worktrees, bots, plantillas, temas, mods), revisar cómo la resuelve Orca: su código en GitHub (`gh api repos/stablyai/orca/...`, árbol en `src/renderer/src/components/`) o su bundle instalado (`app.asar.unpacked/out/renderer/assets`). Replicar la idea y el flujo, no copiar código literal; implementarlo nativo en Rust (ver «Arquitectura de Sunna»). Lo que Orca hace mejor en una función ya existente en Sunna también se pule igual.

## Arquitectura de Sunna: app completa en Rust, mods de terceros en JS

Decisión del dueño del fork (reemplaza lo contrario de `AGENTS.md`: «núcleo pequeño, el resto mods» y «Before you grow the app's core»):

- **Todo lo que es parte de la app va en Rust nativo**: ventana de Configuración, barras lateral y superior, pestañas, paneles (archivos, cambios, búsqueda, agentes, actividad), worktrees, bots, menús, diálogos, editor de archivos, selector «Ir a archivo», bienvenida, etc. No se agregan mods JS de fábrica nuevos (`defaults/mods/*`, `defaults/frame/*`) para funciones propias de la app.
- **Los mods JS son solo para lo que no es nativo**: extensiones de terceros o del usuario sobre la API de mods. La API de mods, `sunna do`, `shared()`, los temas y los layouts de usuario siguen funcionando y se mantienen compatibles.
- **Migración por fases**: lo ya hecho en JS se porta a Rust por etapas (Configuración, barras, paneles, worktree/bots/menús). Cada etapa compila, deja `just test` en verde y verifica con capturas; la JS de una función se borra solo cuando su versión Rust tiene paridad y sus tests migrados.
- **Compatibilidad de datos**: campos nuevos opcionales con `#[serde(default)]`; claves de `shared()` y de `Person` existentes se conservan; si cambian los defaults de teclas, el original anterior se guarda en `src/keys/shipped/`.
- **Consultas a Can**: crecer el núcleo ya no se discute antes, pero cada cambio de núcleo se anota como ítem nuevo en `docs/mensajes-para-can.md` (las tres preguntas de `AGENTS.md`) para que el mantenedor lo conozca; el usuario es quien lo envía.
- `defaults/AGENTS.md` y `src/SKILL.md` se mantienen al día con lo que ven los agentes y los autores de mods.
