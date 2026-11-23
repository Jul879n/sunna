# TODO

Pendientes de Sunna. Los defectos con detalle están en `Defectos.md`.

## En curso: polish con Orca (referencia: `stablyai/orca`, ver `CLAUDE.md`)

### Bloque 1, barra lateral de proyectos (plan aprobado)
- [x] `defaults/mods/menu/main.js`: pestaña nueva `project` con Nuevo worktree (solo repos), Abrir en el explorador (solo local), Copiar ruta y Ocultar proyecto. Reutiliza el popup de menú existente.
- [x] `defaults/mods/spaces/main.js`: clic derecho en la cabecera del proyecto abre ese menú. Los proyectos ocultos salen de la lista (nunca el que tiene el espacio activo) y quedan en una sección plegada «Proyectos ocultos» con Mostrar. Se guarda en `shared("space.hidden")`.
- [x] `defaults/locales/{en,es}.json`: claves nuevas (`menu.reveal_folder`, `menu.copy_path`, `menu.hide_project`, `spaces.hidden_projects`, `spaces.show`).
- [x] `scripts/spaces.test.ts`: ocultar, mostrar, que el proyecto activo no se oculte, y el menú del proyecto.
- [x] `defaults/AGENTS.md`: documentar el comportamiento.

### Barra superior fija (detenida a mitad; ver `BARRA-SUPERIOR.md`)
- [ ] Terminar, compilar, probar y documentar el cambio de layout (nada de esto está probado).

### Barra compacta e iconos de repo (hecho en código; sin reinstalar)
- [x] Icono del proyecto sacado del propio repo (`assets/icon.png`, `logo.svg`, `favicon`…), copiado a `project-icons/`.
- [x] Barra compacta: una celda por proyecto (icono, punto de agentes, menú con clic derecho), respeta ocultos, botón de búsqueda.
- [ ] Ver la barra expandida con icono de repo en pantalla (la compacta sí se vio).

### Bloques siguientes (cada uno con su plan antes de codear)
- [ ] Worktrees: lista y acciones por worktree (abrir, eliminar, estado de rama).
- [ ] Bots: panel y tarjetas según la UI de Orca.
- [ ] Ajustes de Mods: comparar con la gestión de Orca y cerrar diferencias.
- [ ] Configuración y menús laterales.

## Defectos abiertos (`Defectos.md`)
- [ ] 6. Botón Volver de la ficha de la galería: no se pudo reproducir; falta probarlo con clic real.
- [ ] 10. Panel Cambios dice «No es un repositorio git» en `~/Documents/...`.
- [ ] 11. Panel Archivos no ve los archivos de esa misma carpeta.
  - Sospecha: permiso de macOS sobre Documentos. La app instalada se firmó con el certificado Apple Development; `scripts/install-macos` acepta `SUNNA_SIGN_IDENTITY`.
  - Comprobar: Ajustes del Sistema › Privacidad y seguridad › Archivos y carpetas › Sunna › Documentos. Si sigue fallando, Cambios ahora muestra el error real de git.

## Verificación pendiente con la app real (de tu lado)
- [ ] Clic real en Instalar y Volver de la galería de Mods.
- [ ] Ícono de puzzle de las tarjetas y aviso «no compatible» en la ficha.
- [ ] Diálogo «Crear proyecto nuevo» y el `+` de worktree por proyecto, en pantalla.
- [ ] Release real en `Jul879n/sunna` y secreto `RELEASE_TOKEN`.
- [ ] Windows y el ícono del dock.

## Hecho en esta sesión
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
