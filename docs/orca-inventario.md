# Inventario de configuración de Orca frente a Sunna

Fuente: `stablyai/orca` (MIT), clon sparse en `/tmp/orca-src`: `src/renderer/src/components/settings/**`, `src/shared/settings`, `src/shared/keybindings`, `src/shared/orca-profiles`, `src/main/menu` y `src/cli`. Solo se replica la idea, no el código. Complementa a `docs/orca-configuracion.md` (la lista previa).

Comparación con Sunna verificada contra `defaults/frame/settings/*.js`, `src/settings.rs` (campos de `Person`), `defaults/AGENTS.md`, `defaults/keys.macos.json`, `src/macos.rs` y `defaults/mods/*`.

## Cómo leer las tablas

- **Defecto**: valor de `default-global-settings.ts`, `getDefaultUIState()` o el tipo correspondiente. Un guion (—) significa que el defecto no es un literal verificable en la parte del código obtenida (depende de la plataforma, de un argumento de arranque o vive en un archivo no incluido en el clon).
- **Sunna**: **Ya** = existe hoy; **Ya parcial** = existe una parte o un equivalente (se detalla); **Mod** = lo puede resolver un mod con el almacenamiento compartido, sin tocar el núcleo (criterio de `AGENTS.md`); **Núcleo** = hay que tocar el núcleo, y por tanto hablarlo antes con Can; **No aplica** = depende de algo que Sunna no tiene (chat, editor, navegador, tareas, cuentas de proveedor, móvil).
- La clasificación Mod/Núcleo es una estimación; se confirma al implementar cada ajuste.

## Resumen numérico

Ajustes inventariados en tablas por página: **452**. Más **103** comandos de atajos (catálogo) y los menús, la CLI, las variables de entorno y los archivos de configuración listados al final.

| Página | Ajustes | Ya | Ya parcial | Mod | Núcleo | No aplica |
|---|---|---|---|---|---|---|
| Appearance | 51 | 14 | 1 | 10 | 6 | 20 |
| Chat UI | 27 | 1 | 0 | 1 | 0 | 25 |
| Input & Editing | 24 | 1 | 0 | 5 | 1 | 17 |
| Notifications | 14 | 4 | 3 | 3 | 2 | 2 |
| Agents | 26 | 9 | 1 | 6 | 3 | 7 |
| AI Provider Accounts | 15 | 0 | 0 | 0 | 0 | 15 |
| Orchestration, Linear, Computer Use, Voice y Orca Account | 21 | 0 | 0 | 0 | 0 | 21 |
| Setup Guide | 3 | 0 | 1 | 2 | 0 | 0 |
| General | 32 | 14 | 2 | 10 | 1 | 5 |
| Integrations y Mobile | 13 | 1 | 0 | 0 | 0 | 12 |
| Remote, seguridad y avanzado | 23 | 9 | 2 | 2 | 2 | 8 |
| Projects (ajustes por repositorio) | 34 | 2 | 4 | 17 | 0 | 11 |
| Automations, Artifacts, Skills y Session search | 20 | 8 | 3 | 2 | 0 | 7 |
| Git & Source Control | 18 | 5 | 1 | 3 | 0 | 9 |
| Task Sources | 6 | 0 | 0 | 0 | 0 | 6 |
| Terminal | 48 | 22 | 6 | 1 | 15 | 4 |
| Quick Commands, Browser, Mobile emulator y Floating workspace | 26 | 0 | 1 | 5 | 0 | 20 |
| Bots (global y por bot) | 20 | 7 | 4 | 2 | 0 | 7 |
| Shortcuts (ajustes de la página) | 9 | 6 | 1 | 0 | 2 | 0 |
| Estado de interfaz fuera de Ajustes (barra lateral, filtros y vistas) | 22 | 6 | 2 | 9 | 0 | 5 |
| **Total** | **452** | 109 | 32 | 78 | 32 | 201 |
| Catálogo de atajos | 103 | 28 | 14 | 19 | 8 | 34 |

Orden de las páginas de Ajustes en Orca (navigation/*.ts): Appearance, Chat UI, Input & Editing (con Notifications), Shortcuts, Stats & Usage, Agents, AI Provider Accounts, Orchestration (con Linear, Computer Use, Voice y Orca Account), Onboarding checklist, General (con Workspace Cleanup), Integrations (con Mobile), SSH Hosts, Remote Orca servers (con macOS Permissions), Privacy & Telemetry, Advanced (con Dev Tools), Experimental, Plugins, un grupo por proyecto y una página por repositorio, Automations, Artifacts, Share Skills (con Shared links), Agent Session Search, Git & Source Control, Task Sources, Terminal, Quick Commands, Browser, Mobile Emulator, Floating Workspace y Bots (una página por bot). Stats & Usage no tiene ajustes propios (es un panel de lectura); sus opciones son las de uso de la barra de estado, ya listadas en Appearance.

## Appearance

Ruta: Settings > Appearance (secciones Interface, Terminal, Window & sidebar). Fuente: appearance/*.tsx, default-global-settings.ts, persisted-ui-state-types.ts.

| Ajuste | Qué hace | Control | Valores | Defecto | Plataforma | Sunna |
|---|---|---|---|---|---|---|
| Theme | Tema claro, oscuro o el del sistema. | Selector | system / dark / light | system | Todas | Ya · `appearance` |
| App theme (plugin) light/dark | Tema de la app aportado por un plugin, uno para claro y otro para oscuro. | Selector (2) | id de tema de plugin o ninguno | ninguno | Todas | Ya · temas en `themes/` y selector de tema |
| Editor theme (light/dark) | Colores del editor, diff y cuadernos, aparte del tema de la app. | Selector (2) | tema del editor o seguir la app | seguir la app | Todas | No aplica · Archivos es solo lectura |
| Icon theme | Tema de íconos de archivos aportado por un plugin. | Selector | id de tema de íconos o ninguno | ninguno | Todas | Mod |
| Language | Idioma de la interfaz. | Selector agrupado | sistema + lista de idiomas | sistema | Todas | Ya · `language` (es, en) |
| UI zoom | Escala toda la interfaz. | Control de zoom (−/+/reiniciar) | nivel; atajos de zoom | 0 (100 %) | Todas | Ya · `zoom` (0,5 a 3) |
| IDE font | Fuente de la interfaz de Orca. | Selector con búsqueda | familias instaladas | fuente del sistema | Todas | Ya · `ui_font` |
| Follow symlinked directories | Busca en carpetas enlazadas en Quick Open y las expande en el explorador. | Interruptor | true / false | false | Todas | Mod · Archivos |
| Show Git-ignored files | Muestra atenuados los archivos que ignora .gitignore en el explorador. | Interruptor | true / false | true | Todas | Ya · `files.ignored` |
| Titlebar app name | Muestra el nombre de la app en la barra de título. | Interruptor | true / false | true | Todas | Mod · el layout decide su barra superior |
| Tiling window manager mode | Oculta minimizar/maximizar y sube las pestañas al borde superior (gestores de ventanas en mosaico). | Interruptor | true / false | false | Linux | Núcleo |
| App icon | Ícono de la app en el Dock y el selector de ventanas. | Selector (+ importar imagen) | classic / watercolor / blue / custom | classic | macOS/Linux (Dock) | Núcleo |
| Reduce visual effects | Quita el desenfoque tras diálogos y menús y la translucidez de ventana (tras reinicio). Útil en escritorios remotos o sin GPU. | Interruptor | true / false | false | Todas | Núcleo |
| Window material | Qué se ve detrás de terminales translúcidos. Requiere reinicio. | Selector | off / translucent / blur | off | macOS/Windows (no verificado por SO) | Núcleo · hoy solo `terminal_opacity` del layout |
| Left sidebar appearance | Barra lateral por defecto, teñida o igual al terminal. | Selector | default / tinted (match-terminal en versiones antiguas) | default | Todas | Mod · colores del tema |
| Left sidebar tint color | Color del tinte de la barra lateral. | Color | hex | #18181b | Todas | Ya · `colors` |
| Left sidebar tint opacity | Intensidad del tinte. | Deslizador | 0 a 1 | 0,08 | Todas | Ya · `colors` con alfa |
| Workspace card layout (compact) | Tarjetas de espacio compactas o detalladas. | Selector | compact / detailed (`compactWorktreeCards`) | detailed | Todas | Ya · `space.card` |
| Workspace card properties | Qué filas muestra cada tarjeta: status, unread, pr, issue, linear-issue, jira-issue, automation, cli, comment, ports, inline-agents, host. | Casillas múltiples | subconjunto de 12 propiedades | todas salvo ci/branch; en modo compacto solo status | Todas | Ya parcial · `space.card` {git, ago, agents} |
| Also show pinned worktrees in their original lists | Un worktree fijado aparece en Pinned y también en All, Project, Status y PR. | Interruptor | true / false | false | Todas | Mod · Espacios |
| Show tasks button | Botón Tasks arriba en la barra lateral. | Interruptor | true / false | true | Todas | No aplica · Sunna no tiene Tareas |
| Show automations button | Botón Automations en la barra lateral. | Interruptor | true / false | true | Todas | Ya · `sidebar.buttons.bots` |
| Show Orca Mobile button | Botón del móvil en la barra lateral. | Interruptor | true / false | true | Todas | No aplica |
| Show Artifacts button | Atajo a Artifacts en la barra lateral. | Interruptor | true / false | false | Todas | No aplica |
| Show Skills button | Atajo a Skills en la barra lateral. | Interruptor | true / false | false | Todas | No aplica |
| Activity bar position | Barra de actividad arriba o a un costado. | Selector (UI oculta) | top / side | top | Todas | Mod · el layout lo define |
| Activity bar order / hidden | Orden y ocultamiento de los íconos de la barra de actividad (incluye orden de bots). | Lista arrastrable | ids | orden de fábrica | Todas | Ya · Layout > Right sidebar |
| Status bar visible | Muestra u oculta la barra de estado. | Interruptor (también menú Appearance) | true / false | true | Todas | Ya · `status.items` |
| Status bar item: Claude usage | Uso de tokens y costo de Claude. | Interruptor | true / false | true | Todas | No aplica |
| Status bar item: Codex usage | Uso de Codex. | Interruptor | true / false | true | Todas | No aplica |
| Status bar item: Gemini usage | Uso de Gemini. | Interruptor | true / false | true | Todas | No aplica |
| Status bar item: Antigravity usage | Suscripción de Antigravity. | Interruptor | true / false | true | Todas | No aplica |
| Status bar item: OpenCode Go usage | Uso de OpenCode Go. | Interruptor | true / false | true | Todas | No aplica |
| Status bar item: Kimi usage | Suscripción de Kimi. | Interruptor | true / false | true | Todas | No aplica |
| Status bar item: MiniMax usage | Suscripción de MiniMax. | Interruptor | true / false | true | Todas | No aplica |
| Status bar item: Grok usage | Créditos semanales de Grok. | Interruptor | true / false | true | Todas | No aplica |
| Status bar item: Cursor usage | Plan de Cursor. | Interruptor | true / false | true | Todas | No aplica |
| Status bar item: ZCode usage | Cuota del plan ZCode. | Interruptor | true / false | true | Todas | No aplica |
| Status bar item: Muse usage | Cuota de Muse. | Interruptor | true / false | true | Todas | No aplica |
| Status bar item: Remote hosts (ssh) | Estado de conexión de los hosts remotos. | Interruptor | true / false | true | Todas | Ya · `status.items.machine` |
| Status bar item: Resource manager | CPU, memoria, sesiones de terminal y disco de los espacios. | Interruptor | true / false | true | Todas | Mod · Estado |
| Status bar item: Ports | Puertos en uso de los espacios. | Interruptor | true / false | true | Todas | Mod · Estado |
| Status bar item: Git blame | Quién cambió por última vez la línea del cursor. | Interruptor | true / false | true | Todas | No aplica · sin editor |
| Usage format | Plantilla con marcadores para cada proveedor. | Texto con marcadores | plantilla | formato de fábrica | Todas | No aplica |
| Usage percentages | Mostrar porcentaje usado o restante. | Selector | used / remaining | used | Todas | No aplica |
| Status bar usage mode | Estilo verboso o compacto de los medidores. | Selector | verbose / compact | compact | Todas | No aplica |
| Minimize to tray on close | Al cerrar la ventana la app sigue en la bandeja. | Interruptor | true / false | false | Windows/Linux (bandeja) | Núcleo |
| Show menu bar icon | Atajo y indicador de actividad en la barra de menú. | Interruptor | true / false | true | macOS | Núcleo |
| Tab order (Ctrl+Tab) | Ctrl+Tab recorre por uso reciente o en orden de la barra. | Selector | mru / sequential | mru | Todas | Ya · `tabs.order` (recent / strip) |
| Show agent models | Muestra el modelo de cada agente en la lista de agentes. | Interruptor (UI oculta) | true / false | true | Todas | Mod · Agentes |
| Agent activity display mode | Detalle de la actividad del agente en las tarjetas. | Selector (UI oculta) | compact / full | — | Todas | Mod · Agentes |

## Chat UI

Ruta: Settings > Chat UI (chat nativo de Orca, alternativa al CLI en terminal). Sunna no tiene interfaz de chat: todo es No aplica, pero se lista para completar el inventario. Fuente: appearance/chat, chat/*.

| Ajuste | Qué hace | Control | Valores | Defecto | Plataforma | Sunna |
|---|---|---|---|---|---|---|
| Match terminal interface | El chat usa la fuente y colores del terminal. | Interruptor | true / false | — | Todas | No aplica |
| Contrast | Brillo del texto del chat frente al fondo. | Deslizador | rango | — | Todas | No aplica |
| Font family (chat) | Fuente de mensajes, actividad y caja de mensaje. | Selector | familias | — | Todas | No aplica |
| Text size | Tamaño del texto del chat (también con atajos de zoom). | Número/deslizador | px | — | Todas | No aplica |
| Code text size | Tamaño del código, comandos y salidas. | Número/deslizador | px | — | Todas | No aplica |
| Width | Ancho máximo de mensajes en paneles anchos. | Selector | Comfortable / Wide / Full | — | Todas | No aplica |
| Reset chat appearance | Restaura los valores anteriores. | Botón | — | — | Todas | No aplica |
| Use Chat UI by default (`agentDefaultSurface`) | Los agentes abren como chat o como CLI. | Selector | chat / terminal | terminal | Todas | No aplica |
| Default launch mode per agent | Chat o CLI para cada agente por separado. | Selector por agente | chat / terminal | — | Todas | No aplica |
| Permissions for new chats (`nativeChatPermissionMode`) | Permiso inicial de los chats nuevos. | Selector | ask / accept-edits / auto / full-access | full-access | Todas | No aplica |
| Resume working chats after a restart | Reanuda chats activos al reabrir. | Interruptor | true / false | true | Todas | Ya (equivalente) · `resume_agents` para terminales |
| Continue chats when a usage limit resets | Envía «Continue where you left off.» al reponerse el límite. | Interruptor | true / false | true | Todas | No aplica |
| Stop idle agents after | Detiene el agente de un chat inactivo para liberar memoria. | Selector de minutos | N minutos / Never | 30 min | Todas | Mod · Agentes |
| Queue follow-ups | Mensajes enviados mientras trabaja el agente esperan como tarjetas. | Interruptor | true / false | true | Todas | No aplica |
| Scroll to latest when sending | Al enviar salta al último mensaje. | Interruptor | true / false | true | Todas | No aplica |
| Mark new messages when returning | Línea «New» sobre lo llegado en tu ausencia. | Interruptor | true / false | — | Todas | No aplica |
| Link previews | Tarjeta de vista previa al pasar sobre un enlace (hace una petición). | Interruptor | true / false | true | Todas | No aplica |
| Spellcheck and autocorrect | Subrayado y reemplazos del sistema en la caja de mensaje. | Interruptor | true / false | — | Todas | No aplica |
| Suggested prompts | Tras la respuesta de Claude, sugiere tu próximo mensaje (Tab lo acepta; consume uso). | Interruptor | true / false | — | Todas | No aplica |
| Use your shell environment | Los chats heredan las variables de tu shell de login. | Interruptor | true / false | true | Todas | No aplica |
| Shell environment variables | Si se apaga lo anterior, cuáles variables pasan. | Lista de nombres | nombres | vacía | Todas | No aplica |
| Inline visuals | El agente muestra gráficos, diagramas y maquetas en sus respuestas. | Interruptor | true / false | true | Todas | No aplica |
| Chat names | El agente nombra el chat desde su primer mensaje (otro agente en Advanced). | Interruptor | true / false | — | Todas | No aplica |
| Turn diffs | Muestra bajo la respuesta los archivos que cambió cada turno. | Interruptor | true / false | true | Todas | No aplica |
| Opened pull requests | Muestra los PR abiertos en cada turno. | Interruptor | true / false | true | Todas | No aplica |
| CI result lines | Línea en el chat cuando termina el CI de un PR del agente. | Interruptor | true / false | — | Todas | No aplica |
| Claude subagent navigation | Navegar a los subagentes de Claude desde el chat. | Interruptor | true / false | true | Todas | No aplica |

## Input & Editing

Ruta: Settings > Input & Editing (teclado y editor integrado) y sección Editor de General. Fuente: input/*, editor/*. El panel Archivos de Sunna es de solo lectura, así que casi todo lo del editor es No aplica.

| Ajuste | Qué hace | Control | Valores | Defecto | Plataforma | Sunna |
|---|---|---|---|---|---|---|
| Middle-click paste from selection | Clic central pega la selección (en Linux usa el portapapeles de selección del sistema). | Interruptor | true / false | true en Linux y macOS (según plataforma) | Todas | Ya · mismo comportamiento fijo (AGENTS.md: Linux PRIMARY, Windows pega portapapeles) |
| Auto save | Guarda automáticamente tras editar. | Interruptor | true / false | false | Todas | No aplica |
| Auto save delay | Espera tras tu última edición antes de guardar. | Número (ms) | ms | valor de arranque de la plataforma | Todas | No aplica |
| Format on save | Formatea al guardar. | Interruptor | true / false | false | Todas | No aplica |
| Insert final newline | Añade salto de línea final al guardar. | Interruptor | true / false | false | Todas | No aplica |
| End of line | Fin de línea al guardar. | Selector | auto / (lf / crlf) | auto | Todas | No aplica |
| Editor font family | Fuente del editor y de diffs. | Selector con búsqueda | familias o vacío (sigue al terminal) | vacío | Todas | No aplica |
| Editor font size | Tamaño base del editor/diff. | Número | px; vacío sigue al terminal | vacío | Todas | No aplica |
| Editor font weight | Peso de fuente del editor. | Número | 100-900; vacío = 400 | vacío | Todas | No aplica |
| Editor line height | Interlineado: múltiplo, o píxeles si es 8 o más. | Número | múltiplo / px; 0 o vacío = auto | vacío | Todas | No aplica |
| Editor font ligatures | Ligaduras (=> !==) en editor y diff. | Interruptor | true / false | — | Todas | No aplica |
| Font smoothing | Suavizado del texto en terminales y editores. | Selector | Native u otros | — | macOS | Núcleo |
| Editor word wrap | Ajuste de línea en el editor. | Interruptor | true / false | true | Todas | No aplica |
| Minimap | Mapa de código al editar. | Interruptor | true / false | false | Todas | No aplica |
| Inline git blame | Autor y fecha de la línea del cursor. | Interruptor | true / false | false | Todas | No aplica |
| Default Markdown view | Vista inicial de archivos Markdown. | Selector | rich / source | rich | Todas | No aplica |
| Rich Markdown spellcheck | Subrayado ortográfico en Markdown enriquecido. | Interruptor | true / false | true | Todas | No aplica |
| Markdown review notes | Controles de notas de revisión en el editor enriquecido. | Interruptor | true / false | true | Todas | No aplica |
| Default diff view | Presentación de los diffs. | Selector | inline / side-by-side | inline | Todas | Mod · Cambios |
| Diff word wrap | Ajuste de línea en diffs. | Interruptor | true / false | false | Todas | Mod · Cambios |
| Diff show whitespace | Muestra diferencias de espacios iniciales y finales. | Interruptor | true / false | false | Todas | Mod · Cambios |
| Collapse unchanged regions | Solo líneas cambiadas con contexto, el resto plegado. | Interruptor | true / false | false | Todas | Mod · Cambios |
| Default diff file tree | Muestra el árbol de archivos al abrir diffs combinados. | Interruptor | true / false | false | Todas | Mod · Cambios |
| Editor formatter (por proyecto) | Comando e include (glob) del formateador de un repositorio. | Texto + glob | comando, `include` (por defecto `**/*.{ts,tsx,js,jsx,json,css,scss,html,md,yaml,yml}`) | ninguno | Todas | No aplica |

## Notifications

Ruta: Settings > Notifications (subpágina de Input & Editing). Fuente: notifications/*, notification-settings-defaults.ts.

| Ajuste | Qué hace | Control | Valores | Defecto | Plataforma | Sunna |
|---|---|---|---|---|---|---|
| Enable notifications | Interruptor maestro de notificaciones de escritorio. | Interruptor | true / false | true | Todas | Ya parcial · `sounds.enabled` (sonidos); los toasts no tienen maestro |
| Agent task complete | Avisa cuando un agente pasa de trabajar a inactivo. | Interruptor | true / false | true | Todas | Ya · Alerts «An agent finished» |
| Agent needs input | Avisa cuando un agente pide permiso o hace una pregunta. | Interruptor | true / false | true | Todas | Ya · Alerts «An agent needs you» |
| Terminal bell | Avisa cuando un terminal en segundo plano emite BEL. | Interruptor | true / false | false | Todas | Núcleo |
| Suppress while focused | No avisa si Orca está en foco sobre ese worktree. | Interruptor | true / false | true | Todas | Ya parcial · el aviso de «terminó» solo salta si no lo estás mirando; el toast se omite al mirarlo (AGENTS.md) |
| Notification sound | Sonido de las notificaciones. | Selector (+ archivo local) | system / none / two-tone / bong / thump / blip / sonar / blop / ding / clack / beep / custom | system | Todas | Ya parcial · `sounds.done` / `sounds.blocked` (MP3) |
| Custom sound path | Archivo de audio propio. | Ruta | archivo | ninguno | Todas | Ya · `sounds.done` y `sounds.blocked` |
| Notification volume | Volumen de sonidos que no son del sistema. | Deslizador | 0 a 100 | 100 | Todas | Núcleo |
| Machines (muted sources) | Qué máquinas (SSH o servidores emparejados) pueden notificar en este equipo. | Lista de casillas | ids de fuente | ninguna silenciada | Todas | Mod · Alertas |
| Notify for hidden workspaces | Silencia los espacios ocultos por los filtros de la barra lateral. Solo local. | Interruptor | true / false | false | Todas | Mod |
| Notifications muted per worktree | Silenciar un worktree concreto (menú contextual). | Acción por worktree | true / false | ninguno | Todas | Mod · Espacios |
| Disk almost full | Sugiere limpiar espacios si el disco se llena. | Interruptor | true / false | — | Todas | No aplica |
| Hidden browser tabs | Sugiere Memory Saver con muchas pestañas ocultas. | Interruptor | true / false | — | Todas | No aplica |
| Send test notification | Lanza una notificación de prueba por la vía nativa. | Botón | — | — | Todas | Ya · botones Play de Alerts |

## Agents

Ruta: Settings > Agents. Fuente: agents/*, default-global-settings.ts. Sunna: lista de agentes en `programs.json`, `yolo` y `resume_agents` en Ajustes > Agents.

| Ajuste | Qué hace | Control | Valores | Defecto | Plataforma | Sunna |
|---|---|---|---|---|---|---|
| Default agent (`defaultTuiAgent`) | Agente que abren los atajos y botones «nuevo agente». | Selector | agente instalado o ninguno | ninguno | Todas | Núcleo · hoy cada menú pide el agente |
| Enabled agents (`disabledTuiAgents`) | Qué agentes aparecen en menús y selectores; se pueden ocultar. | Lista de interruptores | ~40 agentes (claude, codex, gemini, opencode, grok, cursor, copilot, kimi…) | todos habilitados | Todas | Mod · `programs.json` + Ajustes > Agents |
| Command override (`agentCmdOverrides`) | Comando propio por agente (ruta o alias del binario). | Texto por agente | comando | vacío | Todas | Ya · `run` en `programs.json` |
| Default arguments (`agentDefaultArgs`) | Argumentos que se añaden al lanzar cada agente. | Texto por agente | argumentos | vacío | Todas | Ya · `run` en `programs.json` |
| Default environment (`agentDefaultEnv`) | Variables de entorno por agente. | Pares clave=valor por agente | mapa | vacío | Todas | Núcleo |
| Launch choices (`agentLaunchChoices`) | Modelo y opciones recordados por agente. | Selectores por agente | modelo, esfuerzo… | vacío | Todas | Núcleo |
| Open agents as | Predeterminado de este equipo: chat o CLI. | Selector | chat / CLI | CLI (terminal) | Todas | No aplica |
| Agent runtime | Detectar y lanzar agentes en Windows o en WSL. | Selector | auto / host / WSL | auto | Windows | No aplica |
| Agent permissions (`agentPermissionDefault`) | Permiso de todos los agentes: aprobar por mí, acceso total o el propio de cada agente. | Selector | own / ask / accept-edits / auto / full-access | full-access | Todas | Ya parcial · `yolo.enabled` |
| Per-agent permission (`agentPermissionOverrides`) | Permiso propio de un agente. | Selector por agente | mismos valores | ninguno | Todas | Ya · `yolo.agents` |
| Agent status hooks (`agentStatusHooksEnabled`) | Instala ganchos para saber el estado de cada agente (working, blocked…). | Interruptor | true / false | true | Todas | Ya · estado de agentes del núcleo |
| Agent workspace trust (`agentWorkspaceTrustEnabled`) | Pre-confía los worktrees para los agentes. | Interruptor (+ decisión por proyecto) | true / false; pre-trust / agent-asks | true | Todas | No aplica |
| Claude agent teams (`claudeAgentTeamsMode`) | Equipos de agentes de Claude: apagado, en proceso o paneles nativos con shim de tmux. | Selector | off / in-process / native-panes-shim | off | Todas | No aplica |
| Claude subagent navigation | Navegar a los subagentes de Claude. | Interruptor | true / false | true | Todas | Ya · subagentes bajo su padre en `sunna ls` |
| Codex terminal server isolation | Cada terminal Codex usa su propio servidor. | Interruptor | true / false | true | Todas | Ya · Codex arranca con `--no-daemon` |
| Codex shared server warning | Aviso al compartir servidor Codex. | Interruptor | true / false | true | Todas | No aplica |
| Prompt cache timer | Cuenta regresiva hasta que vence el caché de prompts (Claude). | Interruptor | true / false | false | Todas | No aplica |
| Prompt cache TTL | Duración del caché que usa el temporizador. | Número (ms) | ms | 300 000 | Todas | No aplica |
| Delete agent history when a workspace is deleted | Borra los chats de Orca, Claude Code y Codex de espacios eliminados (no se recuperan). | Interruptor | true / false | false | Todas | Mod · Worktree |
| Resume agents after restart | Reanuda agentes tras reinicio de la app. | Interruptor | true / false | true (`nativeChatResumeWorkOnRestart`) | Todas | Ya · `resume_agents` |
| Tab auto-generate title (`tabAutoGenerateTitle`) | Título automático de pestañas con el agente. | Interruptor | true / false | false | Todas | Mod · Pestañas |
| Keep computer awake while agents run (`keepComputerAwakeWhileAgentsRun`) | Evita la suspensión mientras trabajan agentes (modos on / off / auto). | Selector | on / off / auto | off | macOS/Linux/Windows | Ya · `keep_awake` (off / auto / on); no en Windows |
| Agent sleep (experimental) | Detiene terminales de agentes inactivos y los reanuda al abrirlos. | Interruptor | true / false | false | Todas | Mod · Agentes |
| Agent sleep idle window (`agentHibernationIdleMs`) | Tiempo de inactividad antes de dormir. | Número (ms) | ms | 1 800 000 (30 min) | Todas | Mod · Agentes |
| New agent tab per agent (atajo) | Un atajo de teclado por cada agente («New X tab»). | Atajo | uno por agente | sin asignar | Todas | Mod · comando `tab.new` con `argv` |
| Enable agent CLI installation help | Avisos de agente no instalado (problema de instalación). | Aviso | — | — | Todas | Ya · «Checking installed agents…» |

## AI Provider Accounts

Ruta: Settings > AI Provider Accounts. Cuentas, cuotas y uso por proveedor. Sunna no administra cuentas de proveedores (cada agente usa su propia sesión).

| Ajuste | Qué hace | Control | Valores | Defecto | Plataforma | Sunna |
|---|---|---|---|---|---|---|
| Account location | Dónde se inspeccionan y añaden las cuentas: este equipo o WSL. | Selector | auto / host / WSL (+ distro) | auto | Windows | No aplica |
| Codex accounts | Cambio opcional de cuenta de Codex y límites en vivo. | Lista + botón añadir | cuentas gestionadas | vacía | Todas | No aplica |
| Active Codex account | Cuenta guardada que alimenta las lecturas de cuota. | Selector | cuenta / ninguna | ninguna | Todas | No aplica |
| Share Codex history across accounts | Historial de Codex compartido entre cuentas. | Interruptor | true / false | true | Todas | No aplica |
| Claude accounts | Cambio opcional de cuenta de Claude conservando el contexto. | Lista + botón añadir | cuentas gestionadas | vacía | Todas | No aplica |
| Claude OAuth usage | Lee el uso con el OAuth de Claude. | Interruptor | true / false | true | Todas | No aplica |
| When you switch accounts while Claude is running | Reiniciar terminales continúa la conversación en la nueva cuenta. | Interruptor | true / false | — | Todas | No aplica |
| Use Gemini CLI credentials | Extrae credenciales OAuth del Gemini CLI local. | Interruptor | true / false | false | Todas | No aplica |
| OpenCode Go session cookie / workspace id | Cookie de sesión de opencode.ai y workspace opcional para leer límites. | Texto secreto + texto | cabecera Cookie; id | vacío | Todas | No aplica |
| MiniMax usage | Endpoint, modelos de uso y clave para MiniMax. | Selectores + secreto | overseas / china; general / … | overseas / general | Todas | No aplica |
| Grok (xAI) usage | Inicio de sesión OAuth con `grok login` para créditos semanales. | Botón | — | — | Todas | No aplica |
| Antigravity accounts | Cuentas Google nativas de agy en el host; elección de inicio de sesión (auto / agy / orca). | Lista + selector | auto / agy / orca | auto | Todas | No aplica |
| Antigravity managed agy / server auto update | Gestión y actualización automática del servidor ACP de Antigravity. | Interruptores | true / false | true / true | Todas | No aplica |
| Cursor usage | Lee el plan desde `cursor-agent login`. | Botón | — | — | Todas | No aplica |
| GLM coding plan (ZCode) | Sitio (Z.AI o Zhipu) y clave del plan. | Selector + secreto | zai / zhipu | zai | Todas | No aplica |

## Orchestration, Linear, Computer Use, Voice y Orca Account

Rutas: Settings > Orchestration (y subpáginas Linear, Computer Use, Voice, Orca Account). Fuente: orchestration/*, computer-use/*, voice/*, accounts/orca/*.

| Ajuste | Qué hace | Control | Valores | Defecto | Plataforma | Sunna |
|---|---|---|---|---|---|---|
| Agent orchestration | Coordina varios agentes por mensajería, DAG de tareas, despacho y compuertas de decisión. | Interruptor + páginas | true / false | — | Todas | No aplica |
| Nested worker max depth | Profundidad máxima de workers anidados. | Número | entero | 4 | Todas | No aplica |
| Worker auto-release | Libera automáticamente a los workers al terminar. | Interruptor | true / false | true | Todas | No aplica |
| Coordinator approvals | El coordinador aprueba acciones de sus workers. | Interruptor | true / false | false | Todas | No aplica |
| Orchestration panel (experimental) | Panel de orquestación. | Interruptor | true / false | false | Todas | No aplica |
| Linear skill | Estado y ejemplos de la skill de Linear y enlace a Task Sources. | Info + botón instalar | — | — | Todas | No aplica |
| Computer use | Permite a agentes inspeccionar capturas y operar apps locales cuando lo pides. | Interruptor/permisos | true / false | — | macOS (permisos de accesibilidad) | No aplica |
| Dictation | Interruptor maestro del dictado por voz. | Interruptor | true / false | — | Todas | No aplica |
| Dictation shortcut / mode | Tecla que inicia el dictado; modo alternar o mantener pulsado (hold) o automático. | Atajo + selector | toggle / hold / auto | auto | Todas | No aplica |
| Microphone | Dispositivo de entrada del dictado. | Selector | dispositivo / predeterminado | predeterminado | Todas | No aplica |
| Dictation language | Idioma que habla el usuario. | Selector | auto o código ISO-639-1 | auto | Todas | No aplica |
| Speech model | Modelo de voz local o en la nube (sherpa-onnx, whisper, OpenAI…). | Selector + descarga | decenas de modelos | recomendado | Todas | No aplica |
| OpenAI transcription / OpenAI-compatible | Clave de OpenAI o servidor compatible (URL, modelo, clave). | Texto secreto + texto | baseUrl, model, apiKey | vacío | Todas | No aplica |
| Vocabulary | Términos propios (global y por repo) que el dictado escribe a tu manera. | Lista editable | términos + sugerencia | vacía | Todas | No aplica |
| Remove filler words | Quita «um», «uh» y equivalentes. | Interruptor | true / false | — | Todas | No aplica |
| AI cleanup | Un agente corrige puntuación y números antes de insertar el texto; instrucciones propias y palabra de corrección («scratch that»). | Interruptor + texto | true / false | false | Todas | No aplica |
| Stop after silence | Cuánto escucha el dictado en silencio. | Selector | segundos / nunca | 60 s | Todas | No aplica |
| Keep model loaded | Cuándo se descarga de memoria el modelo local. | Selector | minutos / nunca / tras cada dictado | — | Todas | No aplica |
| Use GPU | Apagar si el dictado congela el equipo. | Interruptor | auto / off | auto | Todas | No aplica |
| Terminal confirm before insert | Confirmar antes de insertar el dictado en un terminal. | Interruptor | true / false | — | Todas | No aplica |
| Orca account | Inicio y cierre de sesión de la cuenta usada por Artifacts y Orca Relay. | Botón | — | — | Todas | No aplica |

## Setup Guide

Ruta: Settings > Onboarding checklist. Sunna tiene pantalla de bienvenida (mod `welcome`) y `app.welcome`.

| Ajuste | Qué hace | Control | Valores | Defecto | Plataforma | Sunna |
|---|---|---|---|---|---|---|
| Onboarding checklist progress | Pasos de arranque (agente, repo, GitHub, navegador…) y avance. | Lista de pasos | — | — | Todas | Ya parcial · mod `welcome`, `app.welcome` |
| Setup guide sidebar dismissed | Oculta la guía de la barra lateral. | Interruptor (oculto) | true / false | false | Todas | Mod |
| Feature tips / contextual tours | Consejos y recorridos que se muestran una vez. | Estado interno | ids vistos | vacío | Todas | Mod |

## General

Ruta: Settings > General (con subpágina Workspace Cleanup). Fuente: general/*, repository/workspace-cleanup, default-global-settings.ts.

| Ajuste | Qué hace | Control | Valores | Defecto | Plataforma | Sunna |
|---|---|---|---|---|---|---|
| Workspace directory | Carpeta raíz donde se crean los espacios. | Ruta (selector de carpeta) | ruta | carpeta de espacios de Orca | Todas | Ya · `worktrees.directory` |
| Nest workspaces / Folder per project | Crea los espacios dentro de una subcarpeta con el nombre del repo. | Interruptor | true / false | true | Todas | Ya · `worktree.nest` |
| External worktrees | Mostrar o no los worktrees creados fuera de Orca (global; también por proyecto y por fuente: Claude, GSD, rutas propias). | Selector | hide / show | hide | Todas | Ya · `worktree.external` (solo global) |
| Custom worktree visibility sources | Rutas raíz propias clasificadas aparte (p. ej. scratch de agentes). | Lista de rutas + selector | hide / show por fuente | vacía | Todas | Mod · Worktree |
| Open in apps | Apps disponibles en el menú «Open in» de un espacio. | Lista editable | apps (Finder, VS Code, Cursor…) | lista de fábrica | Todas | Ya · `menu.open_in` |
| Tab order | Pestañas por uso reciente o por posición. | Selector | recent / tab strip | recent | Todas | Ya · `tabs.order` |
| Preview tabs | Un clic en un archivo reutiliza una pestaña de vista previa. | Interruptor | true / false | true | Todas | Ya · `files.open` (click / double) |
| Confirm close pinned tab | Pide confirmación al cerrar una pestaña fijada. | Interruptor | true / false | true | Todas | Mod |
| Confirm: closing a running chat or terminal | Pregunta antes de detener un agente o comando en ejecución (`skipCloseTerminalWithRunningProcessConfirm`). | Interruptor | true / false | pregunta (skip = false) | Todas | Ya parcial · `confirmed` al cerrar espacio/pestaña |
| Confirm: deleting a workspace | Pregunta al eliminar un espacio (`skipDeleteWorktreeConfirm`). | Interruptor | true / false | pregunta | Todas | Mod · Worktree |
| Always force delete worktrees / preserved branches | Elimina con fuerza sin preguntar. | Interruptores (2) | true / false | false / false | Todas | Mod · Worktree |
| Confirm: deleting an automation / chat / artifact / Codex reset | Confirmaciones de borrado (cada una descartable). | Interruptores | true / false | pregunta | Todas | Ya parcial · Bots pide segundo clic para eliminar |
| {command} in other terminals | Registra o quita el comando `orca` en el PATH. | Botón | — | — | macOS/Linux/Windows | Ya · el instalador crea `~/.local/bin/sunna` en macOS |
| Orca skill for agents | Instala la skill para que los agentes usen la CLI de Orca. | Botón | — | — | Todas | Ya · `sunna --skill` |
| Check for updates | Busca e instala nuevas versiones. | Botón | — | — | Todas | Ya · Settings > Updates |
| Automatic updates | Descarga y reinicia solo para instalar. | Interruptor | true / false | — | escritorio instalado | Ya · actualización automática de la app instalada |
| Release channel | Canal de actualizaciones. | Selector | stable / rc / hourly / daily / adhoc | stable | Todas | No aplica · una sola vía + `updates().fork` |
| Update delay | Días de espera tras publicar una versión antes de ofrecerla. | Selector | 0 / 1 / 3 / 7 / 14 / 30 días (máx. 90) | 0 | Todas | Núcleo |
| About | Versión de la app. | Información | — | — | Todas | Ya · Settings > Updates |
| Star Orca on GitHub | Da una estrella con `gh`; recordatorio por umbrales (starNag*). | Botón | — | — | Todas | No aplica |
| Default project runtime | Runtime heredado por proyectos locales de Windows. | Selector | windows-host / WSL (distro) | windows-host | Windows | No aplica |
| Machine name | Nombre con el que otros equipos listan este computador. | Texto | texto | vacío (nombre del host) | Todas | Ya · renombrar máquinas en Machines |
| Default task source / view preset | Fuente (github / gitlab / linear / jira) y vista predeterminada de Tasks. | Selectores | github…; all… | github / all | Todas | No aplica |
| Workspace Cleanup · Review workspaces | Lista todos los espacios y deja elegir cuáles borrar (filtros guardables). | Pantalla con filtros | — | — | Todas | Ya · Worktree > cleanup (merged, sin cambios, sin agente) |
| Workspace Cleanup · Automatic cleanup | Si la limpieza automática corre, cuándo es la próxima y ejecutar ahora. | Interruptor + botón | true / false | off | Todas | Mod · Worktree |
| Delete merged workspaces | Borra espacios cuyo PR está fusionado y todo está dentro. | Interruptor | true / false | false | Todas | Ya · selección «merged» al limpiar (manual) |
| Delete unchanged workspaces | Sin commits sobre la rama por defecto ni ediciones. | Interruptor + días | true / false; días | false; 1 día | Todas | Mod · Worktree |
| Archive/Delete inactive workspaces | Sin actividad durante N días: archivar o borrar. | Interruptor + días + acción | true / false; días; archive / delete | false; 30; archive | Todas | Mod · Worktree |
| Delete finished automation workspaces | Espacios creados por una automatización ya terminada. | Interruptor | true / false | false | Todas | Mod · Bots |
| Delete released orchestration worker workspaces | Espacios de workers ya liberados. | Interruptor | true / false | false | Todas | No aplica |
| Keep workspaces with ignored files | Conserva los que tengan .env u otros archivos ignorados (any-local-files). | Selector | any-local-files / … | any-local-files | Todas | Mod · Worktree |
| Cleanup activity log | Qué quitó y qué conservó cada corrida. | Registro | — | — | Todas | Mod · Worktree |

## Integrations y Mobile

Rutas: Settings > Integrations, y subpágina Mobile. Fuente: integrations/*, mobile/*.

| Ajuste | Qué hace | Control | Valores | Defecto | Plataforma | Sunna |
|---|---|---|---|---|---|---|
| GitHub integration | Autenticación de GitHub con la CLI `gh`. | Estado + botón | — | — | Todas | Ya · el panel Cambios y los bots usan `gh` ya autenticado |
| GitLab integration | Autenticación con `glab`. | Estado + botón | — | — | Todas | No aplica |
| Bitbucket integration | API token o variables de entorno. | Secreto | token | — | Todas | No aplica |
| Azure DevOps integration | Tokens por variables de entorno. | Estado | — | — | Todas | No aplica |
| Gitea integration | Token por variables de entorno. | Estado | — | — | Todas | No aplica |
| Jira integration | Jira Cloud: conectar o actualizar token. | Formulario | sitio, correo, token | — | Todas | No aplica |
| Linear integration | Conectar Linear para ver y enlazar issues. | Botón OAuth | — | — | Todas | No aplica |
| Mobile · Pairing | Empareja un teléfono con QR. | QR + botón | — | — | Todas | No aplica |
| Mobile · Connected devices | Gestiona dispositivos emparejados. | Lista | — | — | Todas | No aplica |
| Mobile · Network interface | Dirección de red para emparejar. | Selector | interfaz / dirección propia | automática (`mobilePairingConnectionMode`) | Todas | No aplica |
| Mobile · When you leave the app | Qué pasa con los terminales que veías al cerrar la app móvil. | Selector | — | — | Todas | No aplica |
| Mobile fit hold / auto-restore (`mobileAutoRestoreFitMs`) | Cuánto se mantiene el ajuste de tamaño del terminal. | Selector (ms) | null = indefinido | null | Todas | No aplica |
| Experimental mobile | Activa funciones móviles. | Interruptor | true / false | false | Todas | No aplica |

## Remote, seguridad y avanzado

Rutas: SSH Hosts, Remote Orca servers, macOS Permissions, Privacy & Telemetry, Advanced, Dev Tools, Experimental, Plugins. Fuente: ssh/*, runtime-environments/*, developer-permissions/*, privacy/*, advanced/*, plugins/*.

| Ajuste | Qué hace | Control | Valores | Defecto | Plataforma | Sunna |
|---|---|---|---|---|---|---|
| SSH connections · Add SSH target | Añade un host SSH (alias, host, usuario, puerto, llave). | Formulario | — | — | Todas | Ya · Machines > Add a machine |
| Import from SSH config | Importa hosts de ~/.ssh/config. | Botón | — | — | Todas | Ya · «Add machine lists all named ssh-config hosts» |
| Test connection | Prueba la conectividad con un destino. | Botón | — | — | Todas | Ya |
| SSH remote CLI control | Permite controlar Orca por la CLI desde hosts SSH existentes. | Interruptor | true / false | — | Todas | No aplica |
| Remote Orca servers | Agrega un servidor Orca remoto (código de emparejamiento), comparte este equipo y elige dónde abren los proyectos nuevos (`activeRuntimeEnvironmentId`). | Formulario + selector | — | null (escritorio local) | Todas | Ya parcial · Machines (herdr-core remoto) |
| WSL distros as hosts (experimental) | Añade una distro WSL 2 como host. | Interruptor | true / false | false | Windows | No aplica |
| Cloud VM (experimental) | Recetas por repo con entorno desechable por espacio. | Interruptor | true / false | false | Todas | No aplica |
| macOS developer permissions | Micrófono y cámara; grabación de pantalla y accesibilidad; Full Disk Access; LAN, USB y Bluetooth. | Botones por permiso | — | — | macOS | No aplica |
| Share anonymous usage data | Envía eventos anónimos de uso. | Interruptor | true / false | true (nuevos usuarios) | Todas | Ya · `telemetry` |
| Diagnostics | Controles de diagnósticos y soporte. | Botones | — | — | Todas | Ya parcial · `app.feedback` |
| Telemetry environment variables | Variables que desactivan la telemetría: `DO_NOT_TRACK`, `ORCA_TELEMETRY_DISABLED`, `ORCA_DIAGNOSTICS_DISABLED`. | Información | — | — | Todas | Ya · `DO_NOT_TRACK`, `HERDR_TELEMETRY=0` |
| Network proxy (`httpProxyUrl`, `httpProxyBypassRules`) | Proxy para peticiones de Orca y los hijos del terminal local; reglas de exclusión. | Texto (2) | URL; reglas | vacíos | Todas | Núcleo |
| Node dist mirror (`nodeDistMirrorUrl`) | Espejo para descargar Node. | Texto | URL | vacío | Todas | No aplica |
| HTTP/1.1 compatibility | Usa HTTP/1.1 si HTTP/2 falla tras un proxy. | Interruptor | true / false | false | Todas | No aplica |
| Uninstall Orca | Quita ganchos de agentes, el comando `orca`, el servidor `orca serve` y ganchos SSH. | Botón | — | — | Todas | Núcleo |
| Pet (experimental) | Mascota animada flotante abajo a la derecha (tamaño, personalizada). | Interruptor + selector + deslizador | true / false; id; tamaño | false | Todas | Mod (opcional) |
| Agent dashboard (experimental) | Tablero kanban de agentes por worktree, en ventana o emergente. | Interruptor | true / false (`experimentalActivity`) | false | Todas | Ya · mod `activity` |
| Terminal attention (experimental) | Resaltado persistente del panel con campana o fin de agente. | Interruptor | true / false | false | Todas | Mod · Actividad |
| Shared Codex servers (experimental) | Chats Codex con igual configuración comparten un proceso. | Interruptor | true / false | false | Todas | No aplica |
| New worktree card style (experimental) | Estilo de tarjetas de espacio. | Interruptor | true / false | true | Todas | Ya · `space.card` |
| Dev Tools · Orca Cloud / toasts de prueba | Estado de nube, cerrar sesión, probar toasts (info, éxito, error, acción nativa, borrado fallido, rama atrasada). Solo desarrollo. | Botones | — | — | solo dev | No aplica |
| Plugins · enable plugin system (`pluginSystemEnabled`) | Interruptor general de plugins (Alpha). | Interruptor | true / false | true | Todas | Ya · Settings > Mods |
| Plugins · disabled plugins, consents, catalogs, dev paths | Habilitar o deshabilitar plugin, consentimientos de acceso, catálogos (add/remove, requiere aprobar), rutas de desarrollo, política de actualización y sugerencias por tipo de archivo. | Lista + botones | — | vacío | Todas | Ya · mods: instalar, actualizar, desactivar, Explore |

## Projects (ajustes por repositorio)

Ruta: Settings > (grupo de proyectos) > cada proyecto. Fuente: repository/*, repo-types.ts (tipo `Repo`). Sunna guarda todo en ajustes globales; no hay ajustes por proyecto.

| Ajuste | Qué hace | Control | Valores | Defecto | Plataforma | Sunna |
|---|---|---|---|---|---|---|
| Display name | Nombre del proyecto en barra lateral y pestañas. | Texto | texto | nombre de la carpeta | Todas | Mod · Espacios (nombre del espacio) |
| Project icon and color | Ícono (`repoIcon`) y color de insignia (`badgeColor`) del proyecto. | Selector de ícono + color | emoji/imagen; hex | color automático | Todas | Ya parcial · íconos de repositorio (commit df0c067); el color sale del tema |
| Hide from sidebar | Oculta la copia de este host del proyecto de barra, selectores y cambiador; todo sigue corriendo. | Interruptor | true / false (`hiddenAt`) | false | Todas | Ya · menú de proyecto «Hide project» |
| Remove project | Quita el proyecto de Orca. | Botón | — | — | Todas | Ya parcial · no hay «quitar proyecto»; se cierran sus espacios o se oculta |
| Default worktree base (`worktreeBaseRef`) | Rama o ref base al crear worktrees. | Texto/selector de refs | ref | rama por defecto del remoto | Todas | Ya parcial · `worktree.fresh` |
| Worktree location (`worktreeBasePath`) | Directorio de nuevos worktrees para este proyecto (relativo a su ruta). | Ruta | ruta | ajuste global | Todas | Mod · Worktree (solo global hoy) |
| External worktrees (por proyecto) | Sobrescribe si aparecen los worktrees creados fuera de Orca. | Selector | hide / show | hereda el global | Todas | Mod · Worktree |
| Agent worktree visibility (`agentWorktreeVisibility`) | Política para worktrees scratch de agentes. | Selector | hide / show | hide | Todas | Mod · Worktree |
| Sparse checkout presets | Conjuntos de directorios guardados para crear worktrees parciales. | Lista editable | nombre + rutas | vacía | Todas | Mod · Worktree |
| Worktree shared paths (`symlinkPaths`) | Rutas clonadas (APFS) o enlazadas desde el checkout principal a los worktrees nuevos (`node_modules`, `.env`). | Lista de rutas | rutas relativas | vacía | Todas | Mod · Worktree |
| Setup script | Comando que corre tras crear un worktree (orca.yaml `scripts.setup`). | Texto multilínea | shell | vacío | Todas | Mod · Worktree |
| Archive script | Comando que corre antes de archivar un worktree. | Texto multilínea | shell | vacío | Todas | Mod · Worktree |
| When to run setup (`setupRunPolicy`) | Qué hacer por defecto cuando hay script de setup. | Selector | ask / run-by-default / skip-by-default | ask | Todas | Mod · Worktree |
| Setup agent startup (`setupAgentStartupPolicy`) | El agente arranca ya o espera al setup. | Selector | start-immediately / wait-for-setup | — | Todas | Mod · Worktree |
| Command source policy (`commandSourcePolicy`) | Qué comandos ejecutar: los compartidos (orca.yaml), los locales o ambos. | Selector | shared-only / local-only / run-both | — | Todas | Mod · Worktree |
| Hook mode (`hookSettings.mode`) | Hooks automáticos o sobrescritos por la UI. | Selector | auto / override | auto | Todas | Mod · Worktree |
| Custom GitHub issue command | Comando de issue enlazado, desde orca.yaml y un override local. | Texto | shell | vacío | Todas | No aplica |
| Default tabs (orca.yaml `defaultTabs`) | Pestañas de terminal que se crean una vez en un worktree nuevo (título, color, comando). | Lista en archivo | title, color, command | ninguna | Todas | Mod · Worktree/layout |
| Environment recipes (orca.yaml `environmentRecipes`) | Recetas de entornos por workspace (create, suspend, resume, destroy, timeouts, passEnv). | Lista en archivo | — | ninguna | Todas | No aplica |
| Shared directories (orca.yaml `worktree.sharedDirectories`) | Directorios grandes compartidos entre worktrees. | Lista en archivo | rutas | ninguna | Todas | Mod · Worktree |
| Git username (`gitUsername`) | Usuario para prefijos de rama de este repo. | Texto | texto | git config | Todas | Ya parcial · `worktree.prefix_mode = git` |
| Git AI author / source-control AI overrides | Sobrescribe agente, modelo, prompts y acciones de IA de control de código para este repo. | Selectores + texto | — | hereda el global | Todas | No aplica |
| GitHub account (`ghAccount`) | Vincula un login `gh` del llavero a las llamadas de la API de este proyecto. | Selector | login | ninguno | Todas | No aplica |
| Issue source preference | De qué remoto salen los issues. | Selector | upstream / origin / auto | auto | Todas | No aplica |
| Default Jira project | Proyecto Jira predeterminado del selector de issues. | Selector | proyecto | ninguno | Todas | No aplica |
| Keep fork up to date (`forkSyncMode`) | Avanza un fork desde upstream de forma segura. | Selector | ask / safe-auto / off | ask | Todas | Mod · Worktree/Cambios |
| Agent workspace trust (por proyecto) | Si Orca pre-confía los worktrees de este repo. | Selector | pre-trust / agent-asks | pre-trust | Todas | No aplica |
| Project runtime | Windows o WSL para este proyecto. | Selector | windows-host / WSL | heredado | Windows | No aplica |
| Project group (`projectGroupId`, `projectGroupOrder`) | Agrupar y ordenar proyectos en la barra. | Arrastrar | — | sin grupo | Todas | Ya · proyectos y orden en la barra |
| Cleanup rules (por proyecto) | Qué reglas de limpieza aplican (override de las del host). | Interruptores | — | hereda | Todas | Mod · Worktree |
| Dictation terms (por repo) | Términos propios del repo para el dictado. | Lista | — | vacía | Todas | No aplica |
| Browser profile (`browserProfileId`) | Perfil de navegador de las pestañas nuevas de este repo. | Selector | perfil | sigue Browser | Todas | No aplica |
| MCP configs | Inspecciona los archivos de configuración MCP del proyecto. | Lista/solo lectura | — | — | Todas | No aplica |
| Repo kind (`kind`) / folder upgrade | Proyecto git o carpeta simple, con promoción a repo. | Automático | git / folder | — | Todas | Mod |

## Automations, Artifacts, Skills y Session search

Rutas: Automations, Artifacts, Share Skills (+ Shared links), Agent Session Search. Fuente: automations/*, artifacts/*, skills/*, session-history/*, automation-definition.ts.

| Ajuste | Qué hace | Control | Valores | Defecto | Plataforma | Sunna |
|---|---|---|---|---|---|---|
| Automations · Show automations button | Atajo en la barra lateral. | Interruptor | true / false | true | Todas | Ya · `sidebar.buttons.bots` |
| Automation · name / prompt | Nombre y texto que recibe el agente. | Texto | texto | — | Todas | Ya · `SOUL.md` del bot |
| Automation · agent (`agentId`) | Agente que ejecuta. | Selector | agente | — | Todas | Ya · `bot.json` `agent` |
| Automation · surface | Chat o terminal. | Selector | chat / terminal | — | Todas | No aplica |
| Automation · extra agent args | Argumentos extra añadidos en cada corrida nueva. | Texto | texto | vacío | Todas | Ya parcial · `programs.json` |
| Automation · schedule (`rrule`, `timezone`) | Horario recurrente (hourly, daily, weekdays, weekly, custom) y zona horaria. | Selector + hora | rrule | — | Todas | Ya · triggers `interval` y `daily` (días, `grace_minutes`) |
| Automation · event triggers (`on`) | Eventos que la inician (GitHub, Google, etc.). | Lista | eventos | ninguno | Todas | Ya parcial · trigger `github` (consulta de PR) |
| Automation · once (`once`) | Espera única: corre en el primer evento o a una hora y se borra. | Fecha / evento | — | — | Todas | No aplica |
| Automation · precheck | Comando previo; si falla se omite la corrida (timeout 60 s, máx. 600). | Texto + número | shell; segundos | —; 60 | Todas | Ya · `precheck` |
| Automation · workspace mode | Corre en un workspace existente o crea uno nuevo por corrida. | Selector | existing / new_per_run | — | Todas | Ya parcial · cada corrida es una pestaña en el espacio `bot: <name>` |
| Automation · base branch / reuse session | Rama base y reutilizar la sesión. | Texto + interruptor | — | — | Todas | No aplica |
| Automation · missed run policy | Qué hacer con corridas perdidas (`run_once_within_grace`) con minutos de gracia. | Selector + número | run_once_within_grace; minutos | — | Todas | Ya · `grace_minutes` |
| Automation · enabled / run now | Activar, pausar o ejecutar ya. | Interruptor + botón | true / false | — | Todas | Ya · Pausar/Reanudar, «Ejecutar ahora» |
| Automation · execution target | Dónde corre: local o un host SSH. | Selector | local / ssh | local | Todas | Ya · carpeta o máquina del bot |
| Artifacts · Allow publishing public artifact links | Permite publicar HTML y Markdown como enlaces públicos. | Interruptor | true / false | true (`artifactSharingEnabled`) | Todas | No aplica |
| Artifacts · enabled / Show artifacts button | Función y atajo en la barra lateral. | Interruptores | true / false | true / false | Todas | No aplica |
| Share Skills · agent skill sharing (`agentSkillSharingEnabled`) | Comparte skills con un enlace no listado. | Interruptor | true / false | false | Todas | No aplica |
| Shared links | Lista de enlaces compartidos para revocar. | Lista | — | — | Todas | No aplica |
| Agent Session Search · enabled | Búsqueda de texto completo dentro de sesiones de agentes (por equipo). | Interruptor por equipo | true / false | false | Todas | Mod · Búsqueda |
| Agent Session Search · history days | Solo transcripciones modificadas en los últimos N días. | Selector/número | días / todo | todo (null) | Todas | Mod · Búsqueda |

## Git & Source Control

Ruta: Settings > Git & Source Control. Fuente: source-control/*, git/*. La CLI `orca search` y el panel de Source Control usan estas opciones.

| Ajuste | Qué hace | Control | Valores | Defecto | Plataforma | Sunna |
|---|---|---|---|---|---|---|
| Branch prefix | Prefijo que se antepone a las ramas al crear worktrees. | Selector | git-username / custom / none | git-username | Todas | Ya · `worktree.prefix_mode` |
| Branch prefix custom | Texto del prefijo propio. | Texto | texto | vacío | Todas | Ya · `worktree.prefix` |
| Branch name template | Plantilla de nombre de rama. | Texto | {name} {date} {repo} | {name} | Todas | Ya · `worktree.template` |
| Jira key casing | Casing de la clave Jira en nombres sugeridos (PROJ-123 vs proj-123). | Selector | lowercase / preserve | lowercase | Todas | No aplica |
| Keep local main up to date | Al crear un espacio actualiza y avanza la rama local (main/master) de forma segura. | Interruptor | true / false | false | Todas | Ya · `worktree.fresh` |
| Auto-rename branch & worktree | Renombra la rama autogenerada según el trabajo cuando arranca un agente. | Interruptor | true / false | true | Todas | Mod · Worktree |
| Branch name command template | Plantilla de comando del agente que genera nombres. | Texto | plantilla | — | Todas | No aplica |
| Source control group order | Staged Changes o Changes primero. | Selector | changes-first / staged-first / untracked-first | changes-first | Todas | Ya parcial · `changes.groups` (flat / status / new_first) |
| Default compare base | Base de comparación de cambios confirmados. | Selector | default branch / branch upstream (`sourceControlCompareAgainstUpstream`) | default branch | Todas | Ya · `changes.base` (head / default / upstream) |
| Source control view mode | Lista o árbol. | Selector | list / tree | list | Todas | Mod · Cambios |
| PR auto refresh | Cadencia del refresco de PR, reviews y checks. | Selector | default / reduced / minimal / off | default | Todas | No aplica |
| Show Source Control AI actions | Recetas de IA para commit, PR, nombre de rama y arreglos. | Interruptor | true / false | true | Todas | No aplica |
| AI action recipes (commit message, pull request, branch name, fix commit failure, fix push failure, fix checks, resolve conflicts, resolve comments) | Agente, modelo, argumentos y plantilla de comando de cada botón de IA; instrucciones por operación. | Selector + texto por acción | 8 acciones | plantillas de fábrica | Todas | No aplica |
| PR creation defaults | Valores al abrir el compositor: borrador, plantilla, generar detalles, abrir tras crear. | Interruptores (4) | draft, useTemplate, generateDetailsOnOpen, openAfterCreate | — | Todas | No aplica |
| Commit message AI (legado) | Agente, modelo, thinking, prompt y agente propio para mensajes de commit. | Selectores + texto | — | enabled: true | Todas | No aplica |
| GitHub API budget / GitLab API budget | Límites REST, Search y GraphQL actuales. | Información | — | — | Todas | No aplica |
| PR bot author overrides (`prBotAuthorOverrides`) | Autores bot reconocidos como tales en PR. | Lista | logins | vacía | Todas | No aplica |
| Hosted review creation | Crear PR/MR desde la app. | Acción | — | — | Todas | Mod · Cambios (push, pull, rama) |

## Task Sources

Ruta: Settings > Task Sources. Fuente: tasks/*, task-providers.ts. Sunna no tiene lista de tareas.

| Ajuste | Qué hace | Control | Valores | Defecto | Plataforma | Sunna |
|---|---|---|---|---|---|---|
| Visible task providers | Qué proveedores aparecen en Tasks. | Casillas | github / gitlab / linear / jira | los cuatro | Todas | No aplica |
| Default task source | Fuente inicial. | Selector | github / gitlab / linear / jira | github | Todas | No aplica |
| Default task view preset | Vista inicial (all, assigned, created…). | Selector | all … | all | Todas | No aplica |
| Default repo / Linear team selection | Repositorio y equipo Linear por defecto. | Selectores | — | ninguno | Todas | No aplica |
| Sync task status from workspace board | El estado de la tarea sigue al tablero de workspaces. | Interruptor | true / false | false | Todas | No aplica |
| Provider setup steps | Pasos guiados: conectar host de código, Linear, Jira y «Show in Tasks». | Asistente | — | — | Todas | No aplica |

## Terminal

Ruta: Settings > Terminal. Fuente: terminal/*, default-global-settings.ts. Sunna: Ajustes > Look/Keyboard, `settings.json`, layout.

| Ajuste | Qué hace | Control | Valores | Defecto | Plataforma | Sunna |
|---|---|---|---|---|---|---|
| Font size | Tamaño de fuente de terminales (nuevos y en vivo). | Número | px | 14 | Todas | Ya · `font_size` |
| Font family | Familia de fuente del terminal. | Selector con búsqueda | familias | según SO | Todas | Ya · `font` |
| Font weight | Peso del texto; el negrita queda en 700. | Selector/número | 100 a 900 | según SO (Windows: Regular distinto) | Todas | Ya · `font_weight` (500) |
| Bold font weight (`terminalFontWeightBold`) | Peso del texto en negrita. | Selector | valor de fábrica | 700 | Todas | Ya · fijo en 700 |
| Line height | Multiplicador de interlineado. | Número | multiplicador | 1 | Todas | Ya · `terminal_line_height` (0,8 a 2) |
| Font ligatures | Ligaduras de programación. | Selector | auto / on / off | auto | Todas | Ya parcial · `font_ligatures` on/off (off) |
| GPU acceleration | WebGL de xterm.js con fallback. | Selector | auto / on / off | auto | Todas | No aplica · render nativo GPUI |
| Color contrast | Mejora legibilidad o respeta los colores del programa. | Selector/deslizador | ratio | — | Todas | Ya · `minimum_contrast` (1 a 21) |
| Cursor shape | Forma del cursor. | Selector | bar / block / underline | block | Todas | Núcleo |
| Blinking cursor | Variante parpadeante. | Interruptor | true / false | true | Todas | Núcleo |
| Cursor opacity | Opacidad del cursor. | Deslizador | 0 a 1 | — | Todas | Núcleo |
| Dark theme / Light theme | Tema del terminal en modo oscuro y claro (más de 100 integrados). | Selector (2) | nombre de tema | Ghostty Default Style Dark / Builtin Tango Light | Todas | Ya · tema por apariencia (`theme`) |
| Use separate light theme (`terminalUseSeparateLightTheme`) | El modo claro usa su propio tema de terminal. | Interruptor | true / false | true | Todas | Ya · `theme` {light, dark} |
| Dark/Light divider color | Color de la línea divisoria entre paneles. | Color (2) | hex | #3f3f46 / #d4d4d8 | Todas | Ya · `colors` / marco de panel del layout |
| Color overrides (`terminalColorOverridesSplit`) | Sobrescribe colores individuales del terminal, por tema claro/oscuro. | Colores | hex | vacío | Todas | Ya · `colors` |
| Import theme from YAML / Warp / Ghostty | Importa temas. | Botones | — | — | Todas | Ya · Import theme (Ghostty, Warp, VS Code) |
| Custom terminal themes (`terminalCustomThemes`) | Temas importados guardados. | Lista | — | vacía | Todas | Ya · `themes/<name>.json` |
| Match Claude Code to terminal theme | Claude Code usa los colores del terminal y los sigue en vivo. | Interruptor | true / false | false | Todas | No aplica |
| Inactive pane opacity | Opacidad de paneles no activos. | Deslizador | 0 a 1 | según plataforma | Todas | Núcleo · hoy solo marco por layout |
| Active pane opacity / transition ms | Opacidad del panel activo y duración de la transición. | Números | 0-1; ms | 1; 140 | Todas | Núcleo |
| Divider thickness | Grosor de la línea divisoria. | Número | px | 3 | Todas | Ya · tamaños del layout |
| Background opacity | Transparencia del fondo del terminal. | Deslizador | 0 a 1 | — | Todas | Ya · `terminal_opacity` (layout) |
| Horizontal / Vertical padding | Relleno alrededor de la cuadrícula. | Números (2) | px | — | Todas | Ya · `terminal_padding` (layout) |
| Hide mouse while typing | Oculta el puntero al escribir. | Interruptor | true / false | false | Todas | Núcleo |
| Scroll to bottom on input | Al escribir o pegar con scroll arriba salta al final. | Interruptor | true / false | true | Todas | Núcleo |
| Scroll speed (normal / fast / TUI) | Sensibilidad de scrollback normal, rápido con modificador y de TUI a pantalla completa. | Deslizador (3) | multiplicadores | 1,15 / 5 / 1 | Todas | Ya parcial · `terminal_scroll_speed` (uno solo, 0,1 a 5) |
| Scrollback rows | Filas de historial retenidas. | Número | filas | 5 000 | Todas | Núcleo/herdr-core |
| Ask before closing running terminals | Confirma antes de cerrar un terminal con proceso activo. | Interruptor | true / false | true | Todas | Ya · `confirmed` al cerrar |
| Focus follows mouse | Pasar el puntero activa el panel (como Ghostty). | Interruptor | true / false | false | Todas | Núcleo |
| Copy on select | Copia al portapapeles al terminar de seleccionar. | Interruptor | true / false | false | Todas | Ya · `copy_on_select` (true) |
| Allow TUI clipboard writes (OSC 52) | Permite que programas copien al portapapeles, incluso por SSH. | Interruptor | true / false | true | Todas | Ya · `clipboard_write` |
| Trim gutter on copy | Quita la sangría común que pinta la salida de agentes al copiar. | Interruptor | true / false | true | Todas | Núcleo |
| Right-click to paste | Clic derecho pega; Ctrl+clic derecho abre el menú. | Interruptor | true / false | true en Windows, false en el resto | Todas | Ya parcial · `terminal.right_click` {to: program\|menu} |
| Option as Alt | Option envía secuencias Alt/Esc o compone caracteres. | Selector | auto / on / off / left / right | auto | macOS | Ya · `option_as_alt` |
| JIS Yen to Backslash | La tecla ¥ de teclados JIS envía barra invertida. | Interruptor | true / false | false | macOS | Núcleo |
| Word separators | Caracteres que delimitan palabras al hacer doble clic. | Texto | caracteres | — | Todas | Núcleo |
| Maximum running terminals | Tope de terminales en ejecución en el host. | Número | entero | — | Todas | Núcleo |
| Default shell | Shell y argumentos (login, wrapper, rcfile) de terminales locales nuevos; para fish, zsh, bash, nushell… | Texto | ruta + args | vacío (shell de login) | macOS/Linux | Ya parcial · el núcleo elige el shell por defecto (AGENTS.md) |
| Windows shell (`terminalWindowsShell`) / WSL distro | Shell por defecto en Windows (powershell.exe, cmd, WSL). | Selector | powershell.exe / … | powershell.exe | Windows | Ya · PowerShell 7 si existe |
| PowerShell version | Windows PowerShell o PowerShell 7+. | Selector | auto / powershell.exe / pwsh.exe | auto | Windows | Ya |
| Administrator mode | Permisos de Windows o reinicio con aprobación de administrador. | Botón | — | — | Windows | No aplica |
| Inline images / Dropped images preview | Imágenes en línea y vista previa de imágenes soltadas. | Interruptores | true / false | true / true | Todas | Ya · gráficos Kitty (AGENTS.md) |
| Terminal link click behavior | Clic en URL del terminal: acciones, abrir o nada; clic central. | Selectores (2) | actions / open / none; open / actions / none | actions; open | Todas | Núcleo |
| Terminal link action popover | Muestra el menú de acciones sobre el enlace. | Interruptor | true / false | true | Todas | Núcleo |
| Scope history by worktree / hidden view parking / SSH view parking / retention budget | Mantenimiento de terminales ocultos y de historial por worktree (ajustes de rendimiento). | Interruptores (6) | true / false | true | Todas | No aplica |
| Manage sessions | Recupera terminales congelados: matar sesiones, borrar scrollback guardado, reiniciar el daemon. | Botones | — | — | Todas | Ya parcial · `terminal.kill` y herdr-core |
| Import from Ghostty / Kitty | Importa una vez los ajustes compatibles. | Botones | — | — | Todas | Ya parcial · importa solo temas |
| New tab menu layout (`newTabMenuLayout`) | Orden y visibilidad de las filas de creación del menú «+» (barra y panel flotante). | Lista arrastrable | order, hidden | de fábrica | Todas | Mod · mod `new-menu` |

## Quick Commands, Browser, Mobile emulator y Floating workspace

Rutas: Quick Commands; Browser (y Browser use, Extensions, Profiles); Mobile Emulator; Floating Workspace. Fuente: quick-commands/*, browser/*, mobile/emulator/*, floating-workspaces/*.

| Ajuste | Qué hace | Control | Valores | Defecto | Plataforma | Sunna |
|---|---|---|---|---|---|---|
| Quick commands · list | Comandos de terminal guardados, lanzables desde cualquier terminal; alcance global o por proyecto; con búsqueda, límite de largo de prompt y capacidades por agente. | Lista editable (nombre, comando, alcance) | — | lista de fábrica (`getDefaultTerminalQuickCommands`) | Todas | Mod |
| Quick commands · menu shortcut | Atajo para abrir el menú de comandos rápidos (`tab.openQuickCommandsMenu`). | Atajo | — | sin asignar | Todas | Mod · paleta `app.palette` |
| Browser · Default home page | URL de pestañas nuevas. | Texto | URL | vacío (en blanco) | Todas | No aplica |
| Browser · Default search engine | Buscador de la barra de direcciones. | Selector | google / duckduckgo / bing / kagi | — | Todas | No aplica |
| Browser · Default zoom | Zoom de pestañas nuevas. | Selector | nivel | 0 | Todas | No aplica |
| Browser · Open links in app | Los enlaces se abren en el navegador integrado; modificador invierte; en segundo plano; pregunta inicial. | Interruptores (3) | true / false | false / false / false | Todas | No aplica |
| Browser · Terminal URL clicks | Qué hace un clic en una URL de terminal o chat. | Selector | actions / open / none | actions | Todas | No aplica |
| Browser · Localhost worktree labels | URLs localhost específicas por worktree. | Interruptor | true / false | false | Todas | No aplica |
| Browser · Session & cookies / Import cookies | Importa cookies de Chrome, Edge u otros. | Botón | — | — | Todas | No aplica |
| Browser · Site permissions | Cámara, micrófono, ubicación, abrir otras apps, por sitio. | Lista | allow / block | pregunta | Todas | No aplica |
| Browser · Identity (user agent) | Cadena de user agent del navegador. | Texto/selector | — | — | Todas | No aplica |
| Browser · Memory Saver | Libera memoria de pestañas no vistas en N horas. | Selector | horas / apagado | 4 h | Todas | No aplica |
| Browser · Profile for new tabs | Un perfil compartido o uno por repo o worktree. | Selector | shared / repo / worktree | shared | Todas | No aplica |
| Browser · Extensions | Extensiones de Chrome del perfil local. | Interruptor + lista | true / false | true | Todas | No aplica |
| Browser · Passwords | Contraseñas del navegador (BrowserPasswords). | Interruptor | true / false | — | Todas | No aplica |
| Browser use skill | Instala la skill para que los agentes operen el navegador; importar cookies para ellos. | Botones | — | — | Todas | No aplica |
| Browser client-hosted remote | Navegador alojado en el cliente para servidores remotos. | Interruptor | true / false | true | Todas | No aplica |
| Mobile emulator · enabled | Soporte de emulador móvil para Orca y agentes. | Interruptor | true / false | true | Todas | No aplica |
| Mobile emulator · Default device | Dispositivo que se abre por defecto. | Selector | UDID | ninguno | Todas | No aplica |
| Mobile emulator · availability / Android SDK path | Comprueba Xcode, simctl, serve-sim y ruta del SDK de Android. | Información + ruta | — | null | Todas | No aplica |
| Mobile emulator · Agent CLI control | Comandos `orca emulator` (list, attach, tap, type, gesture, button, rotate, exec, kill). | Información | — | — | Todas | No aplica |
| Floating workspace · enabled (`floatingTerminalEnabled`) | Habilita el espacio flotante y dónde empiezan sus pestañas. | Interruptor | true / false | true | Todas | Ya parcial · bloques flotantes del layout y `terminal.peek` |
| Floating workspace · working directory (`floatingTerminalCwd`) | Directorio inicial del terminal flotante. | Ruta | ruta | ~ | Todas | Mod |
| Floating workspace · trusted cwds | Directorios aprobados para el flotante. | Lista | rutas | vacía | Todas | No aplica |
| Floating workspace · toggle button location | Dónde aparece el botón de alternar. | Selector | floating-button / status-bar | floating-button | Todas | Mod · layout |
| Floating workspace · toggle / maximize / minimize shortcuts | Atajos del panel flotante. | Atajos | — | Mod+Alt+A / Mod+Alt+Shift+A / — | Todas | Mod |

## Bots (global y por bot)

Ruta: Settings > Bots y una página por bot. Fuente: bots/*, bot-types.ts, bots-global-settings.ts. Sunna: mod `bots` con `bot.json` y plantillas.

| Ajuste | Qué hace | Control | Valores | Defecto | Plataforma | Sunna |
|---|---|---|---|---|---|---|
| Enable bots (`botsEnabled`) | Activa los bots en este host (los apaga la política del equipo). | Interruptor | true / false | true | Todas | Ya · mod `bots` |
| Connectors (Slack, Linear, GitHub, Google, Composio) | Dónde se puede contactar a tus bots; clave de Composio. | Lista + botones | — | — | Todas | No aplica |
| Bot · Name / avatar / color / outfit | Nombre, mascota, color de baldosa y vestimenta. | Texto + selectores | — | — | Todas | Ya parcial · nombre (carpeta `bots/<name>/`); no hay mascota |
| Bot · Agent | Agente del bot (23 agentes). | Selector | claude, codex, grok, opencode, gemini, cursor, … | — | Todas | Ya · `bot.json` `agent` |
| Bot · Paused | Pausar o reanudar. | Interruptor | true / false | false | Todas | Ya · Pausar/Reanudar (`enabled`) |
| Bot · Permissions (`permissionMode`) | Permiso del bot. | Selector | ask / accept-edits / auto / full-access | como los chats | Todas | Ya parcial · `yolo` por bot |
| Bot · Model and launch options | Modelo y opciones del agente. | Selectores | — | — | Todas | No aplica |
| Bot · Notifications | Avisos por bot. | Interruptores (3) | needsPermission / taskComplete / couldNotFinish | — | Todas | Ya parcial · sonidos globales de Alertas |
| Bot · Who can ask (members) | Personas que pueden pedirle cosas por cada conector. | Lista | — | — | Todas | No aplica |
| Bot · Look around | Revisa solo si algo espera al dueño: activo, cada 4 h / 2 h / dos veces al día / mañanas, solo días hábiles, qué buscar. | Interruptor + selector + texto | every-4h / every-2h / twice-daily / mornings | activo, every-4h | Todas | Ya parcial · triggers `interval` y `daily` con `days` |
| Bot · Tidy up | Ordenado nocturno de su memoria. | Interruptor + selector | night / week | activo, night | Todas | No aplica |
| Bot · Items stale after | Días sin tocar antes de pasar un pedido a Stale. | Selector | 3 / 5 / 7 / 14 / never | 5 | Todas | No aplica |
| Bot · Soul editor / Memory | Editar `SOUL.md`; ver y editar memoria e historial. | Editor | — | — | Todas | Ya · `SOUL.md` |
| Bot · Skills / Tools / project skills | Skills propias, herramientas conectadas y proyectos cuyas skills carga. | Listas | — | — | Todas | No aplica |
| Bot · Google access / Slack mirror | Usar cuentas Google del host; reflejar mensajes en Slack. | Interruptores | true / false | false / true | Todas | No aplica |
| Bot · Folder / run limits / precheck | Carpeta de trabajo, tope diario y comando previo. | Ruta + número + texto | — | — | Todas | Ya · `folder`, `max_runs_per_day`, `precheck` |
| Bot · Triggers | Manual, intervalo, diario, GitHub. | Lista | manual / interval / daily / github | — | Todas | Ya |
| Bot · Templates | Galería de plantillas. | Galería | — | — | Todas | Ya · «Plantillas» |
| Bot sidebar view | Orden, ocultar pausados y trabajadores dormidos. | Selectores | sortBy; hidePausedBots; hideSleepingWorkers | manual; false; false | Todas | Mod · Bots |
| Hide bot-created workspaces | Oculta espacios que crearon bots. | Interruptor | true / false | true | Todas | Mod · Espacios |

## Shortcuts (ajustes de la página)

Ruta: Settings > Shortcuts. El catálogo completo de comandos está en la sección «Catálogo de atajos». Fuente: shortcuts/*, keybindings/global-settings.ts, window-shortcut-policy.ts.

| Ajuste | Qué hace | Control | Valores | Defecto | Plataforma | Sunna |
|---|---|---|---|---|---|---|
| Shortcuts in terminal (`terminalShortcutPolicy`) | Quién gana cuando se superponen: Orca o el terminal enfocado. | Selector | orca-first / terminal-first | orca-first | Todas | Ya · secciones `terminal` vs `global` de `keys.json` |
| Shortcut recorder / per-action override | Cambiar o grabar el atajo de cada acción; aviso de conflictos y de atajos que interceptan el SO. | Grabador de teclas | uno o varios acordes por acción | ver catálogo | Todas | Ya · Settings > Keyboard («Press the new keys…», «Already used by») |
| Reset shortcut / reset all | Vuelve al atajo de fábrica. | Botón | — | — | Todas | Ya parcial · editar `keys.json` lo vuelve propio (nota en la página) |
| Shortcut file (`~/.orca/keybindings.json`) | Los atajos se guardan en un archivo compartido entre perfiles; hay semillas de migración (tab switch, search). | Archivo | JSON de overrides | ausente | Todas | Ya · `keys.json` |
| Search shortcuts / filter by group | Búsqueda dentro de la lista de atajos. | Campo de texto | — | — | Todas | Ya · `app.help` filtrable |
| Double-tap modifier bindings | Un atajo puede ser un doble toque de Cmd, Ctrl, Alt o Shift (ventana de 300 ms); lo usa el dictado. | Atajo | — | — | Todas | Núcleo |
| Plugin shortcuts (`plugin:<id>/<action>`) | Los plugins registran acciones con atajo propio. | Atajo | — | — | Todas | Ya · `<mod>.<command>` en `keys.json` |
| macOS symbolic hotkeys conflict check | Avisa si un atajo choca con atajos del sistema de macOS. | Aviso | — | — | macOS | Núcleo |
| Ctrl+Tab order (`ctrlTabOrderMode`) | Ver Appearance (Tab order). | — | — | — | Todas | Ya · `tabs.order` |

## Estado de interfaz fuera de Ajustes (barra lateral, filtros y vistas)

No tiene página en Settings: se cambia desde menús y filtros de la barra lateral y se guarda en el estado de UI (`persisted-ui-state-types.ts`). Es el grueso de lo «oculto» que la lista previa no cubría.

| Ajuste | Qué hace | Control | Valores | Defecto | Plataforma | Sunna |
|---|---|---|---|---|---|---|
| Sidebar width / open | Ancho y apertura de la barra lateral izquierda. | Arrastrar / atajo | px | 280 / abierta | Todas | Ya · `layout.resize`, `layout.toggle` |
| Right sidebar open / tab / width / explorer view | Apertura, pestaña (explorer, search, source control, checks, plugin:*), ancho y vista del explorador (files / search). | Atajos y arrastre | px; pestañas | abierta / explorer / 350 / files | Todas | Ya · `layout.select`, `layout.resize` |
| Group by | Agrupa las tarjetas de espacio. | Selector (menú Filters) | none / workspace-status / repo / pr-status | repo | Todas | Ya parcial · agrupado por repo y máquina |
| Sort by | Orden de los espacios. | Selector | name / smart / recent / repo / manual | recent | Todas | Mod · Espacios (`space.move`) |
| Project order by | Orden de los proyectos. | Selector | manual / recent | manual | Todas | Ya parcial · orden manual de proyectos |
| Show active only | Solo espacios con agente activo. | Interruptor | true / false | false | Todas | Mod · Espacios |
| Hide sleeping workspaces / show sleeping / inactive | Oculta o muestra espacios dormidos o inactivos. | Interruptores | true / false | false | Todas | Mod · Espacios (ya agrupa «earlier» en el selector) |
| Hide default-branch workspace / always show default branch | Oculta o fuerza el espacio de la rama principal. | Interruptores | true / false | false / true | Todas | Mod · Espacios |
| Hide automation-generated / CLI-created / detached HEAD / other-device workspaces | Filtros por origen del espacio. | Interruptores (4) | true / false | false | Todas | Mod · Espacios |
| Workspace host scope / visible hosts / host order | Qué hosts (local, ssh, runtime) se ven y en qué orden. | Selector + lista | all / local / ssh:* / runtime:* | all | Todas | Ya · agrupado por máquina |
| Filter by project (`filterRepoIds`) | Muestra solo ciertos proyectos. | Casillas | ids | vacío | Todas | Mod · Espacios |
| Collapsed groups | Qué grupos están plegados. | Estado | ids | vacío | Todas | Ya · plegar grupos |
| Sidebar projects first / bots height | Proyectos primero y altura de la sección Bots. | Interruptor + arrastre | true / false; px | — | Todas | Mod · layout |
| Agents view: read filter / group by / compact / search / child agents | Vista Agents: filtro de leídos, agrupar (por defecto status), compacta, buscador, mostrar hijos y filtros por host/proyecto/origen. | Selectores e interruptores | all…; status…; true/false | all; status; true; true; false | Todas | Ya · mod `activity` (filtros por estado, programa y texto) |
| Workspace statuses (kanban) | Estados personalizados del tablero de workspaces (nombre, color, ícono, orden), opacidad y ancho de columna. | Lista editable + deslizadores | — | estados de fábrica; opacidad 1; ancho 308 | Todas | No aplica |
| Explorer: display root / show dotfiles per worktree | Raíz mostrada y archivos ocultos por worktree. | Interruptores | true / false | — | Todas | Mod · Archivos |
| Editor font zoom level | Zoom del editor, separado del de la UI. | Atajos | nivel | 0 | Todas | No aplica |
| Markdown TOC / combined diff tree widths | Anchos de paneles auxiliares. | Arrastrar | px | 240 / 256 | Todas | No aplica |
| Window bounds / maximized / fullscreen | Posición y estado de la ventana. | Automático | — | — | Todas | Ya · `view.json` |
| Notice flags (dismissed notices, tips, tours) | Decenas de banderas de avisos ya vistos (tray, plugins alpha, tarjetas de worktree, speech model, etc.). | Estado | true / false | false | Todas | Mod |
| Task resume state / GitHub projects | Recuerda filtros y vistas de Tasks y proyectos GitHub fijados. | Estado | — | vacío | Todas | No aplica |
| Browser defaults (home, search engine, zoom, DevTools dock) | Persistidos en UI, no en GlobalSettings. | Estado | — | — | Todas | No aplica |

## Catálogo de atajos (src/shared/keybindings/definitions-core-1..4.ts)

103 comandos más un «New X tab» por cada agente soportado (sin atajo por defecto; el título es `New <agente> tab`, categoría Agents). `Cmd` es la tecla Mod en macOS y `Ctrl` en Linux y Windows. Un guion significa sin atajo por defecto en esa plataforma. Los comandos con alcance de editor, navegador, chat o explorador solo actúan con ese panel enfocado; el resto de conflictos se resuelven con la política `terminalShortcutPolicy`.

| Grupo | Comando (inglés) | Id | macOS | Linux | Windows | Sunna |
|---|---|---|---|---|---|---|
| Global | Go to File | worktree.quickOpen | Cmd+P | Ctrl+P | Ctrl+P | Mod · `cmd-p` abre el selector de pestañas, no busca archivos (la búsqueda de archivos está en el panel Search) |
| Global | Open Settings | app.settings | Cmd+, | — | — | Ya · `cmd-,` `app.settings` |
| Global | Force Reload | app.forceReload | Cmd+Shift+R | Ctrl+Shift+R | Ctrl+Shift+R | No aplica |
| Global | Search everything | worktree.palette | Cmd+K | Ctrl+K | Ctrl+K | Ya · `cmd-k` / `cmd-shift-p` `app.palette` |
| Global | Previous worktree | worktree.navigateUp | Cmd+Shift+↑ | Ctrl+Shift+↑ | Ctrl+Shift+↑ | Ya · `cmd-shift-up` `space.prev` |
| Global | Next worktree | worktree.navigateDown | Cmd+Shift+↓ | Ctrl+Shift+↓ | Ctrl+Shift+↓ | Ya · `cmd-shift-down` `space.next` |
| Global | Create worktree | workspace.create | Cmd+N · Cmd+Shift+N | Ctrl+N · Ctrl+Shift+N | Ctrl+N · Ctrl+Shift+N | Ya · `cmd-n` `space.new` |
| Global | Rename worktree | workspace.rename | Cmd+Alt+R | — | — | Ya · `cmd-alt-r` / `shift-f2` `space.rename` |
| Global | Delete Selected Workspace | workspace.delete | Cmd+Shift+⌫ | Ctrl+Shift+⌫ | Ctrl+Shift+⌫ | Mod · `space.close` existe sin atajo |
| Global | Toggle Workspace Board | workspace.openBoard | — | — | — | No aplica |
| Global | Toggle Agent Dashboard | dashboard.toggle | — | — | — | Mod · panel `activity`, se abre con `mod.open` o `layout.select` |
| Global | Select Workspace 1–9 | workspace.selectByIndex | Cmd+1 | Ctrl+1 | Ctrl+1 | Ya · `cmd-1`..`cmd-9` `space.select` |
| Global | Dictation | voice.dictation | Cmd+E | Ctrl+E | Ctrl+E | No aplica |
| Global | Open Tasks | view.tasks | — | — | — | No aplica |
| Global | Open Plugins | view.plugins | Cmd+Alt+X | Ctrl+Shift+Y | Ctrl+Shift+Y | Ya parcial · `app.settings {section: mods}` sin atajo |
| Global | Toggle Sidebar | sidebar.left.toggle | Cmd+B | Ctrl+B | Ctrl+B | Ya · `cmd-b` `layout.toggle:sidebar` |
| Global | Toggle Right Sidebar | sidebar.right.toggle | Cmd+L | Ctrl+L | Ctrl+L | Ya · `cmd-l` `layout.toggle:panel` |
| Global | Show Explorer | sidebar.explorer.toggle | Cmd+Shift+E | Ctrl+Shift+E | Ctrl+Shift+E | Ya parcial · `layout.select` al panel Files, sin atajo |
| Global | Show Search | sidebar.search.toggle | Cmd+Shift+F | Ctrl+Shift+F | Ctrl+Shift+F | Ya parcial · `layout.select` al panel Search, sin atajo |
| Global | Show Source Control | sidebar.sourceControl.toggle | Cmd+Shift+G | Ctrl+Shift+G | Ctrl+Shift+G | Ya parcial · `layout.select` al panel Changes, sin atajo |
| Global | Show Checks | sidebar.checks.toggle | — | — | — | No aplica |
| Global | Show Ports | sidebar.ports.toggle | Cmd+Shift+I | — | — | Mod |
| Global | Toggle Sleeping Workspaces | sidebar.sleepingWorkspaces.toggle | — | — | — | Mod |
| Global | Focus worktree list | sidebar.focusWorktreeList | Cmd+Shift+0 | Ctrl+Shift+0 | Ctrl+Shift+0 | Núcleo |
| Global | Toggle Floating Terminal | floatingTerminal.toggle | Cmd+Alt+A | Ctrl+Alt+A | Ctrl+Alt+A | Mod · `terminal.peek` y bloques flotantes del layout |
| Global | Maximize Floating Workspace Panel | floatingWorkspace.maximize | Cmd+Alt+Shift+A | — | — | Mod |
| Global | Minimize Floating Workspace Panel | floatingWorkspace.minimize | — | — | — | Mod |
| Global | Zoom In | zoom.in | Cmd+= · Cmd+Shift++ · Cmd+Num+ | Ctrl+= · Ctrl+Shift++ · Ctrl+Num+ | Ctrl+= · Ctrl+Shift++ · Ctrl+Num+ | Ya · `cmd-=` / `cmd-+` `app.zoom_in` |
| Global | Zoom Out | zoom.out | Cmd+- · Cmd+Num- | Ctrl+- · Ctrl+Num- | Ctrl+- · Ctrl+Num- | Ya · `cmd--` `app.zoom_out` |
| Global | Reset Size | zoom.reset | Cmd+0 | Ctrl+0 | Ctrl+0 | Ya · `cmd-0` `app.zoom_reset` |
| Global | Worktree History Back | worktree.history.back | Cmd+Alt+← | Ctrl+Alt+← | Ctrl+Alt+← | Núcleo |
| Global | Worktree History Forward | worktree.history.forward | Cmd+Alt+→ | Ctrl+Alt+→ | Ctrl+Alt+→ | Núcleo |
| Tabs | New terminal tab | tab.newTerminal | Cmd+T | Ctrl+T | Ctrl+T | Ya · `cmd-t` `tab.new` |
| Tabs | New agent tab (default agent) | tab.newAgent | Cmd+Alt+T | — | — | Ya parcial · `tab.new {argv}`; sin agente por defecto ni atajo |
| Tabs | New browser tab | tab.newBrowser | Cmd+Shift+B | Ctrl+Shift+B | Ctrl+Shift+B | No aplica |
| Tabs | New browser tab in right split | tab.newBrowserSplitRight | Cmd+Alt+B | — | — | No aplica |
| Tabs | New mobile emulator tab | tab.newSimulator | Cmd+Alt+Shift+E | — | — | No aplica |
| Tabs | New markdown tab | tab.newMarkdown | Cmd+Shift+M | Ctrl+Shift+M | Ctrl+Shift+M | No aplica |
| Tabs | Open markdown tab | tab.openMarkdown | Cmd+Shift+O | Ctrl+Shift+O | Ctrl+Shift+O | No aplica |
| Tabs | Close active tab | tab.close | Cmd+W | Ctrl+W | Ctrl+W | Ya · `cmd-w` `tab.close` |
| Tabs | Close all editor tabs | tab.closeAll | Cmd+Alt+W | Ctrl+Alt+W | Ctrl+Alt+W | Mod |
| Tabs | Rename active tab | tab.rename | Cmd+R | — | — | Ya · `cmd-r` / `f2` `tab.rename` |
| Tabs | Reopen closed tab | tab.reopenClosed | Cmd+Shift+T | Ctrl+Shift+T | Ctrl+Shift+T | Núcleo |
| Tab Navigation | Next tab (same type) | tab.nextSameType | Cmd+Alt+] | Ctrl+Alt+] | Ctrl+Alt+] | Ya parcial · `tab.next` (no distingue tipo) |
| Tab Navigation | Previous tab (same type) | tab.previousSameType | Cmd+Alt+[ | Ctrl+Alt+[ | Ctrl+Alt+[ | Ya parcial · `tab.prev` (no distingue tipo) |
| Tab Navigation | Next tab (all types) | tab.nextAllTypes | Cmd+Shift+] | Ctrl+Shift+] | Ctrl+Shift+] | Ya · `cmd-shift-]` `tab.next` |
| Tab Navigation | Previous tab (all types) | tab.previousAllTypes | Cmd+Shift+[ | Ctrl+Shift+[ | Ctrl+Shift+[ | Ya · `cmd-shift-[` `tab.prev` |
| Tab Navigation | Previous recent tab | tab.previousRecent | Ctrl+Tab | Ctrl+Tab | Ctrl+Tab | Ya parcial · `ctrl-shift-tab` `tab.prev` |
| Tab Navigation | Next terminal tab | tab.nextTerminal | Ctrl+PgDn | Ctrl+PgDn | Ctrl+PgDn | Ya parcial · `ctrl-tab` `tab.next` |
| Tab Navigation | Previous terminal tab | tab.previousTerminal | Ctrl+PgUp | Ctrl+PgUp | Ctrl+PgUp | Ya parcial · `ctrl-shift-tab` `tab.prev` |
| Tab Navigation | Select Tab 1–9 | tab.selectByIndex | Ctrl+1 | Alt+1 | Alt+1 | Ya · `ctrl-1`..`ctrl-9` `tab.select` |
| Quick Commands | Toggle Quick Commands menu | tab.openQuickCommandsMenu | — | — | — | Mod |
| Browser | Find in Browser | browser.find | Cmd+F | Ctrl+F | Ctrl+F | No aplica |
| Browser | Go Back in Browser | browser.back | Cmd+[ | Alt+← | Alt+← | No aplica |
| Browser | Go Forward in Browser | browser.forward | Cmd+] | Alt+→ | Alt+→ | No aplica |
| Browser | Show Browser History | browser.showHistory | Cmd+Y | Ctrl+H | Ctrl+H | No aplica |
| Browser | Reload Browser Page | browser.reload | Cmd+R | Ctrl+R | Ctrl+R | No aplica |
| Browser | Hard Reload Browser Page | browser.hardReload | Cmd+Shift+R | Ctrl+Shift+R | Ctrl+Shift+R | No aplica |
| Browser | Focus Browser Address Bar | browser.focusAddressBar | Cmd+L | Ctrl+L | Ctrl+L | No aplica |
| Browser | Grab Page Element | browser.grabElement | Cmd+C | Ctrl+C | Ctrl+C | No aplica |
| Browser | Annotate Page Element | browser.annotateElement | Cmd+Shift+C | Alt+Shift+N | Alt+Shift+N | No aplica |
| Editors | Find in editor | editor.find | Cmd+F | Ctrl+F | Ctrl+F | No aplica · Files es de solo lectura |
| Editors | Replace in editor | editor.replace | Cmd+Alt+F | Ctrl+H | Ctrl+H | No aplica |
| Editors | Save File | editor.save | Cmd+S | Ctrl+S | Ctrl+S | No aplica |
| Editors | Show Markdown Preview | editor.markdownPreview | Cmd+Shift+V | Ctrl+Shift+V | Ctrl+Shift+V | No aplica |
| Editors | Toggle Word Wrap | editor.toggleWordWrap | Alt+Z | Alt+Z | Alt+Z | No aplica |
| Editors | Toggle Changes View | editor.toggleChangesView | — | — | — | Mod |
| Editors | Copy Context | editor.copyContext | Cmd+Alt+C | Ctrl+Alt+C | Ctrl+Alt+C | No aplica |
| Editors | Go to Previous Change | editor.previousChange | Shift+F7 | Shift+F7 | Shift+F7 | No aplica |
| Editors | Go to Next Change | editor.nextChange | F7 | F7 | F7 | No aplica |
| Editors | Add Review Note | editor.addReviewNote | Cmd+Shift+A | Ctrl+Shift+A | Ctrl+Shift+A | No aplica |
| Global | Send Review Notes to Agent | sourceControl.sendReviewNotes | — | — | — | No aplica |
| File Explorer | Undo file operation | fileExplorer.undo | Cmd+Z | Ctrl+Z | Ctrl+Z | No aplica |
| File Explorer | Redo file operation | fileExplorer.redo | Cmd+Shift+Z | Ctrl+Shift+Z · Ctrl+Y | Ctrl+Shift+Z · Ctrl+Y | No aplica |
| File Explorer | Copy file path | fileExplorer.copyPath | Cmd+Alt+C | Alt+Shift+C | Alt+Shift+C | Mod · «Copy path» del menú |
| File Explorer | Copy relative file path | fileExplorer.copyRelativePath | Cmd+Alt+Shift+C | Ctrl+Alt+Shift+C | Ctrl+Alt+Shift+C | Mod |
| File Explorer | Delete file | fileExplorer.delete | Cmd+⌫ · Supr | Supr | Supr | No aplica |
| Settings | Search Settings | settings.search | Cmd+F | Ctrl+F | Ctrl+F | Ya parcial · caja «Search settings», sin atajo |
| Terminal Panes | Copy terminal selection | terminal.copySelection | Cmd+C | Ctrl+Shift+C · Ctrl+C | Ctrl+Shift+C · Ctrl+C | Ya · `cmd-c` / `ctrl-shift-c` `terminal.copy` |
| Terminal Panes | Select all terminal text | terminal.selectAll | Cmd+A | Ctrl+Shift+A | Ctrl+Shift+A | Ya · `cmd-a` `terminal.select_all` |
| Terminal Panes | Paste into terminal | terminal.paste | Cmd+V | Ctrl+V · Ctrl+Shift+V · Shift+Ins | Ctrl+V · Ctrl+Shift+V · Shift+Ins | Ya · `cmd-v` / `ctrl-shift-v` / `shift-insert` `terminal.paste` |
| Terminal Panes | Search active pane | terminal.search | Cmd+F | Ctrl+F | Ctrl+F | Ya · `cmd-f` / `ctrl-shift-f` `terminal.search` |
| Terminal Panes | Clear active pane | terminal.clear | — | — | — | Mod · se puede enlazar `send:` con ctrl+l en `keys.json` |
| Terminal Panes | Focus next pane | terminal.focusNextPane | Cmd+] | Ctrl+] | Ctrl+] | Ya · `cmd-alt-→` `pane.go:right` |
| Terminal Panes | Focus previous pane | terminal.focusPreviousPane | Cmd+[ | Ctrl+[ | Ctrl+[ | Ya · `cmd-alt-←` `pane.go:left` |
| Terminal Panes | Equalize pane sizes | terminal.equalizePaneSizes | — | — | — | Núcleo |
| Terminal Panes | Expand / collapse pane | terminal.expandPane | Cmd+Shift+Enter | Ctrl+Shift+Enter | Ctrl+Shift+Enter | Ya · `cmd-shift-enter` `pane.zoom` |
| Terminal Panes | Set Title… | terminal.setTitle | — | — | — | Ya parcial · `tab.rename` |
| Global | Collapse / Expand All Projects | sidebar.projects.toggleCollapse | Cmd+Alt+P | Ctrl+Shift+H | Ctrl+Shift+H | Mod |
| Chat | Find in chat | chat.find | Cmd+F | Ctrl+F | Ctrl+F | No aplica |
| Chat | Switch between Chat UI and terminal | terminal.toggleNativeChat | Cmd+Shift+J | Ctrl+Shift+F11 · Ctrl+Shift+J | Ctrl+Shift+F11 · Ctrl+Shift+J | No aplica |
| Global | Toggle Child Workspaces | sidebar.childWorkspaces.toggle | — | — | — | Mod |
| Terminal Panes | Clear Pane Title | terminal.clearPaneTitle | — | — | — | Núcleo |
| Terminal Panes | Copy Session ID | terminal.copySessionId | — | — | — | Núcleo |
| Terminal Panes | Close active pane | terminal.closePane | Cmd+W | Ctrl+W | Ctrl+W | Ya · `cmd-shift-w` `pane.close` |
| Terminal Panes | Split terminal right | terminal.splitRight | Cmd+D | Ctrl+Shift+D | Ctrl+Shift+D | Ya · `cmd-d` `pane.split:right` |
| Terminal Panes | Split terminal down | terminal.splitDown | Cmd+Shift+D | Alt+Shift+D | Alt+Shift+D | Ya · `cmd-shift-d` `pane.split:down` |
| Terminal Panes | Switch input source / language (native) | terminal.switchInputSource | — | — | — | Núcleo |
| Tabs | Move active tab left | tab.moveLeft | — | — | — | Ya parcial · `tab.move {by: -1}` sin atajo |
| Tabs | Move active tab right | tab.moveRight | — | — | — | Ya parcial · `tab.move {by: 1}` sin atajo |
| Bots | Back to Items | botItem.back | Cmd+[ | Alt+← | Alt+← | Mod · mod `bots` |
| Bots | Previous item | botItem.previous | Cmd+Alt+↑ | Ctrl+Alt+↑ | Ctrl+Alt+↑ | Mod · mod `bots` |
| Bots | Next item | botItem.next | Cmd+Alt+↓ | Ctrl+Alt+↓ | Ctrl+Alt+↓ | Mod · mod `bots` |

## Menús del sistema y de la barra de menú

Fuente: `src/main/menu/register-app-menu.ts` y archivos vecinos. Sunna: `src/macos.rs`.

| Menú o elemento | Contenido en Orca | Plataforma | Sunna |
|---|---|---|---|
| macOS app menu | About, Check for Updates…, Settings (con atajo), Explore Orca, Getting Started, Report Crash…, Services, Hide, Hide Others, Show All, Quit y «Quit Completely…» (detiene todas las sesiones). | macOS | Ya parcial · menú Sunna con What's new, Check for updates, Settings, Services, Hide, Hide Others, Show All, Quit; faltan Explore, Getting Started, Report crash y «Quit completely» |
| File menu | En Windows/Linux solo Settings y Quit. | Windows/Linux | Ya parcial · en macOS tiene New tab/space, split, rename y close; en otros SO el menú no está verificado |
| Edit menu | Undo, Redo, Cut, Copy, Paste (con ruta propia hacia el renderer), Select All; Undo/Redo con atajo solo en macOS. | Todas | Ya parcial · Copy, Paste y Find; faltan Undo, Redo, Cut y Select All |
| Appearance submenu | Toggle Left/Right Sidebar, Show Status Bar, Show Tasks/Automations/Mobile Button, Show Titlebar App Name (casillas que alteran los ajustes). | Todas | Ya parcial · View > Toggle sidebar y submenú Layout; sin casillas de ajustes |
| View menu | Reload, Force Reload, Toggle Developer Tools, Reset Size, Zoom In/Out, Open Worktree Palette, Toggle Full Screen. | Todas | Ya parcial · Layout, Toggle sidebar, Zoom pane, Actual size, Zoom in/out, Command palette; sin Reload, DevTools ni Full screen |
| Window menu | Minimize y Zoom (en Windows/Linux sin acelerador de Ctrl+M para no robárselo a los terminales). | Todas | Ya · Minimize, Zoom, Go to tab, tabs, spaces y agentes siguiente/anterior |
| Help menu | Documentación, Report Crash, About (en Windows/Linux). | Todas | Ya parcial · Keyboard shortcuts, Welcome, Send feedback |
| System tray / menu bar icon | Ícono en la bandeja o barra de menú con atajo y actividad; al cerrar la ventana la app queda activa. | Windows/Linux (bandeja), macOS (barra de menú) | Núcleo |
| Dock icon and titlebar app name | Elección de ícono del Dock y nombre en la barra de título. | Todas / macOS | Núcleo |
| Context menus | Menús contextuales de espacio, proyecto, pestaña y terminal (Open in, Copy path, Reveal, rename…). | Todas | Ya · mod `menu` (Open in, New/Open/Remove worktree, Clean up worktrees, Hide project, Copy path, Show in file manager, Rename, Split, Zoom, Close pane) |

## CLI `orca` (fuera de Ajustes)

Fuente: `src/cli/discovery/root-help-text-primary.ts` y `-secondary.ts`. Tiene más de 200 subcomandos en 27 grupos.

| Grupo | Comandos | Sunna |
|---|---|---|
| Startup | `orca open`, `orca serve` (runtime sin cabeza; `--port`), `orca status`. | Ya · `sunna status` |
| Diagnostics / agent-context | `orca diagnostics memory`, `orca agent-context` (esquema de comandos legible por máquina). | Ya · `sunna commands` |
| References | `reference list\|add\|remove\|find`: enlaza PR e issues a un workspace. | No aplica |
| Agent sessions | `orca search <query>` (texto completo de sesiones, `--index-status`). | Mod |
| Accounts | `account add\|list\|select\|rm` de cuentas de agentes. | No aplica |
| Skills | `skills installed\|share\|list\|get\|install\|update`. | Ya parcial · `sunna --skill` |
| Artifacts | `artifacts`: publicar y gestionar artefactos compartidos. | No aplica |
| Hosts / environments | `host list\|rm`, `environment add\|list\|show\|rm`. | Ya · `hc machine ls` y Machines |
| Plugins (27 subcomandos) | `plugin create\|types\|validate\|inspect\|test\|check\|list\|logs\|install\|update\|uninstall\|catalog\|enable\|disable\|system\|restart\|open\|shot\|dev …`. | Ya parcial · `sunna check`, `mod.clone`, instalar desde Settings |
| Environment recipes | `vm recipe doctor`, `vm env doctor`. | No aplica |
| Automations (7) | `automations list\|show\|create\|edit\|remove\|run\|runs`. | Ya parcial · carpeta `bots/` y el mod |
| Bots (15) | `bot reply\|connectors`, Slack, Google, Gmail, calendario, contactos. | No aplica |
| Projects / repos / worktrees | `project …` (7), `repo list\|add\|show\|set\|hide\|unhide\|set-base-ref\|search-refs`, `worktree list\|show\|activate\|current\|create\|set\|rm\|ps`, `ports`. | Ya parcial · `space.new`, mod `worktree` |
| Workspace cleanup | `workspace cleanup preview\|run\|report`. | Mod |
| Agents / chats / files | `agent launch\|installed\|list\|show\|read\|hooks`, `claude-teams`, `chat rename`, `file open\|diff\|open-changed`. | Ya parcial · `tab.new {argv}`, `agent.focus` |
| Terminals (12) | `terminal list\|show\|read\|send\|wait\|create\|rename\|set-pane-title\|split\|switch\|focus\|close`. | Ya · `terminal.*`, `pane.*`, `sunna do` |
| Orchestration (29) | `orchestration run-*`, `send`, `ask`, `reply`, `inbox`, tareas, gates. | No aplica |
| Computer use (16) | `computer capabilities\|permissions\|list-apps\|click\|scroll\|drag\|…`. | No aplica |
| Linear / Jira / emulator | `linear`, `jira`, `emulator list\|attach\|tap\|type\|gesture\|button\|rotate\|exec\|kill`. | No aplica |
| Browser automation (31) | `tab create\|list\|…`, `goto`, `fill`, `keypress`, `tab profile …`; opciones `--element`, `--url`, `--page`, `--profile`. | No aplica |
| Opciones comunes | Selectores `--repo`, `--worktree`, `--terminal`, `--parent-worktree`, `--no-parent`; `--json`, `--pairing-code`, `--environment`; `terminal send` con `--text --enter --interrupt --wait-submit`; `--for exit --timeout-ms`. | Ya · `sunna do <cmd> --arg`, salida JSON |

## Variables de entorno

Extraídas de `src/**` (solo las que un usuario o un script pueden fijar o leer; se omiten marcadores internos de eventos y tokens).

| Variable | Qué hace | Sunna |
|---|---|---|
| ORCA_USER_DATA_PATH / ORCA_DEV_USER_DATA_PATH / ORCA_E2E_USER_DATA_DIR | Carpeta de datos del perfil. | Ya · `HERDR_APP_HOME` |
| ORCA_TELEMETRY_DISABLED, ORCA_DIAGNOSTICS_DISABLED, DO_NOT_TRACK | Apagan telemetría y diagnósticos por proceso. | Ya · `HERDR_TELEMETRY=0`, `DO_NOT_TRACK=1` |
| ORCA_MACHINE_NAME | Nombre de la máquina. | No aplica |
| ORCA_PORT, ORCA_PAIRING_CODE, ORCA_PAIRING_ADDRESS, ORCA_REMOTE_PAIRING | Servidor `orca serve` y emparejamiento. | No aplica |
| ORCA_APP_EXECUTABLE, ORCA_OPEN_COMMAND, ORCA_CLI_BIN_DIR, ORCA_CLI_COMMAND | Qué binario usa la CLI para abrir la app. | No aplica |
| ORCA_WINDOWS_SHELL, ORCA_TERMINAL_WINDOWS_SHELL, ORCA_REQUIRE_FISH | Shell del terminal. | Ya parcial · shell elegido por el núcleo |
| ORCA_TERMINAL_MEMORY_MAX, ORCA_TERMINAL_MEMORY_SWAP_MAX, ORCA_TERMINAL_PROCESS_CEILING | Topes de recursos de terminales. | No aplica |
| ORCA_UNICODE_VERSION, ORCA_IMAGE_PROTOCOL, ORCA_TERMINAL_IMAGE_VIEWER | Comportamiento de ancho Unicode e imágenes. | No aplica |
| ORCA_BROWSER_EXECUTABLE, ORCA_BROWSER_PARTITION, ORCA_BROWSER_BLANK_URL | Navegador integrado. | No aplica |
| ORCA_CODEX_HOME, ORCA_CODEX_ISOLATE, ORCA_CLAUDE_USER_CONFIG_DIR | Carpetas de configuración de Codex y Claude. | No aplica |
| ORCA_WORKTREE_PATH, ORCA_WORKSPACE_ID, ORCA_WORKSPACE_NAME, ORCA_PROJECT_ID, ORCA_PROJECT_ROOT, ORCA_ROOT_PATH, ORCA_REPO_PATH, ORCA_REPO_URL, ORCA_REPO_BRANCH, ORCA_REPO_REF | Inyectadas en los scripts de setup, archive y recetas. | Mod · el mod `worktree` |
| ORCA_TAB_ID, ORCA_TERMINAL_HANDLE, ORCA_AGENT_SESSION_ID, ORCA_STATUS_AGENT, ORCA_AGENT_HOOK_PORT/TOKEN | Identidad del terminal y ganchos de estado dentro de los agentes. | Ya · etiquetas `herdr.*` y ganchos de estado |
| ORCA_AGENT_HOOKS_DISABLED | Desactiva los ganchos de estado de agentes. | No aplica |
| ORCA_ARTIFACTS_API_URL, ORCA_CLOUD_API_URL, ORCA_CLOUD_CLIENT_ID | Endpoints de nube y artefactos. | No aplica |
| ORCA_ELECTRON_NO_SANDBOX, ORCA_BACKGROUND_LAUNCH, ORCA_ORCAD_IDLE_TIMEOUT_MS, ORCA_SERVE_* | Arranque, sandbox y supervisión del servidor. | No aplica |
| ORCA_E2E_*, ORCA_*_EVAL_*, ORCA_DEV_* | Solo pruebas y desarrollo. | No aplica |

## Archivos de configuración

| Archivo | Contenido | Sunna |
|---|---|---|
| orca.yaml (en el repo) | Config de proyecto compartible: `scripts.setup`, `scripts.archive`, `setupAgentStartupPolicy`, `issueCommand`, `defaultTabs` (title, color, command), `environmentRecipes` y `worktree.sharedDirectories`; con override local no versionado. | Mod · Worktree podría leer un `sunna.yaml` equivalente |
| ~/.orca/keybindings.json | Atajos personalizados compartidos entre perfiles. | Ya · `keys.json` |
| Perfil de usuario (`orca-data.json`, perfiles múltiples de `orca-profiles`) | GlobalSettings, estado de UI, repos, worktree meta, sesión, SSH, automatizaciones; varios perfiles aislados con índice propio. | Ya · `settings.json`, `view.json`; no hay perfiles múltiples (se usa `HERDR_APP_HOME`) |
| ~/.orca/bots/<carpeta> | Hogar de cada bot (`SOUL.md`, memoria, archivos). | Ya · `bots/<name>/` |
| Catálogos y carpetas de plugins | Plugins instalados, catálogos y rutas de desarrollo. | Ya · `mods/` y `mod-store/` |
| Temas, mascotas y sonidos personalizados | Temas YAML, mascotas propias, archivo de sonido. | Ya · `themes/`, `sounds.done/blocked` |
| Configuración MCP del proyecto | Se inspecciona en la página del proyecto. | No aplica |

## Lo que no estaba en la lista previa

La lista previa (`docs/orca-configuracion.md`) cubría ocho áreas de forma resumida. Esto es lo que este inventario agrega.

**Páginas y sub-páginas que no tenían fila propia**
- Chat UI completo: apariencia (contraste, fuente, tamaños, ancho) y comportamiento (cola de mensajes, reanudar tras reinicio, continuar tras límite de uso, detener agentes inactivos, vistas previas, mensajes nuevos, entorno de shell, visuales en línea, diffs por turno, PR y CI). Todo No aplica, pero queda documentado.
- Páginas de proveedores en AI Provider Accounts (Codex, Claude, Gemini, OpenCode Go, MiniMax, Grok, Antigravity, Cursor, GLM/ZCode), Orchestration, Computer Use, Voice (dictado), Orca Account, Onboarding checklist, Integrations (GitHub, GitLab, Bitbucket, Azure DevOps, Gitea, Jira, Linear), Mobile, macOS Permissions, Remote Orca servers, Dev Tools, Artifacts, Share Skills, Agent Session Search, Task Sources, Quick Commands, Browser (con perfiles, extensiones, permisos de sitios, identidad), Mobile Emulator y Floating Workspace.

**Ajustes ocultos tras secciones plegables o fuera de Ajustes**
- Estado de la barra lateral: agrupar por (none, estado, repo, PR), ordenar por (nombre, inteligente, reciente, repo, manual), mostrar solo activos, ocultar dormidos/inactivos, filtros por origen (automatización, CLI, HEAD desacoplado, otros equipos, bots), filtro por proyecto y por host, orden de proyectos, grupos plegados, ancho de paneles.
- Vista Agents: filtro de leídos, agrupar por, modo compacto, buscador, mostrar agentes hijos.
- Tablero de estados de workspaces (estados con color e ícono, opacidad, ancho de columna) y `syncTaskStatusFromWorkspaceBoard`.
- Estilo y propiedades de las tarjetas de espacio (12 propiedades), tinte de la barra lateral (color y opacidad), posición de la barra de actividad, orden y ocultamiento de sus íconos.
- Los 15 elementos de la barra de estado, el formato de uso, porcentaje usado/restante y modo compacto/verboso.
- Confirmaciones individuales (cerrar pestaña fijada, cerrar terminal en ejecución, borrar worktree, automatización, chat, artefacto, reinicio de límite de Codex) y las dos de «forzar borrado».
- Ajustes de rendimiento de terminales (parking de vistas ocultas, presupuestos de retención) y de red (proxy, reglas de exclusión, espejo de Node, HTTP/1.1).

**Ajustes por proyecto (repositorio)**
- Scripts de setup y archive, política de ejecución del setup, política de arranque del agente, origen de comandos (compartido, local o ambos), pestañas por defecto, rutas compartidas o enlazadas (`symlinkPaths`), directorios compartidos, presets de sparse checkout, base y ubicación de worktrees, visibilidad de worktrees externos y de agentes (con fuentes propias), sincronización de forks, confianza del agente, cuenta de GitHub, fuente de issues, proyecto Jira, perfil de navegador, runtime (Windows o WSL), reglas de limpieza, términos de dictado, ocultar proyecto, formateador del editor, anulaciones de IA de control de código.
- El archivo `orca.yaml` del repositorio (compartible y versionado) con su override local.

**Por agente, por automatización y por bot**
- Por agente: habilitar, comando, argumentos, variables de entorno, opciones de lanzamiento, permiso propio, agente por defecto y un atajo «New X tab» por cada uno.
- Por automatización: agente, superficie, horario y zona horaria, disparadores por evento, espera única, precheck con tiempo límite, modo de workspace, rama base, política de corridas perdidas con gracia, destino local o SSH.
- Por bot: agente, permisos, modelo, notificaciones (3), conectores, quién puede pedirle cosas, Look around, Tidy up, caducidad de pedidos, memoria, skills, herramientas, acceso a Google, espejo de Slack.

**Atajos, accesibilidad y menús**
- Catálogo completo de 103 comandos con su atajo en macOS, Linux y Windows (arriba), más atajos de doble toque de modificador, política en terminal, aviso de conflictos con atajos del sistema de macOS y distinción entre campos de texto y chat.
- Menús: menú de la app, File, Edit, Appearance, View, Window y Help (con casillas que alteran ajustes), ícono de la bandeja o barra de menú, y los menús contextuales.
- Accesibilidad: Orca no tiene una página de accesibilidad. Lo más cercano son «Reduce visual effects», el contraste del terminal y del chat, el contraste mínimo, el zoom de interfaz y del editor, la fuente y el escalado, los subrayados ortográficos y la edición con teclado Option. Todo está listado en Appearance, Chat UI, Input & Editing y Terminal.

**Fuera de Ajustes**
- CLI `orca` (más de 200 subcomandos en 27 grupos), decenas de variables de entorno con efecto real, `orca.yaml`, `~/.orca/keybindings.json`, perfiles de usuario múltiples, plugins con catálogos y rutas de desarrollo.

## Los diez ajustes de mayor valor que Sunna no tiene

Criterio: uso diario probable, coste bajo en Sunna y encaje con la regla de núcleo pequeño. Entre paréntesis, dónde caería.

1. **Ajustes por proyecto con scripts de setup/archive, rutas enlazadas y pestañas por defecto** (equivalente de `orca.yaml`): un `sunna.yaml` o `projects.json` que lee el mod `worktree`. Mod.
2. **Limpieza automática de worktrees** (fusionados, sin cambios, inactivos N días, de automatizaciones) con registro de actividad: hoy solo existe limpieza manual. Mod.
3. **Agente por defecto y atajo «nuevo agente»** (`tab.newAgent`, más un atajo por agente): hoy cada menú pregunta el agente. Núcleo (un campo de `Person`) o Mod si solo vive en el menú.
4. **Terminal: forma, parpadeo y opacidad del cursor, ocultar puntero al escribir, volver al final al escribir y foco que sigue al ratón.** Son ajustes cotidianos y hoy no hay equivalente. Núcleo.
5. **Notificaciones con matices:** campana del terminal, volumen, silenciar por espacio o por máquina, avisos de espacios ocultos y sonido de prueba por tipo. Núcleo (campana y volumen) y Mod (silenciar).
6. **Filtros y agrupación de la barra lateral:** agrupar por repo/estado, ordenar (reciente, manual), mostrar solo activos, ocultar dormidos y espacios creados por bots o por la CLI. Mod (`spaces`).
7. **Atajos que faltan del catálogo:** reabrir pestaña cerrada, historial atrás/adelante de espacios, igualar tamaños de paneles, limpiar terminal, mover pestaña con teclado, enfocar la lista de espacios. Núcleo para los cuatro primeros, Mod el resto.
8. **Comandos rápidos** (comandos de terminal guardados, globales o por proyecto, con menú y atajo). Mod.
9. **Retraso de actualizaciones y canal** (0 a 90 días de espera, estable/RC), útil para equipos. Núcleo (el actualizador).
10. **Proxy de red y reglas de exclusión** para la app y los terminales locales, además de la confirmación por tipo de acción destructiva (pinned tab, terminal en ejecución, borrar espacio). Núcleo (proxy) y Mod (confirmaciones).

Menciones: variables de entorno por agente (Núcleo), diseño del menú «+» de pestañas (Mod, ya existe `new-menu`), opacidad del panel inactivo y relleno ya existen parcialmente en el layout, e importar de Ghostty/Kitty más allá de los temas (Mod).
