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
