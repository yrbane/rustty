//! Édition du nom d'un onglet : tampon, touches, validation. Pur.

/// Longueur maximale d'un nom d'onglet, en caractères.
pub const MAX_TITLE_CHARS: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenameKey {
    Text(String),
    Backspace,
    Commit,
    Cancel,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenameOutcome {
    Editing,
    /// Nom validé ; `None` pour un nom vide (retour au titre du shell).
    Commit(Option<String>),
    Cancel,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rename {
    pub tab: usize,
    pub buffer: String,
}

impl Rename {
    pub fn new(tab: usize, initial: &str) -> Self {
        Self {
            tab,
            buffer: initial.to_string(),
        }
    }

    pub fn apply(&mut self, key: RenameKey) -> RenameOutcome {
        match key {
            RenameKey::Text(text) => {
                for ch in text.chars().filter(|c| !c.is_control()) {
                    if self.buffer.chars().count() >= MAX_TITLE_CHARS {
                        break;
                    }
                    self.buffer.push(ch);
                }
                RenameOutcome::Editing
            }
            RenameKey::Backspace => {
                self.buffer.pop();
                RenameOutcome::Editing
            }
            RenameKey::Commit => {
                let name = self.buffer.trim();
                RenameOutcome::Commit((!name.is_empty()).then(|| name.to_string()))
            }
            RenameKey::Cancel => RenameOutcome::Cancel,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> RenameKey {
        RenameKey::Text(s.into())
    }

    #[test]
    fn typing_and_backspace() {
        let mut r = Rename::new(0, "");
        assert_eq!(r.apply(text("lo")), RenameOutcome::Editing);
        r.apply(text("gs"));
        r.apply(RenameKey::Backspace);
        assert_eq!(r.buffer, "log");
        r.apply(text("é"));
        r.apply(RenameKey::Backspace);
        assert_eq!(
            r.buffer, "log",
            "le retour arrière retire un caractère, pas un octet"
        );
    }

    #[test]
    fn control_characters_are_ignored() {
        let mut r = Rename::new(0, "a");
        r.apply(text("\u{1b}[31mb\r\n\tc\u{7}"));
        assert_eq!(r.buffer, "a[31mbc");
    }

    #[test]
    fn commit_trims_and_empty_means_none() {
        let mut r = Rename::new(2, "  build  ");
        assert_eq!(
            r.apply(RenameKey::Commit),
            RenameOutcome::Commit(Some("build".into()))
        );
        let mut empty = Rename::new(2, "   ");
        assert_eq!(empty.apply(RenameKey::Commit), RenameOutcome::Commit(None));
    }

    #[test]
    fn cancel_keeps_nothing() {
        let mut r = Rename::new(0, "x");
        assert_eq!(r.apply(RenameKey::Cancel), RenameOutcome::Cancel);
    }

    #[test]
    fn length_is_capped() {
        let mut r = Rename::new(0, "");
        r.apply(text(&"x".repeat(200)));
        assert_eq!(r.buffer.chars().count(), MAX_TITLE_CHARS);
        r.apply(text("y"));
        assert_eq!(r.buffer.chars().count(), MAX_TITLE_CHARS);
    }
}
