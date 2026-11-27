# Barra lateral de Orca: qué se puede hacer

Fuente: `stablyai/orca`, `src/renderer/src/components/sidebar/` y `src/shared/workspace-view/workspace-view-options.ts` (MIT; solo se replica la idea).

## Cómo se agrupa y ordena la lista de worktrees

Menú de opciones (icono de la lista), con secciones **Show / Group by / Sort / Filters**.

| Opción | Valores |
|---|---|
| Agrupar por | Ninguno, Estado (Todo, En progreso, En revisión, Hecho y los personalizados), PR (por estado de la PR), Proyecto |
| Ordenar por | Nombre, Actividad de agentes (los que necesitan atención primero, luego lo más reciente), Reciente, Proyecto, Manual (arrastrar dentro de cada grupo) |
| Orden de proyectos | Manual (arrastrar) o Reciente (última actividad) |
| Diseño de tarjeta | Detallado o Compacto; estilo clásico o experimental |
| Actividad de agentes | Compacta o Lista completa |
| Propiedades visibles | Ticket de GitHub, Linear, Jira, enlace PR/MR, Automatización, Notas, Puertos, Actividad de agentes, Host, Rama |

Estructura de la lista (de arriba abajo):
1. Sección **Fijados** (Pinned), con sus worktrees de cualquier proyecto.
2. Secciones por **host** (local, SSH, WSL, servidor) cuando hay varios; cada una con contador.
3. Dentro, encabezados del criterio de agrupación; con «Proyecto», un encabezado por proyecto (color e icono propios).
4. **Grupos de proyectos** definidos por el usuario (crear desde un proyecto, mover, quitar, renombrar, borrar, refrescar).
5. **Linaje**: worktrees hijos anidados bajo su padre con guías; «Abrir padre», «Quitar del padre».
6. Sección plegada **Proyectos ocultos**.
7. Avisos (escaneo no disponible, distro WSL detenida) y filas pendientes de creación.

Filtros: ocultar en reposo (sleeping), conservar rama por defecto, ocultar rama por defecto, ocultar creados por automatización, ocultar creados por bots, filtro por proyecto (multiselección con buscador), alcance por host. Insignia con el número de filtros activos y «Limpiar filtros».

Atajos de la lista: plegar/desplegar todos los proyectos (con atajo), plegar una sección, «Ir al espacio actual» (scroll al activo), arrastrar worktrees, proyectos, grupos y hosts; soltar un worktree sobre un estado lo cambia.

## Navegación superior de la barra

Buscar (comandos, bots, espacios, pestañas del navegador), Tareas, Artefactos, Skills, Automatizaciones, Panel de agentes, Orca Móvil (con «Ocultar de la barra lateral»), y la sección Bots con su propia división.

## Menú contextual de un worktree (clic derecho; con varios seleccionados aplica a todos)

- Dormir (y dormir con subárbol; ciclo de vida de VM en remotos), iniciar host WSL.
- Marcar leído / no leído, silenciar notificaciones.
- Mover a estado / mover estados a.
- Pasar a un bot.
- Abrir en ▸ (apps instaladas, «Personalizar apps…»; local, SSH, WSL).
- Copiar ▸ (Ruta, Nombre del worktree).
- Padre e hijos ▸ (Abrir padre), Quitar del padre.
- Grupo de proyecto ▸ (Nuevo grupo desde el proyecto, Mover a grupo, Quitar del grupo).
- Editar detalles…, Fijar / Desfijar.
- Eliminar (Quitar espacio, Quitar proyecto de Orca, Eliminar worktree), con diálogos de cambios sin guardar, ramas preservadas y borrado en cascada de hijos.
- Alt + clic derecho: menú Desarrollador (Aparcar terminal).

## Encabezados

- **Proyecto:** Acciones del proyecto (Ajustes del proyecto, Cambiar icono, Nuevo grupo desde el proyecto, Mover a grupo, Quitar del grupo, Ocultar de la barra, Quitar proyecto) y botón de crear espacio.
- **Grupo de proyectos:** Refrescar proyectos, Renombrar grupo, Eliminar grupo.
- **Host:** Re-emparejar, Iniciar distro, Renombrar, Desconectar, Comprobar conexión, Administrar host, Quitar host; avisos «actualizar servidor/cliente».
- **Añadir:** proyecto desde carpeta, clonar, crear; host por SSH (con `ssh config`), WSL o servidor.

## Pie

Configuración y menú Ayuda; tablero de espacios; diálogo de comentarios con imágenes adjuntas.

## Frente a Sunna (hoy)

Ya existe: proyectos plegables, ocultos, menú de proyecto (nuevo worktree, abrir carpeta, copiar ruta, ocultar), segunda línea de worktree, Bots, buscador, Configuración, agregar máquina, fijar worktrees.

Falta (candidatos, cada uno ítem para Can si toca el núcleo): agrupar por estado/PR, ordenar (agentes, reciente, manual), filtros y su insignia, grupos de proyectos, linaje padre/hijo, sección Fijados, secciones por host, menú de worktree completo (dormir, leído/no leído, mover a estado, abrir en…, copiar, editar detalles), plegar todo, ir al espacio actual, arrastrar para reordenar, menú de host, cambiar icono de proyecto.
