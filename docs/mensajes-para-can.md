# Mensajes para Can

Texto listo para enviar. Cada punto es una decisión del núcleo (ver «Before you grow the app's core» en `AGENTS.md`).

## 1. Aviso: cambio en `minimum_contrast` (`src/settings.rs`)

`Person.minimum_contrast` pasa de `f32` (default 1.0) a `Option<f32>`.

- Sin valor: 1.0 en temas oscuros y `LIGHT_MINIMUM_CONTRAST` en claros, donde el texto blanco desaparecía.
- Un valor explícito en `settings.json` se respeta igual que antes. El valor ya no se escribe si no está definido (`skip_serializing_if`).
- Los tests de `settings.rs` cubren ambos casos.

Compatibilidad: los archivos existentes con un número siguen funcionando; solo cambia el comportamiento de quien nunca lo tocó y usa un tema claro. ¿Te parece bien o prefieres otro default?

## 2. Consulta: atajo global para enfocar el buscador de Ajustes

Hoy el buscador de Ajustes (`st-search`) se usa con clic y ↑/↓/Enter/Esc; `app.settings` solo recibe `section`. Un atajo global para escribir directo en el buscador exigiría que la app abra Ajustes con el foco en él.

1. ¿La app actúa sobre esto? Sí: cambia el foco al abrir.
2. ¿Solo la app lo ve? Sí: un mod no recibe el atajo si Ajustes no tiene el foco.
3. ¿Un mod no puede hacerlo bien? Un mod puede abrir Ajustes con `app.settings`, pero no puede darle el foco al buscador.

Propuesta: `app.settings` acepta `focus: "search"` (compatible, parámetro nuevo opcional). ¿Lo agrego o lo descartamos?

## 3. Consulta: badge del Dock con agentes que necesitan atención

Mostrar en el ícono del Dock el número de agentes en «Lo necesitan». La API de mods no toca el Dock.

1. ¿La app actúa? Sí, es del sistema.
2. ¿Solo la app lo ve? Sí.
3. ¿Un mod no puede? No: no hay API.

Propuesta: un comando `app.badge {count}` solo en macOS, apagado por defecto detrás de un ajuste. ¿Va al núcleo o lo dejamos fuera?

## 4. Consulta: estado «error al cargar» de un mod

En Ajustes › Mods un mod que falla al cargar no se distingue de uno desactivado; el error solo está en `sunna status` (campo `errors`).

1. ¿La app actúa? Es información que ya tiene.
2. ¿Solo la app lo ve? Sí: el mod roto no puede informar de su propio fallo.
3. ¿Un mod no puede? Correcto, no hay lectura de errores por mod.

Propuesta: exponer en la API de mods la lista de errores de carga por mod (solo lectura), para pintar la marca «Error al cargar» en la tarjeta. ¿Lo agrego?

## 5. Consulta: `tab.close` (⌘W) con una sola pestaña cierra el espacio

Con una única pestaña, ⌘W cierra el espacio entero en vez de solo la pestaña (la guía dice que pregunta antes salvo `confirmed`; el usuario lo vivió sin verlo). Orca deja el espacio abierto y vacío.

1. ¿La app actúa? Sí: es el comportamiento de `tab.close`.
2. ¿Solo la app lo ve? Sí.
3. ¿Un mod no puede? Correcto: el comando es del núcleo.

Propuesta: con una sola pestaña, `tab.close` cierra la pestaña y deja el espacio vacío (nueva pestaña con ⌘T), o al menos siempre pide confirmación. ¿Cuál prefieres?

## 6. Aviso: atajos de macOS alineados con Orca (`keys.macos.json`)

⌘⇧↑/↓ espacio anterior/siguiente, ⌘1–9 espacios, ⌃1–9 pestañas, ⌘⌥[ y ⌘⌥] pestañas, ⌘K paleta, ⌘L panel derecho, ⌘R renombrar pestaña, ⌘⌥R renombrar espacio. El archivo anterior queda en `src/keys/shipped/v0.1.16-keys.macos.json`, así que solo se actualizan quienes no editaron sus teclas.

## 7. Consulta: pantalla de bienvenida sin espacios (núcleo: `src/window.rs`)

Sin espacio abierto el área principal mostraba solo «No space open. Make one (cmd-n).». Ahora la pinta el mod `defaults/mods/welcome/` (logo, nombre, «Agregar proyecto», «Crear espacio de trabajo» y los atajos leídos de `keys()`). Se evitó el núcleo donde fue posible (textos, botones y atajos son del mod; los paneles Archivos/Cambios cambian solo su texto), pero el mod necesita una copia viva y un lugar donde pintarse.

Cambio en el núcleo (pequeño, solo `src/window.rs`):
- `welcome_mod()` + `wants_welcome()`: añade el slot `welcome` a `visible()` y a `owned_mods()` solo cuando hay un bloque `tab: "active"`, ningún espacio y el documento no está cargando.
- En `render_content`, la rama `None` de `Content::Context` ahora hace `render_slot("welcome")` en lugar de la cadena fija. Se elimina la lectura de `keys::listing()` de esa rama.
- Test: `welcome_only_with_no_space_open`.

1. ¿La app actúa sobre esto? No cambia lo que hace la app, solo qué se ve en ese estado vacío.
2. ¿Solo la app lo ve? Sí: la ausencia de espacio (y de pestaña activa) la conoce solo el núcleo; ningún mod se monta en un bloque que no existe.
3. ¿Un mod no puede hacerlo bien? Un mod en el layout no podría ocultarse al haber espacio sin lógica propia ni reaccionar al modelo de espacios con fiabilidad; por eso lo monta el núcleo.

Propuesta: dejarlo así (mínimo, sin API nueva, compatible: si el mod falta o falla se ve el aviso de error del slot). ¿Lo aprueba o prefiere otra forma?

Linux y Windows (`keys.json`) también siguen a Orca: ⌃T, ⌃N, ⌃P, ⌃K, ⌃B, ⌃L, ⌃⇧↑/↓, ⌃1–9 espacios, Alt+1–9 pestañas, ⌃⇧D dividir a la derecha, Alt+⇧D dividir abajo, ⌃⇧O abrir carpeta. Ojo: ⌃K, ⌃B, ⌃L, ⌃N y ⌃P son teclas de readline que antes pasaban al terminal; la regla «ctrl+letra suelta queda para el programa» ya no se cumple en estos archivos. El original anterior queda en `src/keys/shipped/v0.1.16-keys.json`.

## 8. Consulta: fijar espacios (worktrees fijados) en el modelo

Orca tiene «Mostrar también worktrees fijados en su lista». Sunna no tiene el concepto de espacio fijado (ni el campo en el modelo ni un comando), así que el ajuste no se muestra.

1. ¿La app actúa? Sí: un espacio fijado sobrevive a la limpieza de worktrees y sube en la lista.
2. ¿Solo la app lo ve? Sí: el estado «fijado» tiene que vivir en el documento de layout de la máquina (`sunna`) para que lo vean todas las pantallas conectadas.
3. ¿Un mod no puede? Parcial: un mod puede guardar una lista de claves en `shared()` (como `space.hidden`), pero esa lista no viaja con el espacio entre máquinas ni la ve la limpieza hecha por otro cliente.

Propuesta: un campo opcional `pinned` en el espacio y el comando `space.pin {id, pinned}` (compatible: ausente = no fijado). ¿Lo agrego o lo hacemos solo con `shared()` en un mod?

## 9. Consulta: orden de pestañas recientes en Ctrl+Tab

Orca ordena Ctrl+Tab por «más recientes» o por la barra. En Sunna la rueda sobre la barra de pestañas ya sigue el orden de uso (ajuste `tabs.order` en `shared()`, mod `tabs`), pero un atajo de teclado solo puede ejecutar comandos del núcleo (`tab.next`/`tab.prev`, por posición).

1. ¿La app actúa? Sí: cambia a qué pestaña lleva el atajo.
2. ¿Solo la app lo ve? Sí: un mod no recibe el atajo y no puede cambiar lo que hace `tab.next`.
3. ¿Un mod no puede? Correcto: no hay forma de que un mod enlace una tecla.

Propuesta: comando `tab.recent {direction}` que recorra las pestañas del espacio por último uso (la app ya sabe cuál se enfocó antes), para enlazarlo a Ctrl+Tab en `keys.json`. ¿Lo agrego?


## 10. Aviso: ajustes de Terminal, Alertas y Editor (núcleo: `src/settings.rs`, `src/terminal.rs`, `src/window.rs`)

Tres páginas nuevas en Configuración (Terminal, Editor de archivos, y Alertas ampliada). Cambios en el núcleo, todos opcionales y con valor por defecto igual al comportamiento anterior (`#[serde(default)]`, no se escriben si son el valor por defecto):
- `Person.terminal_padding` y `Person.terminal_opacity` (opcionales). A diferencia de las fuentes, el valor de la persona gana sobre el del layout (si no, `main.json`, que fija el relleno, lo anularía).
- `Person.scroll_on_input` (por defecto `true`): `Terminal::send` solo baja al final si está activo.
- `Person.notifications` `{enabled, done, blocked}` (todo `true`): `Window::alert` omite sonido y aviso del tipo apagado y el aviso si `enabled` es falso (el efecto de «reactivar pestaña» se mantiene).
- Tests: `person_terminal_padding_opacity_and_alerts_override_and_default_on` (settings.rs) y la guía `defaults/AGENTS.md`.

1. ¿La app actúa sobre esto? Sí: cambia el dibujo del terminal y qué alertas emite la app.
2. ¿Solo la app lo ve? Sí: el dibujo del terminal, la escritura y la emisión de alertas ocurren en el núcleo; un mod no los intercepta.
3. ¿Un mod no puede? Correcto: no hay API de mod para esos tres puntos.

No implementados (necesitarían núcleo más grande), por si los quiere: forma y parpadeo del cursor (el cursor lo fija el programa por DECSCUSR y el núcleo no distingue «por defecto»), opacidad del panel inactivo, grosor del divisor (es del layout), GPU, foco sigue al ratón, ocultar el ratón al escribir, preguntar antes de cerrar terminales activas, shell predeterminado (lo decide herdr-core), pegar con clic central (hoy: Linux pega PRIMARY, Windows el portapapeles, macOS nada), campana del terminal (no hay manejo de BEL), volumen del sonido (los reproductores externos no comparten bandera) y, en el editor, peso, alto de línea, ligaduras y suavizado (`EditorState` solo acepta `font_family`, `font_size`, `line_numbers`, `folding`, `soft_wrap`). «Suprimir con la ventana en foco»: la app ya no muestra el aviso si se está mirando ese agente.


## 11. Aviso: editor de archivos nativo (núcleo: `src/editor.rs`, `src/window/editor.rs`, `src/window.rs`, `src/commands.rs`, `src/keys.rs`, `src/macos.rs`, `vendor/gpui-kit/crates/shell`)

Decisión del dueño del fork: lo nativo va en Rust. Sunna abre, lee y edita archivos con el `Editor` de gpui-base (resaltado, números de línea, plegado, buscar y reemplazar). Cambios de núcleo, todos compatibles:
- Un panel de archivo es un panel de mod llamado `editor` con `props: { file, on? }`: no hay variante nueva en el documento de layout ni campos nuevos en `Person`; un layout guardado lo conserva como cualquier mod. La app lo dibuja nativo (sin carpeta en `mods/`) y guarda su estado en `AppWindow.editors`.
- Comandos nuevos: `file.open`, `editor.save`, `editor.preview`, `editor.reload`, `editor.goto`; `confirmed` opcional en `pane.close`, `app.quit` y `window.close` (cerrar, salir o cerrar la ventana con cambios sin guardar pregunta; los scripts deben pasar `confirmed: true`).
- Guardado seguro: `mod_files::write` (ya existente, con comparación del texto base) pasa a `pub(crate)`; `mods::read_remote` también. Archivos de otra máquina: solo lectura.
- `spaces()[].tabs[].dirty` y `panes()[].file/dirty` para los mods (la barra de pestañas muestra un punto).
- Atajos: sección `mod:editor` en `keys.json`/`keys.macos.json` (guardado el original previo como `v0.1.16b-*`); `cmd-alt-f` enlazado en `keys::install` al «reemplazar» del componente (macOS).
- `vendor/gpui-kit/crates/shell`: dos funciones públicas (`editor_highlighter_factory`, `editor_style`) para reutilizar el resaltado de sintaxis de `EditorState`.
- Soltar un archivo sobre la ventana (donde ningún terminal lo toma) lo abre; el macOS `Quit` consulta `window::unsaved_files()`.

1. ¿La app actúa sobre esto? Sí: abre y escribe archivos y decide si se puede cerrar la ventana.
2. ¿Solo la app lo ve? Sí: cerrar/salir con cambios sin guardar y el arrastre de archivos a la ventana ocurren en el núcleo; un mod no los intercepta.
3. ¿Un mod no puede? Parcial: un mod ya podía editar con `EditorState` y `writeTextFile`, pero no puede impedir el cierre ni abrir un archivo soltado en la ventana.


## 12. Aviso: «Ir a archivo» en ⌘P y «Guardar y cerrar» (núcleo: `src/window/editor.rs`, `src/window.rs`, `src/editor.rs`, `src/commands.rs`, `src/keys.rs`)

Cambios de núcleo, compatibles:
- Comando nuevo `editor.save_close {then, panes?}`: guarda los archivos con cambios y después ejecuta `then {name, args}` como confirmado; si un archivo no se guarda (conflicto en disco, error de escritura) no ejecuta nada, enfoca ese panel y avisa. Estado: un campo `AppWindow.save_close` y el modelo puro `editor::SaveThen` (con tests). El diálogo de cambios sin guardar (cerrar pestaña, panel, espacio, ventana o salir) ahora ofrece Guardar y cerrar / Descartar / Cancelar; el mod `confirm` aceptó `confirm_label` y `extra` (opcionales) y se arregló su cuerpo, que quedaba vacío.
- Teclas: `cmd-p` (`ctrl-p`) pasa de `mod.open:picker` a `mod.open:goto-file` (mod nuevo, sin núcleo: lista con `rg --files`/`git ls-files`/`find` mediante `exec`); el selector de pestañas pasa a `cmd-alt-p` (`ctrl-alt-p`) y sigue en la paleta. Los keys previos quedan en `src/keys/shipped/v0.1.16c-*`.

1. ¿La app actúa sobre esto? Sí: decide si se cierra algo con cambios sin guardar y guarda antes de cerrar.
2. ¿Solo la app lo ve? Sí: el guardado asíncrono y el cierre posterior ocurren en el núcleo, donde viven los editores.
3. ¿Un mod no puede? Correcto para guardar y cerrar (el estado de los editores es nativo). Ir a archivo es solo mod; el cambio de tecla es de los defaults.


## 13. Aviso: la ventana de Configuración pasa a Rust, por páginas (núcleo: `src/window/settings/*`, `src/window.rs`)

Decisión del dueño del fork: lo nativo va en Rust. La ventana `frame-settings` (JS) se porta página por página a `src/window/settings/` (cargado desde `window.rs` con `#[path]`, un solo `mod settings_ui;`). Hoy son nativas General, Git y espacios, Editor de archivos y Alertas, más el buscador de ajustes y el menú. Las demás páginas siguen en JS y cada ventana lleva a la otra al elegir una sección que no tiene (`app.settings {section}` sigue siendo el único comando; mismos argumentos). Compatibles: ningún campo nuevo en `Person` ni claves nuevas en `shared()`; las páginas escriben settings.json como objeto (conservando claves ajenas, sin escribir sobre un archivo roto) y `shared()` con los mismos valores por defecto.
- `window.rs`: campo `view: Option<AnyView>` en `Overlay` (una vista nativa en vez de un mod), una rama en `app.settings`, y un id propio `settings-native`. `SUNNA_SETTINGS_CLASSIC=1` fuerza la ventana JS para comparar (se retira al terminar el port).
- `gpui-base` ya existente (Input, Scrollbar); sin dependencias nuevas.

1. ¿La app actúa sobre esto? Sí: la ventana es la superficie de la app para cambiar su configuración y sus comandos.
2. ¿Solo la app lo ve? No: un mod puede dibujarla (así era); se porta por decisión del dueño, no por necesidad técnica.
3. ¿Un mod no puede? Un mod puede; el motivo es el criterio de «lo nativo en Rust».


## 14. Aviso: Máquinas, Actualizaciones, Privacidad y Mods nativos (núcleo: `src/window/settings/{machines,updates,privacy,modlist,modstore,live,timefmt}.rs`)

Siguiente tramo del port de la ventana de Configuración (ítem 13). Sin campos nuevos en `Person`, sin claves nuevas en `shared()` y sin comandos nuevos: las páginas usan los que ya existían (`mod.open machine-add`, `mod.clone`, `app.check_updates`, `app.update`, `app.update_core`, `app.whats_new`, `app.feedback`, `app.settings {section}`).
- Máquinas: `hc machine ls/update/rename/rm` (mismos argumentos), lista releída cada ~3 s mientras la página está abierta.
- Actualizaciones y Privacidad: leen `update::status()` y `telemetry::status()` (lo que ya leían los mods); el interruptor escribe `telemetry: false` (o borra la clave) en settings.json como antes. Las notas de versión siguen siendo la ventana «Novedades» (`app.whats_new`).
- Mods: la lógica de disco y red (`modstore`) reemplaza a `fsx.js`/`store.js`/`mods.js`: instalar desde `owner/repo[@ref]`, URL git o carpeta (con `git` y el mismo `.herdr-install.json`), activar/desactivar (mover a `mod-store/disabled`), eliminar con doble clic, comprobar y revisar actualizaciones, y la galería de GitHub (topics `herdr-mod`/`herdr-theme`/`herdr-layout`; `curl` y, si falla, `gh api`). Mismos formatos en disco, así que lo ya instalado sigue funcionando.
- Horas locales (`timefmt`): `localtime_r` declarado a mano en Unix (sin dependencias nuevas) y UTC en Windows (solo afecta a «Verificado a las HH:MM» y «Enviado …»).
- Se borran `frame/settings/{mods,store,release-notes}.js` y las secciones Máquinas/Actualizaciones/Privacidad/Mods de `main.js`; `fsx.js` queda porque lo usa `sidebar.js`. Se quitan `scripts/mods-section.test.ts` y `scripts/store.test.ts` (migrados a tests Rust) y el test bun del botón del fork.

1. ¿La app actúa sobre esto? Sí: la ventana es la superficie de la app para sus actualizaciones, la telemetría, los mods y las máquinas.
2. ¿Solo la app lo ve? No: un mod podía dibujarla; se porta por decisión del dueño.
3. ¿Un mod no puede? Un mod podía; el motivo es el criterio de «lo nativo en Rust».


## 15. Aviso: Apariencia, Terminal, Teclado y Agentes nativos (núcleo: `src/window/settings/{look,theme_ui,theme_import,sidebar,fonts,terminal,keyboard,agents}.rs`, `src/mods.rs`)

Tramo siguiente del ítem 13. Sin campos nuevos en `Person` ni claves nuevas en `shared()`; `app.settings {section}` igual. Cambios de núcleo:
- `mods.rs`: `known_agents` pasa a `pub(crate)`; nuevas `pub(crate) fn fonts()` (fuentes instaladas) y `layout_snapshot()` (el diseño activo como JSON, para el editor de barra lateral).
- Teclado: botón nuevo «Restaurar atajos originales» (no existía en la página JS): mueve `keys.json` al lado como `keys.json.restored-<hora>` y escribe los atajos de fábrica. Los atajos con forma de objeto se mueven completos al cambiarlos.
- Foco por teclado: Tab/Shift+Tab recorren los controles de la ventana (`tab_index` + anillo `focus_visible` en `controls.rs`, para todas las páginas).
- Se borran `frame/settings/{themes,sidebar,theme-import,fsx}.js`, `terminal()` de `terminal.js`, `look()` de `orca.js` y esas secciones de `main.js`; `scripts/themes-from-ghostty.ts` y `scripts/theme-import.test.ts` se quitan (el importador vive en `theme_import.rs`, con sus pruebas). `scripts/themes-from-ghostty.ts` generaba `defaults/themes`: si se necesita de nuevo hay que reescribirlo contra Rust.
- Pruebas de render JS de `frame/settings` (fuentes, tarjetas de tema, vistas previas, Option en macOS) eliminadas; las cubren pruebas Rust de cada módulo.
- Pendiente ajeno a este tramo: `settings_sections_and_updates` y `settings_keep_notes_in_the_whats_new_window` (render_tests.rs) siguen apuntando a la página JS de Actualizaciones ya borrada.

1. ¿La app actúa sobre esto? Sí. 2. ¿Solo la app lo ve? No; se porta por decisión del dueño. 3. ¿Un mod no puede? Puede; criterio de «lo nativo en Rust».


## 16. Aviso: se retira el frame mod JS de Configuración (núcleo: `src/window.rs`, `src/window/settings/*`, `src/commands.rs`, `build.rs` sin cambios)

Cierre del port (ítems 13 a 15): las 12 secciones son nativas, así que `defaults/frame/settings/` desaparece por completo (`main.js`, `orca.js`, `terminal.js`, `search.js`, `fsx.js`, `mod.json`). Sin campos nuevos en `Person`, sin claves nuevas en `shared()`.
- `app.settings {section?}` conserva sus argumentos: abre la ventana nativa; una sección desconocida cae en la primera (`look`). Se quitan el spec `frame-settings`, la variable `SUNNA_SETTINGS_CLASSIC`, la lista `native` de props y el salto entre ventanas (`go` ya no delega). La descripción de `sunna commands` y `defaults/AGENTS.md` listan las 12 secciones.
- Los SVG que embebe Rust (`include_bytes!`) pasan a `assets/icons/settings/`. Los mods `awake-menu` y `worktree` usaban `frame/settings/check.svg`: ahora usan `mods/workspace-chrome/check.svg` (mismo ícono).
- Pruebas: los tests Rust que leían `orca.js`/`search.js` se reemplazan por pruebas de las tablas Rust (secciones válidas, textos en es/en, claves `settings.orca_*` usadas por las páginas). Se borra `scripts/settings.test.ts`; `orca-settings.test.ts` y `terminal-settings.test.ts` conservan solo las pruebas de los mods que leen los ajustes (y entran a `just test`). En `render_tests.rs` se elimina `settings_sections_and_updates` (cubierto en `updates.rs`) y `settings_keep_notes...` queda como `the_whats_new_window_shows_the_notes` (solo la ventana Novedades).
- Locales: se eliminan 15 claves `settings.*` sin uso (`also_fixed`, `month_day`, meses, `theme_edit`); Novedades usa sus propias claves `whatsnew.*`.

1. ¿La app actúa sobre esto? Sí. 2. ¿Solo la app lo ve? No; es eliminación de código ya reemplazado. 3. ¿Un mod no puede? Puede; criterio de «lo nativo en Rust».


## 17. Aviso: «Ir a archivo» y el diálogo de confirmación pasan a Rust (núcleo: `src/goto.rs`, `src/window/goto_file.rs`, `src/window/confirm.rs`, `src/commands.rs`, `src/keys.rs`)

Se retiran los mods de fábrica `goto-file` y `confirm`. Sin campos nuevos en `Person` ni claves nuevas en `shared()`.
- Comando nuevo `file.goto` (alterna el selector; ⌘P / Ctrl+P en `keys.json` y `keys.macos.json`). Los defaults anteriores se guardan como `src/keys/shipped/v0.1.16d-keys{,.macos}.json` y se registran en `keys.rs`. Un `keys.json` editado que diga `mod.open:goto-file` sigue funcionando: `keys::current` lo traduce a `file.goto` (salvo que exista una carpeta de mod `goto-file` del usuario) y `mod.open {name:"goto-file"}` también.
- Selector nativo: ranking difuso idéntico al de `model.js` (nombre antes que ruta, prefijo, recientes primero con consulta vacía, tope 60000, 200 filas), listado con `rg`/`git ls-files`/`find` en segundo plano, flechas/AvPág, Enter abre con `file.open`, Esc cierra, aviso de «primeros 60000». Los recientes ahora viven en `<app>/data/goto-file/recent.json` (los del `localStorage` del mod no se migran). En el mod, una fila sin letras resaltadas no pintaba su texto; en Rust siempre se pinta.
- Diálogo nativo `Ask { title, detail, confirm_label, command, extra }` (Cancelar / principal / acción extra; Enter ejecuta la extra si existe). Lo usan cerrar espacio/pestaña/panel, salir, «Guardar y cerrar / Descartar / Cancelar» y recargar archivo.
- Compatibilidad: `open_overlay` redirige `openOverlay({mod:"confirm", props})` y `{mod:"goto-file"}` a las versiones nativas mientras el usuario no tenga una carpeta de mod con ese nombre (si la tiene, manda la suya). Se quitan ambos de `SHIPPED_MODS`, y se retiran dos pruebas de render del mod `confirm`.
- `settings/mod.rs`: `mod controls` pasa a `pub(super)` para reutilizar `input_style()` en el selector.

1. ¿La app actúa sobre esto? Sí: abre archivos y ejecuta comandos con confirmación. 2. ¿Solo la app lo ve? Parcial: ve el foco y el panel enfocado. 3. ¿Un mod no puede? Puede; criterio de «lo nativo en Rust».


## 18. Aviso: bloques nativos del layout; barra de estado, bienvenida y marca/barra superior pasan a Rust (núcleo: `src/layout.rs`, `src/window/native.rs`, `src/window.rs`, `defaults/layouts/main.json`)

Mecanismo nuevo, pensado una sola vez para las fases 3b/4/5: un bloque de `content` puede ser `{"native": "<nombre>", "props": {...}}` (`Content::Native`, campos opcionales, `props` con `#[serde(default)]`). `render_native(nombre, props)` en `src/window/native.rs` pinta el bloque en Rust; un nombre desconocido muestra un bloque vacío (un layout de una versión más nueva carga igual). Nombres de esta fase: `status`, `brand`, `sidebar-footer`, `welcome`. Las fases siguientes solo agregan un nombre al `match` de `render_native`.
- Compatibilidad: `layout::native_alias` traduce `{"mod":"workspace-status"}`, `{"mod":"welcome"}` y `{"mod":"workspace-chrome"}` (con `props.footer` = pie de la barra lateral, si no la marca) al bloque nativo, salvo que el usuario tenga una carpeta de mod con ese nombre (manda la suya, igual que `confirm` y `goto-file`). Los layouts guardados, de usuario y `win98` siguen funcionando; `main.json` usa `native` en `brand`, `sidebar-footer` y `status`. El estado vacío nativo se pinta en la rama `None` de `Content::Context`.
- Paridad: mismas claves de `shared()` (`status.items`, `status.format`, `sidebar.width`, `sidebar.collapsed`), mismos textos (`workspace-status.*`, `workspace-chrome.*`, `welcome.empty_*`), mismo atajo de paleta leído de `keys.json` (con caché de 1 s), y la marca sigue el ancho de la barra lateral (`sync_brand` en cada render, solo si el bloque `brand` es el nativo): oculta o riel = 178 px en macOS, 108 en el resto; 84 px de margen para los botones de macOS salvo en pantalla completa.
- Se borran `defaults/mods/{workspace-status,welcome}/` y el código de `workspace-chrome` (`main.js`, `mod.json`), más `scripts/{welcome,workspace-chrome}.test.ts` y el test de la barra de estado de `orca-settings.test.ts`; sus pruebas pasan a `src/window/native.rs` y se quitan dos pruebas de render de `render_tests.rs`. `defaults/mods/workspace-chrome/` queda solo con los `*.svg` (los dibujan otros mods y la app: no es un mod, se excluye de `SHIPPED_MODS` y de la lista de mods instalados); `coffee.svg` pasa a `assets/icons/status/`.
- Sin campos nuevos en `Person` ni claves nuevas en `shared()`.

1. ¿La app actúa sobre esto? Sí: mueve el ancho de la barra lateral y de la marca, abre menús y comandos. 2. ¿Solo la app lo ve? Sí en parte: el arrastre del borde de la barra lateral y el pantalla completa. 3. ¿Un mod no puede? Puede; criterio de «lo nativo en Rust».


## 19. Aviso: `tabs` y `spaces` pasan a Rust; la barra lateral se oculta entera, sin riel (núcleo: `src/window/{tabs,spaces,native}.rs`, `src/mods.rs`, `src/layout.rs`, `defaults/layouts/main.json`)

- Bloques nativos `tabs` y `spaces` (alias `{"mod":"tabs"}` / `{"mod":"spaces"}`; una carpeta de mod de usuario con el mismo nombre sigue ganando). Mismas claves de `shared()` y de `Person`, mismos comandos `sunna do`, mismas teclas; el estado persistido sigue en `data/tabs/store.json` y `data/spaces/store.json`.
- Núcleo: `mods::snapshot_with` y `mods::set_agent_order` (`pub(crate)`) para leer la instantánea publicada y fijar el orden de agentes; nada nuevo en la API de mods.
- La barra lateral ya no tiene riel de 44 px: el botón y ctrl-shift-b / cmd-b la muestran u ocultan entera (`layout.toggle`) y la barra superior queda igual en ambos estados (ancho de la marca constante). Un `sidebar.collapsed=true` guardado, o una barra lateral de menos de 160 px, se migra a «oculta» al ancho que tenía (`sync_sidebar`).
- Pie de la barra lateral: Configuración, Ayuda (`workspace-chrome.help`, nueva en en/es) y Agregar máquina; el bloque `sidebar-footer` del layout pasa a 101 px.
- Se borran `defaults/mods/{tabs,spaces}/`, `scripts/{tabs,spaces}.test.ts` y sus pruebas en `mod-storage`/`orca-settings`; las pruebas pasan a `src/window/{tabs,spaces}.rs`.
- Siguen en JS (próxima fase): menú emergente, new-menu, agents y demás mods hermanos, que se abren con `openOverlay` como hasta ahora.

1. ¿La app actúa sobre esto? Sí: selecciona, mueve y cierra espacios y pestañas, y oculta la barra lateral. 2. ¿Solo la app lo ve? Sí en parte: arrastre de pestañas, rueda y ocultar la barra. 3. ¿Un mod no puede? Puede; criterio de «lo nativo en Rust».


## 20. Aviso: los paneles `files` y `changes` pasan a Rust (núcleo: `src/window/{files,changes,native}.rs`, `src/layout.rs`, `src/window/tabs.rs`, `src/mods.rs`)

- Bloques nativos `files` y `changes` (alias `{"mod":"files"}` / `{"mod":"changes"}`; una carpeta de mod de usuario con el mismo nombre sigue ganando). Mismas claves de `shared()` (`files.ignored`, `files.open`, `changes.groups`, `changes.base`, ...), mismos comandos (`file.open`) y mismas teclas; nada nuevo en `Person`.
- Núcleo: `mods::run_remote_with_limit` pasa a `pub(crate)` para listar carpetas y correr git en otra máquina; `native::tone` (texto secundario/estados llevado a 4.5:1 sobre el fondo y las filas) y `native::sync_panels`. Los botones de panel de la barra de pestañas salen de `NATIVE_PANELS` (`tabs.rs`) cuando no hay carpeta de mod.
- Diferencias asumidas: «Historial» se reemplaza por la sección «Commits en la rama»; el error de git se muestra en línea, no como aviso; descartar pide una confirmación en la propia fila («¿Descartar?»); no hay insignia de PR (requiere `gh`).
- Se borran `defaults/mods/{files,changes}/`, `scripts/{files,changes}.test.ts` y sus pruebas en `orca-settings`/`terminal-settings`; las pruebas pasan a `src/window/{files,changes}.rs` (incluye repositorios git reales en un directorio temporal y contraste en todos los temas incluidos).
- Siguen en JS: search, agents y activity (próxima fase).

1. ¿La app actúa sobre esto? Sí: abre archivos, hace stage/commit/descartar con git. 2. ¿Solo la app lo ve? Sí en parte: vigila carpetas y el repositorio. 3. ¿Un mod no puede? Puede; criterio de «lo nativo en Rust».


## 21. Aviso: `search`, `agents` y `activity` pasan a Rust; fin de la fase 4 de paneles (núcleo: `src/window/{search,agents,activity,native}.rs`, `src/layout.rs`, `src/window.rs`, `defaults/layouts/main.json`)

- Bloques nativos `search`, `agents` y `activity` (alias `{"mod":"search"}`, `{"mod":"agents"}` y `{"mod":"activity"}`; una carpeta de mod de usuario con el mismo nombre sigue ganando). Mismas claves de `shared()` (`agents.sort`, `activity.events`, `activity.seen`, claves de búsqueda), mismo `localStorage` del mod (`data/agents/store.json`: `changed`, `pins`), mismos comandos `sunna do` y mismas teclas.
- Núcleo: `missing_mods` ya no avisa de un mod de fábrica que pasó a nativo; la ventana registra el log de actividad (`activity.events`) cuando el layout tiene un bloque `agents` o `activity` (antes solo lo escribía el mod `agents`, que no estaba en el layout por defecto, así que el registro nunca se llenaba).
- Accesibilidad: foco visible con Tab y flechas en Archivos, Cambios, agentes y actividad; filas de al menos 32 px; contraste con `native::tone`.
- Se descarta el modo riel de la lista de agentes (la barra lateral ya no tiene riel).
- Se borran `defaults/mods/{search,agents,activity}/`, `scripts/{search,agents,activity,mod-storage}.test.ts`; las pruebas pasan a `src/window/{search,agents,activity}.rs`.
- Siguen en JS: worktree, bots, menús y demás mods hermanos, que se abren como hasta ahora.

1. ¿La app actúa sobre esto? Sí: busca en carpetas, enfoca agentes y mantiene el registro de actividad. 2. ¿Solo la app lo ve? Sí en parte: los cambios de estado de los agentes se ven con la ventana abierta. 3. ¿Un mod no puede? Puede en teoría; criterio de «lo nativo es de la app» del dueño del fork.


## 22. Aviso: lista de la barra lateral con agrupar, ordenar, filtros y fijados (núcleo: `src/window/spaces.rs`, `src/window/spaces/view.rs`, `src/commands.rs`, `src/window.rs`, `defaults/keys.json`)

- Menú nativo (icono de lista del encabezado «Proyectos», con insignia del número de filtros) con Agrupar por (Ninguno, Estado, PR, Proyecto), Ordenar por (Nombre, Actividad de agentes, Reciente, Proyecto, Manual), Orden de proyectos (Manual, Reciente), Filtros (ocultar en reposo, ocultar rama por defecto, ocultar creados por bots, solo estos proyectos con buscador) y «Limpiar filtros»; sección Fijados arriba; botón plegar/desplegar todo; «Ir al espacio actual».
- Comandos nuevos: `space.status {id, status}`, `space.pin {id, pinned}`, `sidebar.view`, `sidebar.filter`, `sidebar.collapse_all` (tecla `ctrl-shift-h` en Linux/Windows como Orca; en macOS la de Orca, ⌘⌥P, ya es el selector, así que queda sin tecla), `sidebar.reveal_current`.
- Todo vive en `shared()` (`sidebar.group_by`, `sidebar.sort_by`, `sidebar.project_order`, `sidebar.filters`, `space.status`, `space.pinned`, `space.bot_made`); no hay campo nuevo en el modelo ni en el documento de layout. Esto responde por ahora al ítem 8 con la variante «solo `shared()`»: el estado y el fijado no viajan con el espacio entre máquinas ni los ve otro cliente. Por defecto la lista se ve como antes.
- Límites: el estado de la PR necesita `gh`, así que «PR» agrupa todo en «Sin PR»; «en reposo» = ningún agente trabajando/bloqueado y nada escrito en 30 minutos (Sunna no duerme espacios); «rama por defecto» = main, master o trunk; el modelo no registra quién creó un espacio, por lo que «creados por bots» lee `space.bot_made` y hoy nadie lo escribe.

1. ¿La app actúa sobre esto? Sí: cambia qué espacios se ven y en qué orden. 2. ¿Solo la app lo ve? Sí en parte: el orden y el plegado de la lista. 3. ¿Un mod no puede? Puede; criterio de «lo nativo es de la app» del dueño del fork. Pendiente de tu decisión: ¿`pinned` y `status` pasan al documento de layout (viajan con el espacio) o se quedan en `shared()`?

## 23. Aviso: menús contextuales nativos de espacio, proyecto y máquina (núcleo: `src/window/{ctxmenu,spaces}.rs`, `src/window/spaces/{menus,details}.rs`, `src/commands.rs`, `src/window.rs`, `defaults/mods/menu/main.js`)

Qué cambia: el clic derecho (y el botón «⋯») de un espacio, un proyecto o una máquina abre un menú nativo reutilizable (`ctxmenu.rs`: posición del puntero, submenús, ↑↓←→ Enter Esc, cierre al hacer clic fuera). Cada ítem es un comando nuevo, así que `sunna do` hace lo mismo: `space.read`, `space.open_in`, `space.copy`, `space.details`, `space.remove`, `project.icon`, `project.hide`, `project.open_in`, `project.copy`, `project.remove`, `project.worktree`, `machine.rename`, `machine.check`, `machine.remove`, `sidebar.menu`. Claves nuevas de `shared()` (opcionales): `space.unread`, `space.notes`, `project.icons`. Se borran las pestañas `space` y `project` del mod JS `menu`.

1. ¿La app actúa sobre esto? Sí: quita worktrees (`git worktree remove`), olvida máquinas y cambia qué se ve. 2. ¿Solo la app lo ve? Sí: los menús nacen del puntero y del estado de la barra. 3. ¿Un mod no puede? No lo hacía bien: sin teclado ni submenús propios y sin diálogo de confirmación nativo.

Límites: sin «Dormir» (un terminal terminado quita su panel; un espacio dormido no se puede representar sin núcleo nuevo); sin «Desconectar» máquina (`hc machine` no tiene ese comando); los submenús se abren dentro del mismo menú (con «‹» para volver), no al costado; sin menú en la fila «Agregar máquina». Quitar un worktree conserva la rama.

Pendiente de decisión: si el núcleo debe ofrecer «dormir» un espacio y «desconectar» una máquina.

## 24. Aviso: cursor de teclado en la lista de la barra lateral y pulido de accesibilidad de la fase 5b (núcleo: `src/window/spaces.rs`, `src/window/spaces/{view,details}.rs`, `src/window/ctxmenu.rs`, `src/commands.rs`, `src/window.rs`, `src/i18n.rs`, `defaults/keys*.json`)

- Comando nuevo `sidebar.focus` (teclas `ctrl-shift-e` en Linux/Windows y `cmd-shift-e` en macOS): da el teclado a la lista de espacios. Con el foco, ↑ ↓ Inicio Fin RePág AvPág mueven un cursor con anillo de acento; Enter o Espacio muestran el espacio (o pliegan el encabezado, o enfocan el agente); → y ← despliegan y pliegan; la tecla de menú o Shift+F10 abren el menú de la fila (Esc devuelve el teclado a la lista). Un terminal con foco se queda con Tab, por eso hace falta un comando para entrar.
- Los originales anteriores de `keys.json` y `keys.macos.json` quedan en `src/keys/shipped/v0.1.16e-*` (eran los de la confirmación de la fase 5a, que no se había guardado).
- `i18n::tn(clave, n, vars)` elige `<clave>.one` o `<clave>.other`; el panel Actividad, Cambios, Búsqueda y la barra de estado dejan de decir «1 eventos», «1 archivos», «1 agentes».
- Todos los controles de la lista son tab stops con anillo y Enter/Espacio; filas de al menos 32 px (los botones «⋯», «+» y de worktree, dentro de una fila de 32, miden 28 px); textos secundarios con `native::tone` (4,5:1).

1. ¿La app actúa sobre esto? Sí: cambia a dónde van las teclas de la ventana. 2. ¿Solo la app lo ve? Sí: el foco y el cursor son de la ventana. 3. ¿Un mod no puede? No: no ve el foco de la lista nativa.


## 25. Aviso: un solo diálogo nativo para toda la app; Agregar proyecto, Agregar máquina, worktrees y prompt pasan a Rust (núcleo: `src/window/dialog.rs`, `src/window/{confirm,prompt,add_project,machine_add,goto_file}.rs`, `src/window/spaces/{details,worktree}.rs`, `src/window.rs`, `src/commands.rs`, `defaults/AGENTS.md`, `src/SKILL.md`)

Qué cambia: todos los diálogos usan un componente (`dialog.rs`): tarjeta centrada de 420 px (480 en formularios), radio 14, borde completo de 1 px, sombra amplia, título y descripción, campos de 34 px con anillo de foco de 2 px, pie con Cancelar y botón principal a la derecha (la acción destructiva es un rojo tintado con borde completo), botón ✕ de 32 px siempre visible y fondo atenuado. Tab y Mayús+Tab recorren los campos, los botones y el ✕ sin salir de la tarjeta; Enter confirma el botón principal; el foco inicial es el primer campo, el botón principal, o Cancelar si la acción destruye. El contraste de todo texto (título, descripción, etiquetas, botones) se mide en una prueba a 4,5:1 en un tema oscuro y uno claro.

Migrados sin cambiar comportamiento ni comandos: confirmación (incluye Guardar y cerrar), Editar detalles, Ir a archivo (conserva su lista, ahora con cabecera, ✕ y fondo atenuado), prompt, Agregar proyecto (menú, clonar y crear), Agregar máquina y worktree (nuevo, abrir, quitar y limpiar). Se borran los mods `add-project`, `worktree`, `machine-add` y `prompt` (y sus tests bun); `openOverlay({mod: ...})` y `mod.open` con esos nombres abren el diálogo nativo con las mismas props, salvo que la persona tenga una carpeta de mod con ese nombre. El menú de Agregar proyecto pasa a ser el menú contextual nativo.

Comando nuevo: `project.add {step?: menu|browse|clone|create}` (solo para una persona; un script usa `space.new {folder}`). Sin campos nuevos en el modelo ni en `shared()`; las claves `worktree.*` y `space.*` siguen igual. `settings_ui::timefmt` pasa a ser visible dentro de `window` (la fecha de `{date}` en las ramas).

Límites: el menú de Agregar proyecto ya no muestra la línea de ayuda bajo cada opción; el prompt no selecciona de entrada el texto inicial (el campo no ofrece selección por API); el nombre de rama no se normaliza a NFC salvo el hangul.

1. ¿La app actúa sobre esto? Sí: clona, crea repositorios, añade máquinas y crea o borra worktrees. 2. ¿Solo la app lo ve? Sí en parte: el teclado y el foco de cada diálogo son de la ventana. 3. ¿Un mod no puede? Puede, pero el dueño del fork decidió que todo lo propio de la app va en Rust.


## 26. Aviso: paleta, Ir a, carpetas, novedades, bienvenida y comentarios pasan a diálogos nativos (núcleo: `src/window/{palette,picker,folders,whatsnew,welcome_dialog,feedback_dialog}.rs`, `src/window/dialog.rs`, `src/window.rs`, `src/mods.rs`, `build.rs`, `defaults/AGENTS.md`, `src/SKILL.md`)

Qué cambia: los mods de fábrica `picker` y `folders` y los frames `palette`, `whatsnew`, `welcome` y `feedback` se borran (con sus tests bun) y se reescriben en Rust sobre el diálogo común. `defaults/frame/` queda vacío (`build.rs` tolera la carpeta ausente). Los comandos `app.palette`, `app.whats_new`, `app.welcome`, `app.feedback` y `mod.open`/`openOverlay` con `picker`, `folders`, `frame-*` siguen igual (abren el nativo salvo carpeta de mod propia). Las claves `shared()` (`frame.welcomed`, `workspace.panels`, `palette.recent`, de folders) y las de localStorage se conservan. Regla nueva en `dialog.rs`: el fondo atenuado es visible y un clic fuera cierra solo si no hay texto escrito. `mods::read_snapshot` da lectura del estado de mods al nativo.

Límites: `help` sigue siendo un mod JS; Enter justo tras abrir la bienvenida no avanza hasta que la ventana recibe un clic (el foco inicial no se ve como foco real del teclado en la instancia aislada; sin verificar en una ventana en primer plano).

1. ¿La app actúa sobre esto? Sí: abre paneles, ejecuta comandos y guarda la bienvenida. 2. ¿Solo la app lo ve? Sí en parte: foco y teclado de la ventana. 3. ¿Un mod no puede? Puede, pero el dueño del fork decidió que lo propio de la app va en Rust.



## 27. Aviso: grupos de proyectos, linaje de worktrees, arrastrar para reordenar y secciones por máquina (núcleo: `src/window/spaces/{groups,drag,menus,view,worktree}.rs`, `src/window/spaces.rs`, `src/window/{dialog,native}.rs`, `src/window.rs`, `src/commands.rs`, `defaults/AGENTS.md`, `src/SKILL.md`)

Qué cambia: la barra lateral permite grupos de proyectos definidos por la persona, linaje padre/hijo entre espacios (anidado con guías de glifo), contador por máquina cuando hay más de una, y arrastrar para reordenar espacios, proyectos y grupos (o soltar un espacio sobre una cabecera de estado para cambiarle el estado). Alternativa de teclado: Alt+↑/↓.

Comandos nuevos de `sunna do`: `group.new|rename|color|delete|move|reorder|refresh`, `project.move`, `space.parent`, `space.worktree`. `group.delete` solo desagrupa (no borra proyectos) y pide confirmación. Claves nuevas de `shared()`, todas opcionales (vacío = comportamiento anterior): `sidebar.groups`, `sidebar.project_keys` (orden manual de proyectos), `space.parents`. No cambia el modelo del núcleo ni `herdr-core`; el linaje y los grupos viven solo en `shared()`.

Preguntas abiertas para Can: (a) el linaje se guarda en `shared()` y no en el núcleo; si un worktree se borra fuera de la app, la entrada se poda al sincronizar. Si se quiere que el núcleo conozca el padre, haría falta un campo en el espacio. (b) Arrastrar entre máquinas se bloquea; solo se reordena dentro del bloque de cada máquina.

Límites: los menús contextuales no se pudieron abrir con clic derecho real en la instancia aislada (`orca computer` los rechaza en segundo plano); se verificaron con `sidebar.menu`. No se capturó una imagen a mitad de arrastre con el indicador visible ni se probó el autodesplazamiento en un navegador real, y no se creó un worktree hijo pulsando el botón del diálogo.

1. ¿La app actúa sobre esto? Sí: cambia el orden, el estado y la jerarquía de espacios. 2. ¿Solo la app lo ve? Sí en parte: el arrastre y el foco son de la ventana. 3. ¿Un mod no puede? Puede, pero el dueño del fork decidió que lo propio de la app va en Rust.

## 28. Aviso: pulido de accesibilidad de la barra lateral (núcleo: `src/window/spaces.rs`, `spaces/{drag,groups}.rs`, `src/window/native.rs`, `src/settings.rs`)

Qué cambia: objetivos de 32 px, guías de linaje dibujadas con contraste medido por test, Esc cancela un arrastre en curso, Alt+↑/↓ se mueve entre hermanos del mismo padre (antes no avanzaba sobre filas anidadas), spinner al refrescar un grupo. Sin comandos ni claves nuevas.

1. ¿La app actúa sobre esto? Sí: foco, teclado y arrastre. 2. ¿Solo la app lo ve? Sí: entrada de la ventana. 3. ¿Un mod no puede? No con el mismo detalle (foco y gestos nativos).

## 29. Aviso: bots, limpieza automática de espacios y menús emergentes nativos (núcleo: `src/window/{bots,cleanup,popmenus,help,ctxmenu}.rs`, `spaces/worktree.rs`)

Qué cambia: los mods `bots`, `menu`, `new-menu`, `awake-menu` y `help` pasan a Rust. Comandos nuevos: `bot.new`, `bot.run`, `bot.enable`, `bot.delete`, `cleanup.run`, y los ya declarados `worktree.new|open|cleanup`. Claves de `shared()` nuevas: `cleanup.auto|every_hours|merged|unchanged|unchanged_days|bots|last|last_run|log`. El almacenamiento de bots (`bots/<name>/{SOUL.md,bot.json,runs.json}`) y las claves `bots.new`/`bots.templates` se conservan. Un planificador de la ventana revisa cada minuto los bots y la limpieza automática (apagada de fábrica; nunca toca el checkout principal, espacios fijados, con agente trabajando ni con cambios sin confirmar). Se corrige un fallo del mod anterior: el argv de un bot usaba solo las banderas `yolo` del agente.

1. ¿La app actúa sobre esto? Sí: arranca agentes y elimina worktrees por sí sola. 2. ¿Solo la app lo ve? Sí: el planificador corre con el panel cerrado y la limpieza cierra espacios y borra carpetas. 3. ¿Un mod no puede? No con seguridad (cierre de espacios más `git worktree remove` coordinados, y el menú sobre el puntero).

Preguntas abiertas para Can: (a) ejecutar bots con el agente real y avisar por notificación al terminar necesita al núcleo (hoy se abre una pestaña `bot <name> #n` y no se notifica el resultado); (b) la limpieza solo mira esta computadora, no máquinas remotas; (c) la regla «sin cambios» usa la fecha de creación de la carpeta del checkout, el núcleo no guarda la del espacio.

Límites: el clic derecho real sobre pestañas y paneles no se pudo comprobar (`orca computer` lo rechaza en segundo plano); no se ejecutó un bot con un agente real (cuesta tokens): se verificaron el bloqueo por límite diario y el resto del flujo; no se verificó en Windows ni Linux.

## 30. Aviso: toasts, cabecera de panel, peek, comandos rápidos y terminal flotante nativos (núcleo: `src/window/{toast,panehead,peek,quick,floatterm}.rs`)

Qué cambia: los mods `toast`, `pane`, `peek` y `terminals-pane` pasan a Rust (un layout que los nombra recibe el bloque nativo salvo que la persona conserve una carpeta de mod con ese nombre). Comandos nuevos de `sunna do`: `app.notify`, `terminal.quick`, `quick.set`, `quick.edit`, `quick.delete`, `terminal.float`; claves de `shared()`: `quick_commands`, `float.terminal`; tecla por defecto `cmd-alt-a` / `ctrl-alt-a`. `terminals-bar` y `win98` se quedan como mods de ejemplo (layout y skin de terceros, sin lógica de la app).

1. ¿La app actúa sobre esto? Sí: escribe en el terminal enfocado, arranca un shell propio y descarta avisos. 2. ¿Solo la app lo ve? Sí: foco, cola de avisos y ventana flotante son de la ventana. 3. ¿Un mod no puede? Con el mismo foco y teclado, no.

Preguntas abiertas para Can: (a) el terminal flotante se crea sin etiqueta `herdr.space` para que no se adopte como pestaña; ¿el núcleo debería tener un concepto de terminal «de la ventana»? (b) `app.notify` es un comando nuevo pensado para agentes; ¿prefieres que `notify` quede solo en mods? (c) la cola de avisos (máximo 4 visibles) vive en la app.

Límites: no se verificó en Windows/Linux, clic derecho ni avisos con agentes reales (cuesta tokens); la cabecera de panel no tiene navegación por teclado con lector de pantalla verificada.

## 31. Aviso: Tab global, Esc en avisos, terminal flotante centrada y estado recordado (núcleo: `src/window.rs`, `src/window/{movable,spaces}.rs`)

Qué cambia: (a) Tab y Mayús+Tab recorren los controles de los paneles nativos cuando ni un diálogo, menú ni terminal tienen el teclado (el terminal conserva Tab; se sale con `sidebar.focus`); (b) Esc cierra el aviso visible más antiguo cuando el teclado está fuera de un terminal, diálogo o menú; (c) la terminal flotante se centra sobre el área de trabajo y no cubre la barra lateral; (d) la sección Bots de la barra lateral parte plegada y la de proyectos ocultos recuerda si la persona la abrió (`data/spaces/store.json`: `spaces.bots.open`, `spaces.hidden.open`); (e) con el tema win98 los paneles usan el fondo `surface` y los campos de Configuración `dialog::field_bg()` para ser legibles.

1. ¿La app actúa sobre esto? Sí: foco de teclado, cierre de avisos y posición de la ventana flotante cambian lo que hace la app. 2. ¿Solo la app lo ve? Sí: el foco y los eventos de teclado son de la ventana. 3. ¿Un mod no puede? No: un mod no ve el foco global ni puede reordenar la navegación con Tab.

Preguntas abiertas para Can: (a) mientras haya un aviso visible, Esc no llega a la barra lateral; ¿prefieres otra tecla para cerrar avisos? (b) Tab dentro de un terminal sigue siendo del terminal: ¿quieres una tecla fija para salir de él a los paneles (hoy `sidebar.focus`, `ctrl-shift-e` / `cmd-shift-e`)?

Límites: la tecla `cmd-alt-a` del flotante no llega a instalaciones con `keys.json` ya existente; no se verificó en Windows ni Linux.

## 32. Aviso: ajustes de núcleo de Orca y ⌘W (núcleo: `src/settings.rs`, `src/terminal.rs`, `src/vt.rs`, `src/window.rs`, `src/window/{pane_size,titlebar,dialog}.rs`, ajustes en `src/window/settings/`)

Decisión del dueño del fork. Campos nuevos opcionales en `settings.json` (sin ellos nada cambia): `cursor_shape`, `cursor_blink`, `cursor_opacity`, `inactive_pane_opacity`, `divider_width`, `focus_follows_mouse`, `hide_mouse_on_type`, `confirm_close_running`, `shell`, `middle_click_paste`, `trim_copy_margin`, `reduce_effects`, `show_app_name`, y `notifications.bell` / `notifications.suppress_when_focused`. Además, `vt.rs` engancha `GHOSTTY_TERMINAL_OPT_BELL` y `TerminalEvent::Bell` avisa a la ventana.

Cambio de comportamiento (compatible para scripts): `tab.close` de la última pestaña de un espacio, desde tecla o clic (⌘W), cierra solo la pestaña y deja el espacio con un terminal nuevo; desde `sunna do` sigue cerrando el espacio (pide `confirmed`). Cerrar el espacio: `space.close` o su menú. Sin cambio de teclas por defecto.

1. ¿La app actúa sobre esto? Sí: cambia qué se cierra, qué ejecuta un terminal nuevo, el foco y los avisos. 2. ¿Solo la app lo ve? Sí: el foco con el puntero, la campana y el cierre de pestañas pasan sin que ningún mod esté mirando. 3. ¿Los mods no pueden hacerlo bien? Correcto: no ven BEL, ni el puntero sobre un panel, ni pueden frenar `tab.close`.

| Límite | Por qué |
|---|---|
| Ícono en la bandeja / barra de menús al cerrar | GPUI no tiene NSStatusItem ni bandeja; exigiría una dependencia nueva |
| Aceleración por GPU | GPUI no expone un interruptor; el renderizador elige |
| Ocultar puntero al escribir en Linux y Windows | GPUI no tiene `CursorStyle::None`; en macOS se usa `NSCursor` vía `objc2` (ya dependencia) |
| Shell predeterminado en otras máquinas / del core | lo decide herdr-core; `shell` solo vale para terminales que lanza la app aquí |
| Ícono de la app | no entra en este ítem |

Verificado en pantalla (macOS, instancia aislada): cursor subrayado con opacidad, panel inactivo atenuado, divisor de 16 px, toast de campana, ⌘W real (espacio intacto, terminal nuevo) y con pregunta si algo corre, foco con el puntero, shell bash, filas de Configuración. Solo con prueba Rust: parpadeo, ocultar puntero, recortar margen, clic central. Sin verificar en Linux ni Windows.

## 33. Aviso: se retira el diálogo «Abrir carpeta…»; Agregar proyecto acepta varias carpetas (núcleo: `src/window.rs`, `src/window/{add_project,ctxmenu,spaces,palette,popmenus}.rs`, `src/commands.rs`)

Qué cambia: (a) `project.add {step: "browse"}` abre el selector del sistema con selección múltiple y abre un espacio por carpeta; `project.add {folders: [...]}` lo hace sin selector (también desde `sunna do`); (b) se borra `src/window/folders.rs` (búsqueda de carpetas): el botón de carpeta de la barra lateral, la paleta y el menú llaman a `project.add {step: "browse"}`, y `mod.open {name: "folders"}` abre ese mismo selector; (c) el ajuste `folders.roots` se lee sin error pero ya no tiene efecto, y se quita su fila de Configuración; (d) el menú Agregar proyecto lleva encabezado y sin línea de ayuda; (e) la selección de texto en campos de diálogo se calcula desde el tema para que el texto se lea (4.5:1).

1. ¿La app actúa sobre esto? Sí: cambia qué abre cada acceso y cuántos espacios crea. 2. ¿Solo la app lo ve? Sí: el selector del sistema es de la ventana. 3. ¿Un mod no puede? No: un mod no puede abrir el selector nativo con selección múltiple.

Límites: sin verificar en Windows ni Linux.
