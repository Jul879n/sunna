# Defectos

Defectos encontrados usando la interfaz de Sunna. Cada uno con su estado. Los que se corrigen se
dejan aquí con la causa, para no volver a introducirlos.

| # | Defecto | Dónde | Estado |
|---|---|---|---|
| 1 | El botón **Instalar** de la ficha de un mod de la galería «no hacía nada»: cambiaba a la pestaña Instalados y, si el repo no era un mod, solo salía un aviso breve. | Ajustes › Mods › ficha | Corregido: la ficha se queda abierta, muestra «Instalando…», el error en rojo o la revisión en el mismo lugar. Test en `scripts/mods-section.test.ts`. |
| 2 | La galería lista repos del topic `herdr-mod` / `herdr-theme` que no son mods de Sunna (plugins en TOML, herramientas en Go), así que instalarlos siempre falla con «No se encontraron mods, temas ni layouts». | Ajustes › Mods › Explorar | Corregido: al abrir la ficha se consulta el árbol del repo en GitHub; sin `main.js`, `mods/*/main.js`, `themes/*.json` ni `layouts/*.json` se muestra «no compatible» y Instalar no hace nada. Test en `scripts/mods-section.test.ts`. |
| 3 | El panel **Bots** no se podía cerrar. | Panel Bots | Corregido: botón X en la cabecera (también en Búsqueda). |
| 4 | El `+` de Proyectos creaba un espacio directo; no ofrecía explorar, clonar ni crear un proyecto, y no había `+` de worktree por proyecto. | Barra lateral | Corregido: menú «Agregar proyecto» (mod `add-project`) y `+` de worktree por proyecto. |
| 5 | `tab.close falló: no tab to close` aparece como error cuando se pulsa cerrar pestaña sin ninguna abierta. | Cualquier pantalla sin espacios | Corregido: `Window::run` ignora ese error al cerrar pestaña o panel sin ninguno abierto; los scripts siguen recibiéndolo. |
| 6 | Botón **Volver** de la ficha: reportado como «no funciona»; no se pudo reproducir. | Ajustes › Mods › ficha | Sin reproducir: los tests lo cubren; falta confirmarlo con un clic real. |
| 7 | Las tarjetas de la galería muestran un recuadro gris hasta que se descargan la vista previa y el avatar. | Ajustes › Mods › Explorar | Corregido: ícono de puzzle mientras no hay imagen. |
| 8 | Tests sensibles a carga: el render de un mod se cortaba como `interrupted` (presupuesto de 50 ms) y `macos_check` agotaba su plazo con launchd real. | `cargo test` | Corregido: `SUNNA_JS_BUDGET_SCALE` en `.cargo/config.toml` y plazo de 60 s en el test. |
| 9 | macOS pide permiso de iCloud Drive al abrir una carpeta que vive en iCloud. | Explorar carpeta | Esperado del sistema; sin cambio. |
| 10 | El panel **Cambios** dice «No es un repositorio git» (0 archivos) en una carpeta que sí es repo (`~/Documents/work/cohasa/...`, con rama `feature/JA_STATUS_PILLS_ROUTES`). En un shell normal `git rev-parse` funciona ahí. | Panel Cambios | Causa sin confirmar (sospecha: permiso de macOS sobre Documents o PATH de la app). Mitigado: ya muestra el mensaje real de git en vez del genérico, para ver la causa en la próxima ocurrencia. Test en `scripts/changes.test.ts`. |
| 11 | El panel **Archivos** tampoco detecta los archivos de esa misma carpeta (`~/Documents/...`), igual que Cambios (defecto 10). | Panel Archivos | Causa sin confirmar; misma sospecha (permiso de macOS sobre Documentos para la app firmada ad hoc: cada reinstalación cambia su firma). No verificable desde aquí (la base TCC no es legible). Para comprobarlo: Ajustes del Sistema › Privacidad y seguridad › Archivos y carpetas › Sunna › Documentos. Mitigación: la app instalada se firmó con el certificado Apple Development (identidad estable) y `scripts/install-macos` acepta `SUNNA_SIGN_IDENTITY`; queda abierto hasta confirmarlo con la app real. |
