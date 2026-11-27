# TODO

Pendientes de Sunna. Los defectos con detalle están en `Defectos.md`.

## Plan vigente: app completa en Rust, mods de terceros en JS (ver `CLAUDE.md`)

Atajos de Orca por defecto (macOS, Linux, Windows), estado vacío estilo Orca y ajustes de Orca (inventario en `docs/orca-inventario.md`, lista corta en `docs/orca-configuracion.md`). Cada cambio de núcleo queda como ítem en `docs/mensajes-para-can.md` (1–17).

- [x] Fase 1: editor nativo de archivos (abrir, editar, ⌘S con control de conflicto, buscar/reemplazar, vista previa de markdown, `file.open`, soltar archivos) y «Guardar y cerrar».
- [x] Fase 1b: «Ir a archivo» (⌘P) y diálogo de confirmación nativos; mods `goto-file` y `confirm` retirados.
- [x] Fase 2: ventana de Configuración nativa (12 secciones); frame JS retirado. Sin verificar a mano: buscador de ajustes y Esc en la ventana nativa.
- [x] Fase 3: barras lateral y superior nativas (`spaces`, `tabs`, `brand`, `status`, `sidebar-footer`, `welcome`) con `Content::Native` (ítems 18 y 19 para Can); barra lateral se oculta por completo como Orca (sin riel); `just test` verde (cargo 632, bun 128). Sin verificar en pantalla: menú de clic derecho del proyecto, tarjeta de worktree con agentes, tooltip de estado, Windows/Linux. Menús emergentes (`menu`, `new-menu`, `agents`) siguen en JS hasta la fase 5.
- [x] Fase 4: paneles nativos (`files`, `changes`, `search`, `agents`, `activity`) con foco de teclado visible, filas de 32 px o más y contraste vía `native::tone`; mods JS, tests bun y entradas de `SHIPPED_MODS` retirados. Sin verificar: búsqueda en máquinas remotas y Windows, árbol de subagentes y foco de teclado (Tab/flechas) a ojo en Archivos y agentes, modo riel de agentes (descartado), reproducción de sonido al terminar un agente.
- [x] Fase 5: `worktree`, `bots`, menús; comandos rápidos, terminal flotante y limpieza automática de espacios, nativos. Alcance ampliado por el usuario («todo», Sunna = hijo de Orca con herdr): todo lo de `docs/orca-barra-lateral.md`, en tramos:
  - [x] 5a modelo y lista: agrupar (Ninguno/Estado/PR/Proyecto), ordenar (Nombre/Actividad/Reciente/Proyecto/Manual), orden de proyectos, filtros con insignia, sección Fijados, plegar todo, ir al espacio actual. No verificado: teclado del menú (flechas/Enter/búsqueda de proyecto) solo por código, sin pulsaciones reales; ordenar por Actividad de agentes y «ocultar en reposo/creados por bots» solo con pruebas puras (sin agentes reales); PR agrupa todo en «Sin PR» (requiere `gh`); sin tecla en macOS (⌘⌥P es el selector).
  - [x] 5b menú del worktree completo (leído/no leído, mover a estado, abrir en…, copiar, editar detalles, fijar, eliminar con diálogos) y menú de proyecto/grupo/host. No verificado: clic derecho real (sin `--restore-window`; se usó `sidebar.menu`), menú Alt Developer, menú de máquina y `space.open_in` con apps reales; sin dormir ni desconectar (ver mensajes-para-can 23).
  - [x] Diálogos (HECHO salvo lo no verificado: Enter recién abierta la bienvenida sin clic previo, clic real de worktree/prompt/folders/novedades/comentarios con ✕ y Esc en esta ronda, `help` sigue en JS) (pedido del usuario, propuesta A «Orca limpia» elegida: `/tmp/sunna-evidencia/dialogos/propuestas.png`): un solo componente nativo de diálogo (tarjeta ~420–480 px, radio 14, título + descripción atenuada, campos con borde completo y foco visible, pie con Cancelar y botón principal tintado a la derecha, botón ✕ de cierre visible de 32 px además de Esc) aplicado a confirmaciones, Editar detalles, Ir a archivo, paleta, selectores, Agregar proyecto/worktree/máquina, bienvenida/novedades, comentarios y prompt (los JS se portan a nativo).
  - [x] 5c grupos de proyectos, linaje padre/hijo, secciones por host, arrastrar para reordenar, cambiar icono de proyecto. Pasada de `impeccable` y `ui-ux-pro-max` hecha: botones ⋯ y de cabecera a 32 px (cabeceras de 36 px), guías de linaje dibujadas (trazo 1,5 px, contraste >=3:1 medido por test en paper, graphite y win98), Esc cancela el arrastre sin activar el clic, Alt+↑/↓ mueve entre hermanos (no pasa sobre su hijo), feedback de Refrescar con spinner. Verificado con entrada real de `orca computer`: Alt+↑, Esc, clic corto, soltar sobre cabecera de estado, plegar/eliminar grupo con diálogo, worktree hijo con `parent` guardado, cambio de icono. No verificado: indicador a mitad de arrastre y autodesplazamiento (`orca computer drag` es atómico; cubiertos por tests puros), clic derecho (se usó ⋯), contador con 2+ máquinas (requiere ssh real; `hc machine add` escribiría en `~/.ssh`). Ver mensajes-para-can 27.
  - [x] 5d-1 bots, limpieza automática de espacios y menús (`menu`, `new-menu`, `awake-menu`, `help`) nativos. No verificado: clic derecho real en pestañas, bot con agente real, Windows/Linux.
  - [x] 5d-2 pane, peek, toast, terminals-pane, comandos rápidos y terminal flotante nativos; `terminals-bar` y `win98` quedan como ejemplos de terceros (ver mensajes-para-can 30). No verificado: Windows/Linux, clic derecho, toasts con agentes reales, teclado de la cabecera de panel con lector de pantalla.
- [x] Ajustes de Orca que exigen núcleo (cursor del terminal, GPU, foco con ratón, shell predeterminado, etc.): esperan el visto bueno de Can (ítem 10).
- [x] ⌘W con una sola pestaña cierra el espacio (ítem 5 para Can).
- [x] Reinstalar en `/Applications` (hecho 26-nov; se repite al cerrar la tanda de pendientes del 27-nov) y comprobar que ya no salen avisos de «mods editados» (la instalación quedó a medias).
- [x] Defectos 10 y 11 (Cambios/Archivos en `~/Documents`): dados por solucionados por el usuario. El envío a Can se descartó: Sunna ya no es un fork.

## Hecho antes: polish con Orca (referencia: `stablyai/orca`, ver `CLAUDE.md`)

### Bloque 1, barra lateral de proyectos (plan aprobado)
- [x] `defaults/mods/menu/main.js`: pestaña nueva `project` con Nuevo worktree (solo repos), Abrir en el explorador (solo local), Copiar ruta y Ocultar proyecto. Reutiliza el popup de menú existente.
- [x] `defaults/mods/spaces/main.js`: clic derecho en la cabecera del proyecto abre ese menú. Los proyectos ocultos salen de la lista (nunca el que tiene el espacio activo) y quedan en una sección plegada «Proyectos ocultos» con Mostrar. Se guarda en `shared("space.hidden")`.
- [x] `defaults/locales/{en,es}.json`: claves nuevas (`menu.reveal_folder`, `menu.copy_path`, `menu.hide_project`, `spaces.hidden_projects`, `spaces.show`).
- [x] `scripts/spaces.test.ts`: ocultar, mostrar, que el proyecto activo no se oculte, y el menú del proyecto.
- [x] `defaults/AGENTS.md`: documentar el comportamiento.

### Barra superior fija (detenida a mitad; ver `BARRA-SUPERIOR.md`)
- [x] Layout `topbar` + `body`, marca que sigue el ancho de la barra lateral (`syncBrand`), UI sin líneas de borde a borde (separadores cortos con puntas), proyectos colapsables, `minimum_contrast` por defecto en tema claro. `just test` en verde (bun 208, cargo 508).
- [ ] Confirmar en pantalla con clic real: chevron de proyecto, botones del pie y del menú «Agregar proyecto», borde superior fijo (arrastre del borde de la barra lateral confirmado: la marca lo sigue) (arrastre de ventana desde la barra superior ya confirmado con `orca computer --restore-window`).

### Barra compacta e iconos de repo (hecho en código; sin reinstalar)
- [x] Icono del proyecto sacado del propio repo (`assets/icon.png`, `logo.svg`, `favicon`…), copiado a `project-icons/`.
- [x] Barra compacta: una celda por proyecto (icono, punto de agentes, menú con clic derecho), respeta ocultos, botón de búsqueda.
- [x] Ver la barra expandida con icono de repo en pantalla (vista en una instancia aislada).

### Bloques siguientes (cada uno con su plan antes de codear)
- [x] Worktrees: segunda línea en la fila (rama, ↑↓, punto de cambios sin confirmar), «Abrir worktree» en el menú del proyecto, botón eliminar por fila (nunca en el principal; también sin espacio abierto) y estado vacío con acción. Tests en `spaces.test.ts` y `worktree.test.ts`; visto en pantalla la fila. Clic real confirmado (orca computer): crear worktree, menú del proyecto y limpieza (desmarcar uno, doble clic, borra solo el marcado). Botón eliminar por fila y diálogo «¿Quitar worktree?» confirmados con clic real.
- [x] Bots: estado vacío con plantillas, tarjeta de plantilla clicable, programación y próxima/última ejecución, Pausar/Eliminar, motivo de «Ejecutar ahora» bloqueado y 8 ejecuciones recientes. Clic real confirmado (orca computer): Pausar/Reanudar, Eliminar con confirmación, plantilla y Agregar máquina. Sin éxito/fallo por ejecución (exigiría guardar el resultado).
- [x] Ajustes de Mods: buscador, filtro (Todos/Activos/Desactivados/Actualizaciones), contador, vacíos, chequeo automático de actualizaciones, marca «Instalado» en Explorar, incluidos al final. Pendiente: «Actualizar todo»; estado «error al cargar» exige el núcleo (consultar a Can).
- [x] Configuración y menús laterales: navegación agrupada, buscador de ajustes (↑/↓/Enter/Esc), filas en tarjeta con borde completo, Copiar ruta y Revelar carpeta en el menú de espacio. Pendiente (núcleo; texto listo en `docs/mensajes-para-can.md`): atajo global para enfocar el buscador.

## Defectos abiertos (`Defectos.md`)
- [x] 6. Botón Volver de la ficha de la galería: con clic real (orca computer) vuelve a la galería; no se reproduce el defecto.
- [x] 10. Panel Cambios dice «No es un repositorio git» en `~/Documents/...`. No se reproduce con la build actual (rama de prueba con espacios en la ruta: lista el cambio y el diff). El usuario tiene Acceso total al disco para Sunna; confirmado en la app instalada tras `just install` (24-nov 13:58: barra superior nueva, sin líneas de borde a borde).
- [x] 11. Panel Archivos no ve los archivos de esa misma carpeta. No se reproduce con la build actual (lista `a.txt` con su marca M).
  - Sospecha: permiso de macOS sobre Documentos. La app instalada se firmó con el certificado Apple Development; `scripts/install-macos` acepta `SUNNA_SIGN_IDENTITY`.
  - Comprobar: Ajustes del Sistema › Privacidad y seguridad › Archivos y carpetas › Sunna › Documentos. Si sigue fallando, Cambios ahora muestra el error real de git.

## Verificación pendiente con la app real (de tu lado)
- [ ] Clic real en Instalar y Volver de la galería de Mods.
- [ ] Ícono de puzzle de las tarjetas y aviso «no compatible» en la ficha.
- [ ] Diálogo «Crear proyecto nuevo» y el `+` de worktree por proyecto, en pantalla.
- [ ] Release real en `Jul879n/sunna` y secreto `RELEASE_TOKEN`.
- [ ] Windows y el ícono del dock.

## Hecho en esta sesión
- [x] #18 Ajustes de núcleo de Orca (cursor, opacidad inactiva, divisor, foco con ratón, ocultar puntero, confirmar cierre, shell, clic central, recortar margen, campana, silenciar lo que miras, reducir efectos, nombre en la barra) y ⌘W que cierra solo la pestaña. Límites: bandeja/barra de menús, GPU, ocultar puntero fuera de macOS. Pendiente: parpadeo del cursor y ocultar puntero solo con prueba Rust (no visto en captura), Windows/Linux sin verificar, ícono de la app.
- [x] Menú «Agregar proyecto» (mod `add-project`) y `+` de worktree por proyecto.
- [x] Botón Instalar de la ficha: se queda en la página y muestra progreso, error o revisión.
- [x] Galería: «no compatible» en repos sin mods, temas ni layouts.
- [x] Ícono de puzzle mientras carga la imagen de las tarjetas.
- [x] `tab.close` sin pestañas ya no muestra error (con test).
- [x] Panel Cambios muestra el error real de git.
- [x] `CLAUDE.md` con Orca como referencia y `Defectos.md`.

## Estado al detener (ordenado por el usuario)
- La app instalada en `/Applications` NO incluye: proyectos ocultos y menú del proyecto, iconos de repo, barra compacta por proyecto, ni la barra superior fija.
- Los tests de `just test` y clippy pasaron antes de empezar la barra superior; después de ese cambio no se ha vuelto a correr nada.
- Todo está sin commitear en la rama `feat/sunna-themes-mods-bots` (ya fusionada); no se ha hecho ninguna acción de git.
- Orca como referencia sigue en curso: worktrees, bots, ajustes de mods, configuración y menús laterales.

- [x] Panel de agentes y bandeja de actividad (mod `activity`: filtros por estado/agente/texto, contadores, registro de «terminó/te necesita»), automatizaciones (días de semana, precheck, periodo de gracia, ejecuciones omitidas/fallidas), «Actualizar todo» en Mods y limpieza de worktrees. Tests en `activity`, `bots`, `mods-section` y `worktree`. Visto en pantalla el panel Actividad vacío; limpieza de worktrees con clic real confirmada. Clic real: `orca computer` (el clic derecho y el arrastre de ventana exigen `--restore-window`); agent-desktop no activó los botones.
