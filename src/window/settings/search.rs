//! Settings search: where each setting lives and how a query ranks them
//! (title, then description, then keywords).

use crate::i18n;

/// The window's sections in groups, in the order the sidebar shows them.
pub const GROUPS: &[(&str, &[&str])] = &[
    ("settings.group_app", &["look", "general", "terminal", "editor", "git", "keyboard", "alerts"]),
    ("settings.group_extend", &["mods", "agents", "machines"]),
    ("settings.group_system", &["updates", "privacy"]),
];

/// Locale key of each section's title.
pub fn section_title(id: &str) -> &'static str {
    match id {
        "look" => "settings.look",
        "terminal" => "settings.term_title",
        "editor" => "settings.editor_title",
        "mods" => "settings.mods",
        "general" => "settings.general",
        "git" => "settings.orca_git",
        "agents" => "settings.agents",
        "keyboard" => "settings.keyboard",
        "machines" => "settings.machines",
        "alerts" => "settings.alerts",
        "updates" => "settings.updates",
        _ => "settings.privacy",
    }
}

/// `(section, title key, about key or "", extra words)`.
pub const SETTINGS: &[(&str, &str, &str, &str)] = &[
    ("look", "settings.theme", "", "tema color dark light"),
    ("look", "settings.appearance", "settings.light_or_dark_or_as", "claro oscuro dark light"),
    ("look", "settings.size_and_fonts", "", "zoom tamaño"),
    ("terminal", "settings.terminal_font", "", "font monospace"),
    ("terminal", "settings.terminal_font_weight", "", "bold negrita grosor"),
    ("terminal", "settings.terminal_line_height", "", "interlineado"),
    ("terminal", "settings.terminal_cell_width", "", "ancho celda"),
    ("terminal", "settings.terminal_scroll_speed", "", "scroll"),
    ("look", "settings.interface_font", "", "font tipografía"),
    ("general", "settings.language", "settings.language_about", "idioma español english"),
    ("general", "settings.resume_agents", "", "restaurar sesión"),
    ("general", "settings.folder_roots", "", "carpetas buscador"),
    ("agents", "settings.full_access", "settings.full_access_about", "yolo permisos"),
    ("alerts", "settings.play_sounds", "settings.when_an_agent_needs_you", "sonido audio"),
    ("alerts", "settings.an_agent_needs_you", "", "sonido bloqueado"),
    ("alerts", "settings.an_agent_finished", "", "sonido terminó"),
    ("machines", "settings.this_computer", "", "ssh remoto"),
    ("terminal", "settings.term_ligatures", "settings.term_ligatures_about", "ligaduras fuente font ligatures fira"),
    ("terminal", "settings.term_contrast", "settings.term_contrast_about", "contraste legibilidad minimum contrast"),
    ("terminal", "settings.term_opacity", "settings.term_opacity_about", "transparencia fondo opacidad transparent"),
    ("terminal", "settings.term_padding_x", "", "relleno margen padding horizontal"),
    ("terminal", "settings.term_padding_y", "", "relleno margen padding vertical"),
    ("terminal", "settings.term_scrollbars", "settings.term_scrollbars_about", "barra desplazamiento scrollbar"),
    ("terminal", "settings.term_copy_on_select", "settings.term_copy_on_select_about", "copiar seleccionar portapapeles copy"),
    ("terminal", "settings.term_osc52", "settings.term_osc52_about", "osc 52 portapapeles clipboard remoto"),
    ("terminal", "settings.term_scroll_on_input", "settings.term_scroll_on_input_about", "bajar final escribir scroll teclear"),
    ("terminal", "settings.term_option", "settings.term_option_about", "option alt mac teclado"),
    ("alerts", "settings.notif_enabled", "settings.notif_enabled_about", "notificaciones avisos toast"),
    ("alerts", "settings.notif_done", "settings.notif_done_about", "notificaciones agente terminó listo"),
    ("alerts", "settings.notif_blocked", "settings.notif_blocked_about", "notificaciones agente necesita entrada bloqueado"),
    ("alerts", "settings.notif_test", "settings.notif_test_about", "probar prueba notificación enviar"),
    ("editor", "settings.editor_font_size", "settings.editor_font_size_about", "editor visor archivos fuente tamaño"),
    ("editor", "settings.editor_line_numbers", "", "editor visor números línea"),
    ("editor", "settings.editor_folding", "settings.editor_folding_about", "editor visor plegar código"),
    ("editor", "settings.editor_wrap", "settings.editor_wrap_about", "editor visor ajuste línea wrap"),
    ("look", "settings.orca_card_git", "settings.orca_card_git_about", "tarjeta espacio card rama branch"),
    ("look", "settings.orca_card_ago", "settings.orca_card_ago_about", "tarjeta espacio tiempo hace"),
    ("look", "settings.orca_card_agents", "settings.orca_card_agents_about", "tarjeta espacio agentes"),
    ("look", "settings.orca_button_search", "settings.orca_buttons_about", "botones barra lateral sidebar buscar"),
    ("look", "settings.orca_button_bots", "", "botones barra lateral sidebar bots automatizaciones"),
    ("look", "settings.orca_status_machine", "", "barra de estado status equipo"),
    ("look", "settings.orca_status_branch", "", "barra de estado status rama"),
    ("look", "settings.orca_status_agents", "", "barra de estado status agentes"),
    ("look", "settings.orca_status_awake", "", "barra de estado status taza despierto"),
    ("look", "settings.orca_status_hint", "", "barra de estado status atajo paleta"),
    ("look", "settings.orca_status_format", "settings.orca_status_format_about", "barra de estado status formato uso agentes"),
    ("look", "settings.orca_tab_order", "settings.orca_tab_order_about", "pestañas recientes tabs rueda orden"),
    ("look", "settings.orca_ignored", "settings.orca_ignored_about", "archivos gitignore ocultos ignorados"),
    ("general", "settings.orca_worktrees_dir", "settings.orca_worktrees_dir_about", "worktrees carpeta directorio espacios"),
    ("general", "settings.orca_nest", "settings.orca_nest_about", "worktrees carpeta proyecto subcarpeta"),
    ("general", "settings.orca_external", "settings.orca_external_about", "worktrees externos fuera"),
    ("general", "settings.orca_files_open", "settings.orca_files_open_about", "archivos vista previa preview pestañas clic doble"),
    ("general", "settings.orca_open_in", "settings.orca_open_in_about", "abrir en apps editor vscode cursor menú"),
    ("git", "settings.orca_prefix", "settings.orca_prefix_about", "rama branch prefijo worktree"),
    ("git", "settings.orca_prefix_text", "settings.orca_prefix_text_about", "rama branch prefijo personalizado"),
    ("git", "settings.orca_template", "settings.orca_template_about", "rama branch plantilla nombre worktree"),
    ("git", "settings.orca_fresh", "settings.orca_fresh_about", "main actualizado fetch origin rama base"),
    ("git", "settings.orca_branch_names", "settings.orca_branch_names_about", "renombrar espacio rama nombre automático"),
    ("git", "settings.orca_base", "settings.orca_base_about", "comparar base cambios upstream diff"),
    ("git", "settings.orca_groups", "settings.orca_groups_about", "cambios grupos orden control de código source control"),
];

const SECTIONS: &[&str] = &["look", "terminal", "editor", "mods", "general", "git", "agents", "keyboard", "machines", "alerts", "updates", "privacy"];

#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub section: &'static str,
    pub title: String,
    pub about: String,
    pub score: u32,
}

/// Lowercase without accents (what NFD plus stripping marks does to es/en text).
pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' | 'ã' | 'å' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'õ' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

/// How well `text` matches `q` (already folded): exact 4, prefix 3, word start 2, substring 1, none 0.
fn level(text: &str, q: &str) -> u32 {
    let t = fold(text);
    if t.is_empty() {
        return 0;
    }
    if t == q {
        return 4;
    }
    if t.starts_with(q) {
        return 3;
    }
    if t.split(|c: char| c.is_whitespace() || matches!(c, '/' | '_' | '-')).any(|w| w.starts_with(q)) {
        return 2;
    }
    u32::from(t.contains(q))
}

/// The entries `query` finds, best first, at most `limit`.
pub fn rank(query: &str, limit: usize) -> Vec<Hit> {
    let q = fold(query);
    let q = q.trim();
    if q.is_empty() {
        return Vec::new();
    }
    let sections = SECTIONS.iter().map(|s| (*s, section_title(s), "", "", true));
    let entries = sections.chain(SETTINGS.iter().map(|&(s, t, a, w)| (s, t, a, w, false)));
    let mut scored = Vec::new();
    for (section, title, about, words, section_hit) in entries {
        let on_title = level(&i18n::t(title), q);
        let on_about = if about.is_empty() { 0 } else { level(&i18n::t(about), q) };
        let on_words = level(words, q);
        let score = [
            if on_title > 0 { (if section_hit { 800 } else { 600 }) + on_title * 25 } else { 0 },
            if on_about > 0 { 400 + on_about * 25 } else { 0 },
            if on_words > 0 { 200 + on_words * 25 } else { 0 },
        ]
        .into_iter()
        .max()
        .unwrap_or(0);
        if score > 0 {
            let section = SECTIONS.iter().copied().find(|s| *s == section).unwrap_or("look");
            scored.push(Hit { section, title: i18n::t(title), about: if about.is_empty() { String::new() } else { i18n::t(about) }, score });
        }
    }
    // Stable, like Array.sort: equal scores keep their order.
    scored.sort_by(|a, b| b.score.cmp(&a.score));
    scored.truncate(limit);
    scored
}

#[cfg(test)]
mod tests {
    use super::{SETTINGS, fold, rank};
    use crate::i18n;

    #[test]
    fn sections_are_found_by_title_first() {
        let hits = rank("terminal", 12);
        assert_eq!(hits[0].section, "terminal");
        assert_eq!(hits[0].title, i18n::t("settings.term_title"));
    }

    #[test]
    fn accents_and_case_do_not_matter() {
        assert_eq!(fold("Teclá DO"), "tecla do");
        assert!(rank("IDIOMA", 12).iter().any(|h| h.section == "general"));
    }

    #[test]
    fn title_outranks_description_outranks_words() {
        let hits = rank("worktree", 12);
        assert!(hits.windows(2).all(|w| w[0].score >= w[1].score));
        assert!(rank("zzzz nothing", 12).is_empty());
        assert!(rank("   ", 12).is_empty());
    }

    #[test]
    fn at_most_limit() {
        assert!(rank("a", 5).len() <= 5);
    }

    fn locale(name: &str) -> serde_json::Value {
        let path = format!("{}/defaults/locales/{name}.json", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn every_entry_is_in_a_section_with_texts_in_both_languages() {
        let (es, en) = (locale("es"), locale("en"));
        for &(section, title, about, _) in SETTINGS {
            assert!(super::SECTIONS.contains(&section), "{title}: no section {section}");
            for key in [title, about].into_iter().filter(|k| !k.is_empty()) {
                assert!(es.get(key).is_some() && en.get(key).is_some(), "{key}");
            }
        }
    }

    #[test]
    fn every_orca_text_the_pages_use_exists_in_both_languages() {
        let (es, en) = (locale("es"), locale("en"));
        let mut used = std::collections::BTreeSet::new();
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/window/settings");
        for file in std::fs::read_dir(dir).unwrap().flatten() {
            let text = std::fs::read_to_string(file.path()).unwrap();
            for part in text.split("\"settings.orca_").skip(1) {
                if let Some(end) = part.find('"') {
                    if end > 0 && part[..end].chars().all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit()) {
                        used.insert(format!("settings.orca_{}", &part[..end]));
                    }
                }
            }
        }
        assert!(used.len() > 40, "{}", used.len());
        for key in &used {
            assert!(es.get(key).is_some() && en.get(key).is_some(), "{key}");
        }
    }

    #[test]
    fn terminal_editor_and_alert_settings_live_in_their_own_pages() {
        let at = |title: &str| SETTINGS.iter().find(|e| e.1 == title).map(|e| e.0);
        for (title, section) in [("settings.term_opacity", "terminal"), ("settings.term_osc52", "terminal"), ("settings.terminal_font", "terminal"), ("settings.notif_test", "alerts"), ("settings.editor_wrap", "editor")] {
            assert_eq!(at(title), Some(section), "{title}");
        }
    }

    #[test]
    fn the_worktree_settings_are_found_in_git() {
        assert!(rank("worktree", 12).iter().any(|h| h.section == "git"));
    }
}
