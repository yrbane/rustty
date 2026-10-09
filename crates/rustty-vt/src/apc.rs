//! Extraction des chaînes APC (`ESC _ … ESC \`) avant le parseur `vte`, qui les
//! avale silencieusement. Le protocole graphique kitty y transporte ses commandes.

/// Taille maximale d'une charge APC : au-delà de la limite de 64 Mio des
/// données d'image, pour laisser passer les clés de contrôle.
pub(crate) const MAX_APC: usize = 96 * 1024 * 1024;

const ESC: u8 = 0x1b;

/// Fragment du flux d'entrée, dans l'ordre d'origine.
#[derive(Debug)]
pub(crate) enum Chunk<'a> {
    /// Octets ordinaires, empruntés à la lecture courante.
    Bytes(&'a [u8]),
    /// Octets à rendre à `vte` qui ne viennent pas de la lecture courante
    /// (ESC retenu entre deux lectures, ESC + octet d'une APC interrompue).
    Owned(Vec<u8>),
    /// Charge d'une APC complète, sans `ESC _` ni `ESC \`.
    Apc(Vec<u8>),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum State {
    #[default]
    Ground,
    /// ESC vu en fin de lecture : on attend l'octet suivant pour trancher.
    Esc,
    Apc,
    /// ESC vu dans une APC.
    ApcEsc,
    /// APC trop longue : on jette jusqu'au terminateur.
    Overflow,
    OverflowEsc,
}

/// Machine à états qui sépare les APC du reste du flux, d'une lecture à l'autre.
#[derive(Debug, Default)]
pub(crate) struct ApcSplitter {
    state: State,
    payload: Vec<u8>,
}

impl ApcSplitter {
    pub(crate) fn feed<'a>(&mut self, bytes: &'a [u8], out: &mut Vec<Chunk<'a>>) {
        let mut start = 0;
        let mut i = 0;
        if self.state == State::Esc {
            // L'ESC retenu est tranché par le premier octet de cette lecture.
            let Some(&first) = bytes.first() else { return };
            self.state = State::Ground;
            if first == b'_' {
                self.state = State::Apc;
                i = 1;
                start = 1;
            } else {
                out.push(Chunk::Owned(vec![ESC]));
            }
        }
        while i < bytes.len() {
            let b = bytes[i];
            match self.state {
                State::Ground => {
                    if b == ESC {
                        match bytes.get(i + 1) {
                            Some(b'_') => {
                                push_bytes(out, &bytes[start..i]);
                                self.state = State::Apc;
                                i += 1;
                                start = i + 1;
                            }
                            None => {
                                push_bytes(out, &bytes[start..i]);
                                self.state = State::Esc;
                                start = bytes.len();
                            }
                            // Autre séquence : elle reste dans la tranche pour vte.
                            Some(_) => {}
                        }
                    }
                }
                State::Apc => {
                    if b == ESC {
                        self.state = State::ApcEsc;
                    } else if self.payload.len() >= MAX_APC {
                        self.payload = Vec::new();
                        self.state = State::Overflow;
                    } else {
                        self.payload.push(b);
                    }
                }
                State::ApcEsc => {
                    if b == b'\\' {
                        out.push(Chunk::Apc(std::mem::take(&mut self.payload)));
                        self.state = State::Ground;
                        start = i + 1;
                    } else {
                        // Choix : un ESC suivi d'autre chose que `\` termine l'APC,
                        // qui est abandonnée ; l'ESC est rendu à vte et l'octet
                        // courant est retraité en sol.
                        self.payload.clear();
                        out.push(Chunk::Owned(vec![ESC]));
                        self.state = State::Ground;
                        start = i;
                        continue;
                    }
                }
                State::Overflow => {
                    if b == ESC {
                        self.state = State::OverflowEsc;
                    }
                }
                State::OverflowEsc => {
                    self.state = match b {
                        b'\\' => {
                            start = i + 1;
                            State::Ground
                        }
                        ESC => State::OverflowEsc,
                        _ => State::Overflow,
                    };
                }
                // Traité avant la boucle.
                State::Esc => {}
            }
            i += 1;
        }
        if self.state == State::Ground {
            push_bytes(out, &bytes[start..]);
        }
    }
}

fn push_bytes<'a>(out: &mut Vec<Chunk<'a>>, bytes: &'a [u8]) {
    if !bytes.is_empty() {
        out.push(Chunk::Bytes(bytes));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(parts: &[&[u8]]) -> (Vec<u8>, Vec<Vec<u8>>) {
        let mut s = ApcSplitter::default();
        let (mut text, mut apcs) = (Vec::new(), Vec::new());
        for p in parts {
            let mut out = Vec::new();
            s.feed(p, &mut out);
            for c in out {
                match c {
                    Chunk::Bytes(b) => text.extend_from_slice(b),
                    Chunk::Owned(b) => text.extend_from_slice(&b),
                    Chunk::Apc(a) => apcs.push(a),
                }
            }
        }
        (text, apcs)
    }

    #[test]
    fn plain_bytes_pass_through() {
        assert_eq!(
            split(&[b"hello\x1b[31m"]),
            (b"hello\x1b[31m".to_vec(), vec![])
        );
    }

    #[test]
    fn apc_is_extracted_between_text() {
        let (text, apcs) = split(&[b"a\x1b_Gi=1;QUJD\x1b\\b"]);
        assert_eq!(text, b"ab");
        assert_eq!(apcs, vec![b"Gi=1;QUJD".to_vec()]);
    }

    #[test]
    fn apc_split_across_reads_is_reassembled() {
        let (text, apcs) = split(&[b"x\x1b", b"_Ga=T;", b"AA\x1b", b"\\y"]);
        assert_eq!(text, b"xy");
        assert_eq!(apcs, vec![b"Ga=T;AA".to_vec()]);
    }

    #[test]
    fn lone_escape_at_the_end_of_a_read_is_not_lost() {
        let (text, _) = split(&[b"a\x1b", b"[1m"]);
        assert_eq!(text, b"a\x1b[1m");
    }

    #[test]
    fn unterminated_apc_does_not_eat_the_stream_forever() {
        let mut big = b"\x1b_G".to_vec();
        big.extend(std::iter::repeat_n(b'A', MAX_APC + 10));
        big.extend_from_slice(b"\x1b\\visible");
        let (text, apcs) = split(&[&big]);
        assert!(apcs.is_empty(), "une APC trop longue est abandonnée");
        assert_eq!(text, b"visible");
    }

    #[test]
    fn escape_inside_apc_not_followed_by_backslash_ends_it() {
        let (text, apcs) = split(&[b"a\x1b_Gxx\x1b[1mb"]);
        assert!(apcs.is_empty());
        assert_eq!(text, b"a\x1b[1mb");
    }
}
