//! Lecture des octets d'une image : fichier local ou URL http(s).

use std::io::Read;

/// Taille maximale acceptée pour une réponse HTTP.
const MAX_BODY: usize = 64 * 1024 * 1024;
/// Durée maximale d'un téléchargement, pour ne pas rester bloqué sur un serveur muet.
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

pub fn read_source(source: &str) -> Result<Vec<u8>, String> {
    if source.starts_with("http://") || source.starts_with("https://") {
        fetch(source).map_err(|e| format!("impossible de télécharger {source} : {e}"))
    } else {
        std::fs::read(source).map_err(|e| format!("impossible de lire {source} : {e}"))
    }
}

fn fetch(url: &str) -> Result<Vec<u8>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .build()
        .into();
    let mut response = agent.get(url).call().map_err(|e| describe(&e))?;
    read_limited(response.body_mut().as_reader(), MAX_BODY)
}

/// Message français pour une erreur HTTP.
fn describe(e: &ureq::Error) -> String {
    match e {
        ureq::Error::StatusCode(code) => format!("code HTTP {code}"),
        other => other.to_string(),
    }
}

/// Lit tout `reader` ; plus de `limit` octets est une erreur (jamais tronqué en silence).
fn read_limited(reader: impl Read, limit: usize) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    reader
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err(format!(
            "réponse trop volumineuse (plus de {} Mio)",
            limit / (1024 * 1024)
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_error_names_the_source() {
        let e = read_source("/nonexistent/x.png").unwrap_err();
        assert!(e.contains("/nonexistent/x.png"), "{e}");
    }

    #[test]
    fn read_limited_accepts_up_to_the_limit() {
        assert_eq!(read_limited(&[1u8; 9][..], 10).unwrap().len(), 9);
        assert_eq!(read_limited(&[1u8; 10][..], 10).unwrap().len(), 10);
    }

    #[test]
    fn read_limited_rejects_over_the_limit() {
        let e = read_limited(&[1u8; 11][..], 10).unwrap_err();
        assert!(e.contains("trop volumineuse"), "{e}");
    }

    #[test]
    fn http_status_error_is_in_french() {
        assert_eq!(describe(&ureq::Error::StatusCode(404)), "code HTTP 404");
    }

    #[test]
    fn local_file_is_read() {
        let path = std::env::temp_dir().join("rustty-img-fetch-test.bin");
        std::fs::write(&path, b"abc").unwrap();
        assert_eq!(read_source(path.to_str().unwrap()).unwrap(), b"abc");
        let _ = std::fs::remove_file(path);
    }
}
