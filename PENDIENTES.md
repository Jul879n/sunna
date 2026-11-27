# Pendientes y objetivo (27-nov-2026)

## Objetivo vigente (/goal)

«pendientes implementados y verificados, con 0 errores utiliza /impecable y /ui-ux-pro-max»

Reglas de trabajo del usuario: un solo subagente a la vez (sonnet o haiku), sin git salvo orden expresa, español chileno formal, primera línea `ᵔᴗᵔ`. Antes de tocar UI o varios archivos: plan numerado y esperar aprobación. Sunna ya no es un fork: no se envía nada a Can ni al repositorio original; `docs/mensajes-para-can.md` y `docs/resumen-para-can.md` son registro interno.

## Estado al cierre (27-nov-2026)

- Compilar y probar con `RUSTUP_TOOLCHAIN=1.95.0` (con el toolchain por defecto 1.94.1 falla `vendor/gpui-pre`: `cold_path` inestable). `cargo test --locked`: 875 pruebas en verde; `cargo clippy --all-targets`: 0 avisos.
- Instalada en `/Applications/Sunna.app` con `just install` (27-nov).
- Evidencia: `/tmp/sunna-evidencia/pendientes/` (menú Agregar proyecto con encabezado; dos carpetas por `project.add {folders}` abren dos espacios).

## Hecho (27-nov)

- **A1–A5**: selección legible en campos (`dialog::field_selection` + prueba por tema), menú Agregar proyecto con encabezado y sin ayudas, varias carpetas (`project.add {folders}`, selector múltiple), `folders.rs` retirado (`mod.open folders` abre el selector; `folders.roots` sin efecto y sin fila en Configuración), documentado en `defaults/AGENTS.md`, `src/SKILL.md` y `docs/mensajes-para-can.md` (ítem 33).
- **E**: español formal (usted), `read_snapshot` eliminada, avisos de compilación y clippy en cero; `cmd-alt-a` ya migra desde el original `v0.1.16e` (archivos editados no se reemplazan).
- **B (parcial)**: cadenas de actualización, «Commit», agentes vacíos y selector de adjuntos a i18n; botones de la píldora de actualización, aviso de core, «Listo» del editor de layout y botones del editor a 32 px; tooltips en nombre de grupo y filas de Cambios.

## Verificado con clic real (agent-desktop, instancia aislada)
- «Nuevo grupo» con texto seleccionado legible en graphite, nord, paper (claro) y win98 (capturas `grupo-sel-*.png`).
- Menú «Agregar proyecto» en tema claro con encabezado e íconos; «Explorar carpeta…» abre el selector y crea el espacio con la carpeta elegida (`after-pick.png`).

## No hecho

### B. Auditoría: pendientes
- Pestañas a 29 px (tienen atajos); ✕, «+» y botones de panel ya a 32 px con foco.
- Agentes sin estado de carga/error (sus datos son síncronos).
- Submenús de `ctxmenu` y altura mínima del menú con lista: sin cambio (exigiría otro modelo de menús).
- Elegir 2 o más carpetas dentro del selector del sistema: la entrada sintética no aplica modificadores (⌘/⇧); verificado con 1 carpeta en el selector real y con `project.add {folders:[a,b]}` (abre 2 espacios). `multiple: true` está activo.
- Menú contextual por clic derecho en filas: no existe; el menú del proyecto se abre con «…».

### C. Verificaciones que dependen del usuario o de otras plataformas (#17)

- Windows y Linux (ningún cambio se probó allí).
- Contador con 2 o más máquinas conectadas.
- Indicador de arrastre a mitad de gesto y autodesplazamiento (`orca computer drag` es instantáneo).
- Abrir la app instalada y comprobar que no salen avisos de «mods editados»; que Bots parta plegado; ⌘W con una sola pestaña.
- Verificados solo con prueba Rust, sin captura: parpadeo del cursor, ocultar el puntero al escribir, recortar margen al copiar, clic central, suprimir avisos con la ventana en foco, nombre de la app en la barra de título.
- Sin menú de clic derecho (no existe): encabezado de grupo, cabecera de panel, fila de Cambios.
- Sin verificar con agentes reales: avisos, bots («Ejecutar ahora»).

### D. Límites de GPUI (documentados en el ítem 32)

Ícono en bandeja/barra de menús, interruptor de aceleración por GPU, ocultar el puntero en Linux y Windows. El shell predeterminado solo vale para terminales lanzadas por la app en este equipo.

## Hecho y verificado (resumen)

Fases 1 a 5 (editor, Configuración, barras, paneles, menús, grupos, linaje, arrastre, bots, limpieza, avisos, comandos rápidos, terminal flotante), diálogos unificados, ajustes del terminal de Orca y ⌘W con una pestaña, Bots plegado por defecto con estado recordado, defectos 10 y 11 dados por solucionados por el usuario.
