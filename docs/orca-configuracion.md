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
| Ícono de la app | Can |
| Reducir efectos visuales | Can |
| Mostrar archivos ignorados por Git | Mod (Archivos) |
| Nombre de la app en la barra de título | Can |
| Apariencia de la barra lateral izquierda | Mod (ya configurable) |
| Diseño de la tarjeta de espacio | Mod (Espacios) |
| Mostrar también worktrees fijados en su lista | Mod (Espacios) |
| Botones Tareas / Automatizaciones / Móvil | Mod (barra lateral) |
| Formato de uso en la barra de estado | Mod (Estado) |
| Minimizar a la bandeja al cerrar, ícono de la barra de menú | Can |
| Orden de pestañas recientes | Mod (Pestañas) |

## General
| Ajuste de Orca | Estado |
|---|---|
| Carpeta de espacios de trabajo | Mod (Worktree) |
| Worktrees externos | Mod (Worktree) |
| Carpeta por proyecto | Mod (Worktree) |
| Abrir en apps (menú «Abrir en») | Mod (menú de proyecto) |
| Pestañas de vista previa | Mod (Archivos) |
| Confirmaciones (cerrar, eliminar) | Can para las del núcleo; Mod para las de mods |
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
| Forma, parpadeo y opacidad del cursor | Can |
| Opacidad del fondo y del panel inactivo | Can |
| Grosor del divisor | Can |
| Relleno horizontal y vertical | Can |
| Aceleración por GPU | Can |
| Foco sigue al ratón | Can |
| Ocultar ratón al escribir | Can |
| Bajar al final al escribir | Can |
| Preguntar antes de cerrar terminales activas | Can |
| Shell predeterminado | Can |
| Importar de Ghostty / Kitty | Mod (lee los archivos de configuración) + Can para aplicar |
| Pegar con clic central | Can |
| Recortar margen al copiar | Can |

## Notificaciones
| Ajuste de Orca | Estado |
|---|---|
| Sonidos (activar, volumen, sonido) | Ya (`sounds`) |
| Agente terminó / necesita entrada | Ya (Alertas) |
| Campana del terminal | Can |
| Suprimir con la ventana en foco | Can |
| Notificación de prueba | Mod |
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
