//! Lecture des octets d'une image : fichier local ou URL http(s).

use std::io::Read;

/// Taille maximale acceptée pour une réponse HTTP.
const MAX_BODY: u64 = 64 * 1024 * 1024;

pub fn read_source(source: &str) -> Result<Vec<u8>, String> {
    if source.starts_with("http://") || source.starts_with("https://") {
        fetch(source).map_err(|e| format!("impossible de télécharger {source} : {e}"))
    } else {
        std::fs::read(source).map_err(|e| format!("impossible de lire {source} : {e}"))
    }
}

fn fetch(url: &str) -> Result<Vec<u8>, ureq::Error> {
    let mut response = ureq::get(url).call()?;
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(MAX_BODY)
        .read_to_end(&mut bytes)?;
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
    fn local_file_is_read() {
        let path = std::env::temp_dir().join("rustty-img-fetch-test.bin");
        std::fs::write(&path, b"abc").unwrap();
        assert_eq!(read_source(path.to_str().unwrap()).unwrap(), b"abc");
        let _ = std::fs::remove_file(path);
    }
}
