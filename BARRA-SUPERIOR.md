# Barra superior fija (en curso, detenida)

Referencia: Orca. Una barra que cruza toda la ventana (semáforos, marca, botón de barra lateral, pestañas, `+`, paneles) con la barra izquierda debajo. Plan aprobado por el usuario; **detenido a mitad por orden del usuario**. Nada de esto está compilado, probado ni instalado.

## Qué ya se cambió (sin probar)
- `defaults/layouts/main.json`: `root` pasó de fila a columna.
  - `topbar` (fila, 42 px) con `brand` (268 px) y `tabs` (el resto).
  - `body` (fila) con `sidebar` (sin `brand`) y `work` (sin `tabs`).
- `defaults/mods/workspace-chrome/main.js`:
  - `brandWidth()` y `toggleSidebar()` redimensionan `sidebar` (44 px en modo compacto) y `brand` (150 px en macOS, 100 en el resto).
  - La marca lleva borde inferior, deja sitio a los semáforos en macOS también en modo compacto, y siempre muestra búsqueda y el botón de barra lateral.
- `defaults/mods/tabs/main.js`: se quitó el margen de los semáforos (`lightsPadding`); ahora lo da la marca.
- `src/mods/render_tests.rs`: el test `tabs_leave_room_for_macos_traffic_lights` se reemplazó por `tabs_start_after_the_top_bar_brand_which_holds_the_window_buttons`.

## Falta
- [ ] Compilar y correr `cargo test --locked mods::render_tests` (la ejecución se detuvo).
- [ ] Revisar `scripts/settings.test.ts` (línea ~168 asume `sidebar` con hijos `brand` y `spaces`) y cualquier test que lea `layout["children"][0]`.
- [ ] Revisar el editor de barra lateral (`defaults/frame/settings/sidebar.js`) y `find()` en los mods: el árbol ahora tiene un nivel más.
- [ ] Probar en la app real (home aislado): semáforos de macOS, arrastrar la ventana desde la barra, alternar ancha/compacta, pantalla completa.
- [ ] La marca no sigue al arrastrar el borde de la barra lateral (solo al usar el botón).
- [ ] Documentar en `defaults/AGENTS.md`.
- [ ] Los layouts de usuario que clonaron `main` no se actualizan solos.

## Cómo revertir
`git checkout -- defaults/layouts/main.json defaults/mods/tabs/main.js` y deshacer a mano lo de `workspace-chrome/main.js` y `render_tests.rs` descrito arriba (esos archivos también tienen cambios anteriores de esta sesión).
