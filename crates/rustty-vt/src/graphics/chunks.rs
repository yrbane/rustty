//! Réassemblage des charges fragmentées (`m=1` … `m=0`) et décodage base64.

use super::command::{Action, GraphicsCommand};
use base64::{Engine, engine::general_purpose::STANDARD};

/// Taille maximale de la charge brute accumulée (64 Mio).
pub const MAX_PAYLOAD: usize = 64 * 1024 * 1024;

/// Erreur renvoyée à l'application, avec son code kitty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphicsError {
    Invalid(String),
    TooBig,
    NoData,
    NotFound,
}

impl GraphicsError {
    /// Code court à placer dans la réponse (`EINVAL`, …).
    pub fn code(&self) -> &'static str {
        match self {
            Self::Invalid(_) => "EINVAL",
            Self::TooBig => "EFBIG",
            Self::NoData => "ENODATA",
            Self::NotFound => "ENOENT",
        }
    }
}

/// Résultat d'un morceau.
#[derive(Debug, PartialEq, Eq)]
pub enum ChunkResult {
    /// Transmission incomplète, ou morceau écarté après une erreur.
    Pending,
    /// Commande complète et charge décodée.
    Complete(GraphicsCommand, Vec<u8>),
    Error(GraphicsCommand, GraphicsError),
}

/// Accumulateur de morceaux : le premier fixe la commande, les suivants
/// n'apportent que `m=` (et `q=`, ou le même `i=`). Un morceau portant
/// d'autres clés, ou un autre `i=`, ouvre une nouvelle commande et abandonne
/// la transmission interrompue (comme kitty).
#[derive(Debug)]
pub struct Chunks {
    limit: usize,
    pending: Option<(GraphicsCommand, Vec<u8>)>,
    /// Après une erreur en cours de transmission, les morceaux restants
    /// sont écartés jusqu'au `m=0` pour ne pas produire de données parasites.
    dropping: bool,
    /// `i=` de la transmission en cours d'abandon.
    dropped_id: Option<u32>,
}

impl Default for Chunks {
    fn default() -> Self {
        Self::with_limit(MAX_PAYLOAD)
    }
}

impl Chunks {
    /// Limite explicite de la charge brute (sert aux tests).
    pub fn with_limit(limit: usize) -> Self {
        Self {
            limit,
            pending: None,
            dropping: false,
            dropped_id: None,
        }
    }

    pub fn push(&mut self, cmd: GraphicsCommand, payload: &[u8]) -> ChunkResult {
        if self.starts_new_command(&cmd) {
            self.pending = None;
            self.dropping = false;
        }
        if self.dropping {
            self.dropping = cmd.more;
            return ChunkResult::Pending;
        }
        let more = cmd.more;
        let (first, mut raw) = match self.pending.take() {
            Some(p) => p,
            None => {
                if let Some(err) = Self::reject(&cmd) {
                    self.drop_rest(more, cmd.id);
                    return ChunkResult::Error(cmd, err);
                }
                (cmd, Vec::new())
            }
        };
        if raw.len() + payload.len() > self.limit {
            self.drop_rest(more, first.id);
            return ChunkResult::Error(Self::finished(first), GraphicsError::TooBig);
        }
        raw.extend_from_slice(payload);
        if more {
            self.pending = Some((first, raw));
            return ChunkResult::Pending;
        }
        let first = Self::finished(first);
        if !carries_data(&first) {
            return ChunkResult::Complete(first, Vec::new());
        }
        if raw.is_empty() {
            return ChunkResult::Error(first, GraphicsError::NoData);
        }
        match STANDARD.decode(&raw) {
            Ok(data) => ChunkResult::Complete(first, data),
            Err(e) => ChunkResult::Error(first, GraphicsError::Invalid(format!("base64 : {e}"))),
        }
    }

    /// Écarte les morceaux restants de la transmission `id` si `more`.
    fn drop_rest(&mut self, more: bool, id: Option<u32>) {
        self.dropping = more;
        self.dropped_id = id;
    }

    /// Vrai si `cmd` ne peut pas être la suite de la transmission en cours.
    fn starts_new_command(&self, cmd: &GraphicsCommand) -> bool {
        let current = match &self.pending {
            Some((first, _)) => first.id,
            None if self.dropping => self.dropped_id,
            None => None,
        };
        cmd.other_keys || (cmd.id.is_some() && cmd.id != current)
    }

    fn finished(mut cmd: GraphicsCommand) -> GraphicsCommand {
        cmd.more = false;
        cmd
    }

    fn reject(cmd: &GraphicsCommand) -> Option<GraphicsError> {
        let invalid = |m: &str| Some(GraphicsError::Invalid(m.into()));
        if cmd.invalid {
            return invalid("action ou format inconnu");
        }
        if !carries_data(cmd) {
            return None;
        }
        if cmd.medium != 'd' {
            return invalid("seule la transmission directe (t=d) est acceptée");
        }
        if cmd.compressed {
            return invalid("la compression (o=z) n'est pas prise en charge");
        }
        None
    }
}

fn carries_data(cmd: &GraphicsCommand) -> bool {
    matches!(cmd.action, Action::Transmit | Action::TransmitAndPut)
}

#[cfg(test)]
mod tests {
    use super::super::command::parse;
    use super::*;

    fn push(chunks: &mut Chunks, apc: &[u8]) -> ChunkResult {
        let (cmd, payload) = parse(apc).unwrap();
        chunks.push(cmd, payload)
    }

    #[test]
    fn chunks_concatenate_until_the_last() {
        let mut c = Chunks::default();
        assert_eq!(push(&mut c, b"Ga=T,f=100,m=1;QUJD"), ChunkResult::Pending);
        match push(&mut c, b"Gm=0;REVG") {
            ChunkResult::Complete(cmd, data) => {
                assert_eq!(data, b"ABCDEF");
                assert!(!cmd.more);
            }
            other => panic!("inattendu : {other:?}"),
        }
    }

    #[test]
    fn bad_base64_is_invalid() {
        let mut c = Chunks::default();
        match push(&mut c, b"Gf=100;@@@") {
            ChunkResult::Error(_, e) => assert_eq!(e.code(), "EINVAL"),
            other => panic!("inattendu : {other:?}"),
        }
    }

    #[test]
    fn payload_over_the_limit_is_too_big() {
        let mut c = Chunks::with_limit(16);
        assert_eq!(push(&mut c, b"Gm=1;QUJDREVGR0hJSktM"), ChunkResult::Pending);
        match push(&mut c, b"Gm=1;QUJDREVGR0hJSktM") {
            ChunkResult::Error(_, e) => assert_eq!(e, GraphicsError::TooBig),
            other => panic!("inattendu : {other:?}"),
        }
        // La suite de la transmission est écartée, puis l'état est propre.
        assert_eq!(push(&mut c, b"Gm=0;QUJD"), ChunkResult::Pending);
        assert!(matches!(
            push(&mut c, b"Gf=100;QUJD"),
            ChunkResult::Complete(..)
        ));
    }

    #[test]
    fn file_medium_and_compression_are_refused() {
        for apc in [&b"Gt=f;QUJD"[..], b"Go=z;QUJD"] {
            let mut c = Chunks::default();
            match push(&mut c, apc) {
                ChunkResult::Error(_, GraphicsError::Invalid(_)) => {}
                other => panic!("inattendu : {other:?}"),
            }
        }
    }

    #[test]
    fn empty_transmission_is_no_data_but_put_needs_none() {
        let mut c = Chunks::default();
        match push(&mut c, b"Ga=t;") {
            ChunkResult::Error(_, e) => assert_eq!(e.code(), "ENODATA"),
            other => panic!("inattendu : {other:?}"),
        }
        assert!(matches!(push(&mut c, b"Ga=p,i=1;"), ChunkResult::Complete(_, d) if d.is_empty()));
    }

    #[test]
    fn rejected_first_chunk_swallows_its_continuations() {
        let mut c = Chunks::default();
        assert!(matches!(
            push(&mut c, b"Gt=f,m=1;QUJD"),
            ChunkResult::Error(_, GraphicsError::Invalid(_))
        ));
        assert_eq!(push(&mut c, b"Gm=1;QUJD"), ChunkResult::Pending);
        assert_eq!(push(&mut c, b"Gm=0;QUJD"), ChunkResult::Pending);
        match push(&mut c, b"Gf=100;QUJD") {
            ChunkResult::Complete(_, data) => assert_eq!(data, b"ABC"),
            other => panic!("inattendu : {other:?}"),
        }
    }

    #[test]
    fn a_full_command_abandons_an_interrupted_transfer() {
        let mut c = Chunks::default();
        assert_eq!(push(&mut c, b"Ga=T,m=1;QUJD"), ChunkResult::Pending);
        match push(&mut c, b"Ga=T,f=100,i=5;REVG") {
            ChunkResult::Complete(cmd, data) => {
                assert_eq!(cmd.id, Some(5));
                assert_eq!(data, b"DEF");
            }
            other => panic!("inattendu : {other:?}"),
        }
    }

    #[test]
    fn put_and_delete_are_never_appended_to_a_pending_transfer() {
        for apc in [&b"Ga=p,i=2;"[..], b"Ga=d,d=a;"] {
            let mut c = Chunks::default();
            assert_eq!(push(&mut c, b"Ga=T,m=1;QUJD"), ChunkResult::Pending);
            match push(&mut c, apc) {
                ChunkResult::Complete(cmd, data) => {
                    assert!(matches!(cmd.action, Action::Put | Action::Delete));
                    assert!(data.is_empty());
                }
                other => panic!("inattendu : {other:?}"),
            }
        }
    }

    #[test]
    fn a_different_id_starts_a_new_command_but_the_same_id_continues() {
        let mut c = Chunks::default();
        assert_eq!(push(&mut c, b"Gi=1,m=1;QUJD"), ChunkResult::Pending);
        assert_eq!(push(&mut c, b"Gi=1,q=2,m=1;REVG"), ChunkResult::Pending);
        match push(&mut c, b"Gi=1,m=0;R0hJ") {
            ChunkResult::Complete(_, data) => assert_eq!(data, b"ABCDEFGHI"),
            other => panic!("inattendu : {other:?}"),
        }
        assert_eq!(push(&mut c, b"Gi=1,m=1;QUJD"), ChunkResult::Pending);
        match push(&mut c, b"Gi=2;REVG") {
            ChunkResult::Complete(cmd, data) => {
                assert_eq!(cmd.id, Some(2));
                assert_eq!(data, b"DEF");
            }
            other => panic!("inattendu : {other:?}"),
        }
    }

    #[test]
    fn continuations_with_the_same_id_are_dropped_after_an_error() {
        let mut c = Chunks::default();
        assert!(matches!(
            push(&mut c, b"Gt=f,i=3,m=1;QUJD"),
            ChunkResult::Error(..)
        ));
        assert_eq!(push(&mut c, b"Gi=3,m=0;QUJD"), ChunkResult::Pending);
    }

    #[test]
    fn a_full_command_stops_dropping_after_an_error() {
        let mut c = Chunks::default();
        assert!(matches!(
            push(&mut c, b"Gt=f,m=1;QUJD"),
            ChunkResult::Error(..)
        ));
        assert!(matches!(
            push(&mut c, b"Gf=100,i=3;QUJD"),
            ChunkResult::Complete(..)
        ));
    }

    #[test]
    fn error_codes() {
        assert_eq!(GraphicsError::TooBig.code(), "EFBIG");
        assert_eq!(GraphicsError::NotFound.code(), "ENOENT");
    }
}
