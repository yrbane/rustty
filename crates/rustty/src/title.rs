//! Titres : gabarit des onglets et titre de la fenêtre (avec la version).

pub fn tab_title(template: &str, index: usize, title: &str) -> String {
    let title = if title.is_empty() { "shell" } else { title };
    template
        .replace("{index}", &(index + 1).to_string())
        .replace("{title}", title)
}

/// Le titre affiché d'un onglet : en cours d'édition, le tampon et un
/// curseur ; sinon le gabarit, avec le nom choisi à la place du titre du shell.
pub fn display_title(
    template: &str,
    index: usize,
    shell_title: &str,
    custom: Option<&str>,
    editing: Option<&str>,
) -> String {
    match editing {
        Some(buffer) => format!("{buffer}▌"),
        None => tab_title(template, index, custom.unwrap_or(shell_title)),
    }
}

/// Titre de la fenêtre : le nom d'onglet personnalisé s'il y en a un, sinon
/// le titre du terminal, puis « rustty » et sa version.
pub fn window_title(custom: Option<&str>, term_title: &str, version: &str) -> String {
    let title = custom.filter(|c| !c.is_empty()).unwrap_or(term_title);
    if title.is_empty() {
        format!("rustty {version}")
    } else {
        format!("{title} — rustty {version}")
    }
}

/// Le texte de la barre de titre et de la fenêtre : en cours d'édition du
/// nom de fenêtre, le tampon et un curseur ; sinon le nom fixe de la
/// fenêtre, à défaut le nom de l'onglet actif, à défaut le titre du shell,
/// avec la version.
pub fn bar_title(
    window_name: Option<&str>,
    tab_name: Option<&str>,
    shell_title: &str,
    editing: Option<&str>,
    version: &str,
) -> String {
    match editing {
        Some(buffer) => format!("{buffer}▌"),
        None => window_title(window_name.or(tab_name), shell_title, version),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_title_replaces_the_shell_title() {
        assert_eq!(
            display_title("{index}: {title}", 1, "bash", Some("logs"), None),
            "2: logs"
        );
        assert_eq!(
            display_title("{index}: {title}", 1, "bash", None, None),
            "2: bash"
        );
    }

    #[test]
    fn editing_shows_the_buffer_with_a_cursor() {
        assert_eq!(
            display_title("{index}: {title}", 0, "bash", Some("x"), Some("nouv")),
            "nouv▌"
        );
        assert_eq!(
            display_title("{index}: {title}", 0, "bash", None, Some("")),
            "▌"
        );
    }

    #[test]
    fn tab_title_fills_the_template() {
        assert_eq!(tab_title("{index}: {title}", 0, "vim"), "1: vim");
        assert_eq!(tab_title("[{title}]", 3, ""), "[shell]");
        assert_eq!(tab_title("sans gabarit", 0, "x"), "sans gabarit");
    }

    #[test]
    fn window_title_carries_the_version() {
        assert_eq!(window_title(None, "bash", "0.1.0"), "bash — rustty 0.1.0");
        assert_eq!(window_title(None, "", "0.1.0"), "rustty 0.1.0");
    }

    #[test]
    fn the_bar_title_prefers_the_window_name_then_the_tab_name() {
        assert_eq!(
            bar_title(Some("deploy"), Some("build"), "zsh", None, "1.0"),
            "deploy — rustty 1.0"
        );
        assert_eq!(
            bar_title(None, Some("build"), "zsh", None, "1.0"),
            "build — rustty 1.0"
        );
        assert_eq!(
            bar_title(None, None, "zsh", None, "1.0"),
            "zsh — rustty 1.0"
        );
    }

    #[test]
    fn editing_the_window_name_shows_the_buffer_and_a_cursor() {
        assert_eq!(
            bar_title(Some("deploy"), None, "zsh", Some("dep"), "1.0"),
            "dep▌"
        );
        assert_eq!(bar_title(None, None, "", Some(""), "1.0"), "▌");
    }

    #[test]
    fn window_title_prefers_the_custom_tab_name() {
        assert_eq!(
            window_title(Some("build"), "zsh", "1.0"),
            "build — rustty 1.0"
        );
        assert_eq!(window_title(Some(""), "", "1.0"), "rustty 1.0");
    }
}
