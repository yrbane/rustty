//! Titres : gabarit des onglets et titre de la fenêtre (avec la version).

pub fn tab_title(template: &str, index: usize, title: &str) -> String {
    let title = if title.is_empty() { "shell" } else { title };
    template
        .replace("{index}", &(index + 1).to_string())
        .replace("{title}", title)
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
