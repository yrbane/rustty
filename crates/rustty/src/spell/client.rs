//! Interrogation de LanguageTool pour un mot survolé : requête sur un thread,
//! réponse par `UserEvent::SpellResult`. L'analyse de la réponse est pure.

use std::time::Duration;

use serde::Deserialize;

use crate::events::{UserEvent, Waker};
use crate::tab::TermId;

/// Durée maximale d'une requête : au-delà, le serveur est tenu pour muet.
const TIMEOUT: Duration = Duration::from_secs(5);
/// Remplacements montrés au plus.
pub const MAX_REPLACEMENTS: usize = 6;

/// Ce qu'un survol identifie : un mot à une place, dans un contexte. Deux
/// survols de même clé sont le même mot, même si la souris a bougé dessus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookupKey {
    pub term: TermId,
    pub row: usize,
    pub start: usize,
    pub end: usize,
    pub context: String,
}

/// Pourquoi une requête a échoué : rien ne répond à l'adresse (port fermé,
/// hôte inconnu, réponse illisible…), ou quelque chose écoute mais n'a rien
/// renvoyé dans le délai : un serveur gelé, qu'il faut dire à l'utilisateur.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LookupError {
    Unreachable(String),
    Silent,
}

/// Ce que LanguageTool dit du mot : son message et ses remplacements.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestion {
    pub message: String,
    pub replacements: Vec<String>,
}

#[derive(Deserialize)]
struct Response {
    #[serde(default)]
    matches: Vec<Match>,
}

#[derive(Deserialize)]
struct Match {
    message: String,
    #[serde(default)]
    replacements: Vec<Replacement>,
    offset: usize,
    length: usize,
}

#[derive(Deserialize)]
struct Replacement {
    value: String,
}

/// La correspondance qui recouvre `[offset, offset + length)` (unités UTF-16),
/// ou `None` si LanguageTool n'en a pas. Une réponse illisible est `None` aussi.
pub fn parse_suggestion(json: &str, offset: usize, length: usize) -> Option<Suggestion> {
    let response: Response = serde_json::from_str(json).ok()?;
    let (start, end) = (offset, offset + length);
    response
        .matches
        .into_iter()
        .find(|m| m.offset < end && m.offset + m.length > start)
        .map(|m| Suggestion {
            message: m.message,
            replacements: m
                .replacements
                .into_iter()
                .map(|r| r.value)
                .filter(|v| !v.is_empty())
                .take(MAX_REPLACEMENTS)
                .collect(),
        })
}

/// Corps `application/x-www-form-urlencoded` de la requête `/v2/check`.
pub fn check_body(text: &str) -> String {
    format!(
        "text={}&language=auto&preferredVariants=en-US",
        url_encode(text)
    )
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Interroge `{url}/v2/check` pour `key` sur un thread et réveille l'interface
/// avec le résultat : `Ok(None)` sans correspondance, `Err` si le serveur ne
/// répond pas (et pourquoi).
pub fn spawn_lookup(url: String, key: LookupKey, offset: usize, length: usize, waker: Waker) {
    std::thread::spawn(move || {
        let result =
            query(&url, &key.context, TIMEOUT).map(|json| parse_suggestion(&json, offset, length));
        waker.wake(UserEvent::SpellResult(key, result));
    });
}

/// La réponse brute de `{url}/v2/check` pour `text`, ou l'erreur classée.
fn query(url: &str, text: &str, timeout: Duration) -> Result<String, LookupError> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .build()
        .into();
    let endpoint = format!("{}/v2/check", url.trim_end_matches('/'));
    agent
        .post(&endpoint)
        .header("content-type", "application/x-www-form-urlencoded")
        .send(check_body(text).as_bytes())
        .map_err(classify)?
        .body_mut()
        .read_to_string()
        .map_err(classify)
}

/// Un délai dépassé est un serveur muet ; tout le reste, un serveur injoignable.
fn classify(error: ureq::Error) -> LookupError {
    match error {
        ureq::Error::Timeout(_) => LookupError::Silent,
        other => LookupError::Unreachable(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RESPONSE: &str = r#"{"matches":[
        {"message":"Faute de frappe possible trouvée.","replacements":[{"value":"poule"},{"value":"poulie"},{"value":""}],"offset":17,"length":6,"rule":{"id":"FR_SPELLING_RULE"}},
        {"message":"Accord","replacements":[{"value":"coulé"}],"offset":8,"length":5,"rule":{"id":"X"}}
    ]}"#;

    #[test]
    fn picks_the_match_covering_the_word_and_drops_empty_replacements() {
        let s = parse_suggestion(RESPONSE, 17, 6).unwrap();
        assert_eq!(s.message, "Faute de frappe possible trouvée.");
        assert_eq!(s.replacements, vec!["poule", "poulie"]);
        let s = parse_suggestion(RESPONSE, 9, 2).unwrap();
        assert_eq!(s.replacements, vec!["coulé"], "recouvrement partiel");
    }

    #[test]
    fn no_match_over_the_word_is_none() {
        assert_eq!(parse_suggestion(RESPONSE, 0, 2), None);
        assert_eq!(parse_suggestion("{}", 0, 2), None);
        assert_eq!(parse_suggestion("pas du json", 0, 2), None);
    }

    #[test]
    fn replacements_are_capped() {
        let many: Vec<String> = (0..10).map(|i| format!(r#"{{"value":"r{i}"}}"#)).collect();
        let json = format!(
            r#"{{"matches":[{{"message":"m","replacements":[{}],"offset":0,"length":1}}]}}"#,
            many.join(",")
        );
        assert_eq!(
            parse_suggestion(&json, 0, 1).unwrap().replacements.len(),
            MAX_REPLACEMENTS
        );
    }

    #[test]
    fn a_server_that_accepts_without_answering_is_silent_and_a_closed_port_unreachable() {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        // Le serveur accepte la connexion et garde le silence tant que dure le test.
        let hold = std::thread::spawn(move || listener.accept().map(|(s, _)| s));
        assert_eq!(
            query(&url, "ma poulle", Duration::from_millis(300)),
            Err(LookupError::Silent)
        );
        let stream = hold.join().unwrap().unwrap();
        let addr = stream.local_addr().unwrap();
        drop(stream);
        // Plus rien n'écoute à cette adresse : refus immédiat.
        let refused = query(
            &format!("http://{addr}"),
            "ma poulle",
            Duration::from_millis(300),
        );
        assert!(
            matches!(refused, Err(LookupError::Unreachable(_))),
            "{refused:?}"
        );
    }

    #[test]
    fn body_is_form_encoded() {
        assert_eq!(
            check_body("cé coule & ok"),
            "text=c%C3%A9+coule+%26+ok&language=auto&preferredVariants=en-US"
        );
    }
}
