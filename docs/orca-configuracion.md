# Configuración de Orca frente a Sunna

Fuente: `stablyai/orca`, `src/renderer/src/components/settings/` (MIT; solo se replica la idea).
Estado: **Ya** = Sunna lo tiene (`settings.json` / Ajustes). **Mod** = lo puede hacer un mod con su almacenamiento compartido, sin tocar el núcleo. **Can** = necesita el núcleo (`AGENTS.md`: consultar antes). **No aplica** = depende de algo que Sunna no tiene.

Clasificación hecha leyendo el código de Sunna; los **Mod** y **Can** se confirman al implementar cada ajuste.

## Apariencia
| Ajuste de Orca | Estado |
|---|---|
| Tema, tema del editor, colores | Ya (`theme`, `colors`) |
| Idioma | Ya (`language`) |
| Zoom de la interfaz | Ya (`zoom`) |
| Fuente de la interfaz | Ya (`ui_font`) |
| Contraste | Ya (`minimum_contrast`) |
| Ícono de la app | Can (pendiente: no entra en este ítem) |
| Reducir efectos visuales | Ya (`reduce_effects`: sin sombras, vidrio ni terminales translúcidos) |
| Mostrar archivos ignorados por Git | Mod (Archivos) |
| Nombre de la app en la barra de título | Ya (`show_app_name`; solo macOS con barra del sistema; Linux/Windows ya muestran el título) |
| Apariencia de la barra lateral izquierda | Mod (ya configurable) |
| Diseño de la tarjeta de espacio | Mod (Espacios) |
| Mostrar también worktrees fijados en su lista | Mod (Espacios) |
| Botones Tareas / Automatizaciones / Móvil | Mod (barra lateral) |
| Formato de uso en la barra de estado | Mod (Estado) |
| Minimizar a la bandeja al cerrar, ícono de la barra de menú | Límite (GPUI no ofrece ícono de bandeja ni de barra de menús; sin dependencias nuevas no se puede) |
| Orden de pestañas recientes | Mod (Pestañas) |

## General
| Ajuste de Orca | Estado |
|---|---|
| Carpeta de espacios de trabajo | Mod (Worktree) |
| Worktrees externos | Mod (Worktree) |
| Carpeta por proyecto | Mod (Worktree) |
| Abrir en apps (menú «Abrir en») | Mod (menú de proyecto) |
| Pestañas de vista previa | Mod (Archivos) |
| Confirmaciones (cerrar, eliminar) | Ya para cerrar terminales activas (`confirm_close_running`) y espacios; Mod para las de mods |
| Buscar actualizaciones | Ya (Actualizaciones) |
| Acerca de | Mod |

## Terminal
| Ajuste de Orca | Estado |
|---|---|
| Fuente, tamaño, peso, ligaduras, alto de línea | Ya (`font*`, `terminal_line_height`) |
| Velocidad de desplazamiento | Ya (`terminal_scroll_speed`) |
| Copiar al seleccionar | Ya (`copy_on_select`) |
| Escritura de portapapeles de TUI (OSC 52) | Ya (`clipboard_write`) |
| Contraste de color | Ya (`minimum_contrast`) |
| Option como Alt | Ya (`option_as_alt`) |
| Forma, parpadeo y opacidad del cursor | Ya (`cursor_shape`, `cursor_blink`, `cursor_opacity`) |
| Opacidad del fondo y del panel inactivo | Ya (`terminal_opacity`, `inactive_pane_opacity`) |
| Grosor del divisor | Ya (`divider_width`) |
| Relleno horizontal y vertical | Ya (`terminal_padding`) |
| Aceleración por GPU | Límite (GPUI usa Metal/wgpu sin interruptor público; el renderizador la elige) |
| Foco sigue al ratón | Ya (`focus_follows_mouse`) |
| Ocultar ratón al escribir | Ya en macOS (`hide_mouse_on_type`); Límite en Linux/Windows (GPUI no tiene `CursorStyle::None`) |
| Bajar al final al escribir | Ya (`scroll_on_input`) |
| Preguntar antes de cerrar terminales activas | Ya (`confirm_close_running`) |
| Shell predeterminado | Ya parcial (`shell`: solo terminales que lanza la app en este equipo; el core no tiene ajuste propio) |
| Importar de Ghostty / Kitty | Mod (lee los archivos de configuración) + Can para aplicar |
| Pegar con clic central | Ya (`middle_click_paste`) |
| Recortar margen al copiar | Ya (`trim_copy_margin`) |

## Notificaciones
| Ajuste de Orca | Estado |
|---|---|
| Sonidos (activar, volumen, sonido) | Ya (`sounds`) |
| Agente terminó / necesita entrada | Ya (Alertas) |
| Campana del terminal | Ya (`notifications.bell`) |
| Suprimir con la ventana en foco | Ya (`notifications.suppress_when_focused`) |
| Notificación de prueba | Ya (Alertas > Enviar) |
| Disco casi lleno, pestañas ocultas | No aplica |

## Agentes, chat y automatización
| Ajuste de Orca | Estado |
|---|---|
| Reanudar agentes tras reinicio | Ya (`resume_agents`) |
| Modo sin confirmación (yolo) | Ya (`yolo`) |
| Agente predeterminado, lista de agentes | Ya (Agentes) |
| Detener agentes inactivos tras N tiempo | Mod (Agentes) |
| Suspensión de agentes | Mod (Agentes) |
| Panel de agentes | Ya (mod `agents`) |
| Automatizaciones | Ya (mod `bots`) |
| Atención del terminal | Mod (Actividad) |
| Cuentas Codex / Gemini / Cursor / etc. | No aplica |
| Chat (todas sus opciones) | No aplica: Sunna no tiene interfaz de chat |
| Orquestación de agentes | No aplica |
| Voz / dictado | No aplica |

## Control de código fuente y espacios
| Ajuste de Orca | Estado |
|---|---|
| Prefijo de rama, plantilla de nombre | Mod (Worktree) |
| Renombrar rama y espacio automáticamente | Mod (Worktree) |
| Base de comparación predeterminada | Mod (Cambios) |
| Mantener `main` local al día | Mod (Worktree) |
| Orden de grupos de control de código | Mod (Cambios) |
| Limpieza de espacios (merged, sin cambios, automatizaciones) | Ya (limpieza de worktrees) |
| Presupuesto de API de GitHub / GitLab | No aplica |
| Proveedores de tareas | No aplica |

## Atajos
| Ajuste de Orca | Estado |
|---|---|
| Lista y grabación de atajos | Ya (Teclado) |
| Atajos por defecto de Orca | Pendiente: en curso (`keys.macos.json`) |
| Política de atajos en el terminal | Ya (`terminal` en `keys.json`) |
| Comandos rápidos | Mod |
| Terminal flotante | Can |

## Conexión, seguridad y avanzado
| Ajuste de Orca | Estado |
|---|---|
| Máquinas / servidores remotos / SSH | Ya (Máquinas) |
| WSL, VM en la nube | No aplica en macOS |
| Privacidad y telemetría | Ya (`telemetry`) |
| Diagnósticos | Mod |
| Permisos de desarrollador | No aplica |
| Mantener despierto | Ya (`keep_awake`) |
| Navegador integrado (memoria, permisos, extensiones) | No aplica: Sunna no tiene navegador |
| Emulador móvil, Orca Mobile | No aplica |
| Plugins | Ya (Mods) |
| Mascota, estadísticas | Mod (opcional) |
