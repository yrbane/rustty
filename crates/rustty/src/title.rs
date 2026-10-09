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

pub fn window_title(term_title: &str, version: &str) -> String {
    if term_title.is_empty() {
        format!("rustty {version}")
    } else {
        format!("{term_title} — rustty {version}")
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
        assert_eq!(window_title("bash", "0.1.0"), "bash — rustty 0.1.0");
        assert_eq!(window_title("", "0.1.0"), "rustty 0.1.0");
    }
}
