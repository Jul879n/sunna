# Resumen para Can: Sunna y los avisos de núcleo (ítems 1 a 32)

Hola, Can. Este es un resumen único del detalle que está en `docs/mensajes-para-can.md`.

## Contexto

- Sunna es un fork nativo en Rust de herdr-app. Los mods JS quedan solo para extensiones de terceros o del usuario; la API de mods, `sunna do`, `shared()`, los temas y los layouts de usuario siguen compatibles.
- El dueño del fork decidió que todo lo propio de la app va en Rust y que crecer el núcleo ya no se discute antes: cada cambio se te avisa por ítem, con las tres preguntas de `AGENTS.md`.
- Qué cambió en gran parte (fases 1 a 5): editor de archivos nativo, ventana de Configuración (12 secciones), barras lateral y superior, pestañas y espacios, paneles (archivos, cambios, búsqueda, agentes, actividad), menús contextuales y emergentes, diálogos comunes, bots y limpieza automática, ajustes de terminal de Orca, terminal flotante y ⌘W.
- Pruebas: `just test` en verde (cargo, unas 875 pruebas). Verificado en pantalla en macOS con instancia aislada.
- No verificado: Windows, Linux, 2 o más máquinas conectadas, y el arrastre capturado a mitad de gesto. Tampoco el clic derecho real sobre pestañas y paneles, ni bots o avisos con agentes reales.

## Decisiones que necesito de Can

| Tema | Ítem | Pregunta | Recomendación |
|---|---|---|---|
| Modelo de espacios: fijado y estado | 8, 22 | ¿`pinned` y `status` pasan al documento de layout (viajan con el espacio entre máquinas) o se quedan en `shared()`? | Recomendación: campo opcional `pinned` y `space.pin` en el modelo (ausente = no fijado); hoy está solo en `shared()`. |
| Modelo de espacios: linaje | 27 (a) | ¿El núcleo debe conocer el padre de un worktree (campo en el espacio) o el linaje sigue en `shared()`? | Recomendación: dejarlo en `shared()` hasta que el fijado se decida igual (misma pregunta de modelo). |
| Modelo de espacios: fecha de creación | 29 (c) | La regla «sin cambios» usa la fecha de la carpeta del checkout porque el núcleo no guarda la del espacio. ¿Se agrega? | Recomendación: no agregarla por ahora; la fecha de carpeta basta y es opcional. |
| Dormir y desconectar | 23 | ¿El núcleo debe ofrecer «dormir» un espacio y «desconectar» una máquina (`hc machine` no tiene ese comando)? | Recomendación: solo si herdr-core ya tiene o acepta el comando de desconectar; «dormir» queda fuera hasta que el núcleo pueda representarlo. |
| Arrastre y limpieza entre máquinas | 27 (b), 29 (b) | El arrastre entre máquinas está bloqueado y la limpieza solo mira esta computadora. ¿Se mantiene? | Recomendación: mantener ambos límites hasta verificar con 2 o más máquinas. |
| Bots con agente real | 29 (a) | Ejecutar bots con el agente real y avisar al terminar necesita al núcleo (hoy se abre una pestaña `bot <name> #n` sin notificar). ¿Se hace en el núcleo? | Recomendación: sí, con una notificación única al terminar; hoy no se ha ejecutado un bot real (cuesta tokens). |
| Notificaciones: `app.notify` | 30 (b) | ¿`app.notify` queda como comando del núcleo o solo en mods? | Recomendación: dejarlo en el núcleo; es el medio para que un agente avise a la persona. |
| Notificaciones: cola de avisos | 30 (c) | La cola (máximo 4 visibles) vive en la app. ¿Te parece bien? | Recomendación: aceptar; la cola depende del foco y la ventana. |
| Notificaciones: tecla de cierre | 31 (a) | Mientras hay un aviso visible, Esc no llega a la barra lateral. ¿Otra tecla para cerrar avisos? | Recomendación: mantener Esc, porque solo actúa con el teclado fuera de terminal, diálogo y menú. |
| Notificaciones: ícono del Dock | 3 | ¿Va al núcleo `app.badge {count}` (solo macOS, apagado por defecto)? | Recomendación: sí, detrás del ajuste apagado. |
| Terminal flotante | 30 (a) | Se crea sin etiqueta `herdr.space` para que no se adopte como pestaña. ¿El núcleo debería tener el concepto de terminal «de la ventana»? | Recomendación: sí, un concepto explícito en vez de depender de la ausencia de etiqueta. |
| Teclas: salir del terminal | 31 (b) | Tab dentro de un terminal es del terminal. ¿Una tecla fija para salir a los paneles (hoy `sidebar.focus`, `ctrl-shift-e` / `cmd-shift-e`)? | Recomendación: mantener `sidebar.focus` como única salida. |
| Teclas: Ctrl+Tab y buscador | 9, 2 | ¿Se agrega `tab.recent {direction}` para Ctrl+Tab y `focus: "search"` en `app.settings`? | Recomendación: sí a ambos; son parámetros o comandos opcionales y compatibles. |
| Teclas: readline en Linux y Windows | 7 | ⌃K, ⌃B, ⌃L, ⌃N y ⌃P ya no llegan al terminal (siguen a Orca). ¿Se acepta romper la regla «ctrl+letra suelta es del programa»? | Recomendación: pedir tu criterio; es el cambio de compatibilidad más visible. |
| Límites de GPUI (bandeja, GPU, puntero) | 32 | GPUI no tiene bandeja/NSStatusItem, interruptor de GPU ni `CursorStyle::None`. ¿Se acepta no hacerlo o se plantea a GPUI? | Recomendación: aceptar los límites; ocultar puntero ya funciona solo en macOS. |
| Otros avisos con propuesta | 1, 4, 5, 7 | `minimum_contrast` opcional; errores de carga de mods en la API; `tab.close`; bienvenida montada por el núcleo. ¿Se aprueban? | Recomendación: aprobar 1, 4 y 7 tal como están; el 5 ya quedó resuelto en el ítem 32 (⌘W cierra solo la pestaña). |

## Compatibilidad y riesgos

- Los campos nuevos de `Person` y de `settings.json` son opcionales (`#[serde(default)]`, sin escribirse si son el valor por defecto) y las claves nuevas de `shared()` son opcionales (vacío = comportamiento anterior).
- Mods JS de fábrica retirados (confirm, goto-file, workspace-status, welcome, workspace-chrome, tabs, spaces, files, changes, search, agents, activity, add-project, worktree, machine-add, prompt, picker, folders, bots, menu, new-menu, awake-menu, help, toast, pane, peek, terminals-pane y los frames de Configuración, palette, whatsnew, welcome y feedback): un layout que los nombra con `{"mod": ...}` recibe el bloque nativo mediante `layout::native_alias`, y `openOverlay`/`mod.open` abren el nativo. Si la persona tiene una carpeta de mod con ese nombre, manda la suya.
- `keys.json` ya existente: las teclas por defecto nuevas no llegan a archivos editados (por ejemplo `cmd-alt-a` / `ctrl-alt-a` del terminal flotante). Un `keys.json` con `mod.open:goto-file` sigue funcionando porque `keys::current` lo traduce a `file.goto`.
- Originales de teclas conservados en `src/keys/shipped/` (`v0.1.16-*`, `v0.1.16b-*`, `v0.1.16c-*`, `v0.1.16d-*`, `v0.1.16e-*`); solo se actualizan quienes no editaron sus teclas.
- Cambios de comportamiento: `minimum_contrast` ahora es opcional y los temas claros usan `LIGHT_MINIMUM_CONTRAST`; ⌘W en la última pestaña deja el espacio con un terminal nuevo (por `sunna do` sigue cerrando el espacio y pide `confirmed`); cerrar, salir o cerrar la ventana con cambios sin guardar pregunta (los scripts pasan `confirmed: true`).
- El fijado, el estado, el linaje y los grupos viven en `shared()`: no viajan con el espacio entre máquinas ni los ve otro cliente.
- Pendientes declarados: `scripts/themes-from-ghostty.ts` se quitó (habría que reescribirlo contra Rust); Enter justo tras abrir la bienvenida no avanza hasta un clic en la instancia aislada (sin verificar en primer plano); sin verificar Windows ni Linux.

## Anexo: índice de ítems

1. `minimum_contrast` pasa a opcional.
2. Atajo global para enfocar el buscador de Ajustes.
3. Badge del Dock con agentes que necesitan atención.
4. Estado «error al cargar» de un mod.
5. `tab.close` (⌘W) con una sola pestaña.
6. Atajos de macOS alineados con Orca.
7. Pantalla de bienvenida sin espacios (y teclas de Linux/Windows).
8. Fijar espacios en el modelo.
9. Orden de pestañas recientes en Ctrl+Tab.
10. Ajustes de Terminal, Alertas y Editor.
11. Editor de archivos nativo.
12. «Ir a archivo» en ⌘P y «Guardar y cerrar».
13. Configuración pasa a Rust, por páginas.
14. Máquinas, Actualizaciones, Privacidad y Mods nativos.
15. Apariencia, Terminal, Teclado y Agentes nativos.
16. Se retira el frame mod JS de Configuración.
17. «Ir a archivo» y diálogo de confirmación a Rust.
18. Bloques nativos del layout: estado, bienvenida y marca.
19. `tabs` y `spaces` a Rust; barra lateral sin riel.
20. Paneles `files` y `changes` a Rust.
21. `search`, `agents` y `activity` a Rust (fin de fase 4).
22. Lista lateral: agrupar, ordenar, filtros y fijados.
23. Menús contextuales de espacio, proyecto y máquina.
24. Cursor de teclado en la lista y accesibilidad (fase 5b).
25. Diálogo nativo único; Agregar proyecto, máquina y worktrees.
26. Paleta, Ir a, carpetas, novedades, bienvenida y comentarios.
27. Grupos de proyectos, linaje, arrastre y secciones por máquina.
28. Pulido de accesibilidad de la barra lateral.
29. Bots, limpieza automática y menús emergentes.
30. Toasts, cabecera de panel, peek, comandos rápidos y terminal flotante.
31. Tab global, Esc en avisos, flotante centrada y estado recordado.
32. Ajustes de núcleo de Orca y ⌘W.
