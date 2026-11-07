//! Garde de forme : aucune fonction de plus de 100 lignes dans les sources
//! de l'espace de travail (tests et exemples exclus). Heuristique par
//! accolades, suffisante pour un code passé par rustfmt.

use std::path::{Path, PathBuf};

const MAX_LINES: usize = 100;

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("lecture du dossier") {
        let path = entry.expect("entrée").path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if path.is_dir() {
            if !matches!(name.as_str(), "tests" | "examples" | "target") {
                rust_sources(&path, out);
            }
        } else if name.ends_with(".rs") && !name.ends_with("_tests.rs") && name != "tests.rs" {
            out.push(path);
        }
    }
}

/// « fichier:ligne nom (n lignes) » pour chaque fonction de plus de `max` lignes.
fn long_functions(path: &Path, max: usize) -> Vec<String> {
    let text = std::fs::read_to_string(path).expect("lecture du fichier");
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim_start();
        let is_fn = ["fn ", "pub fn ", "pub(crate) fn ", "pub(super) fn "]
            .iter()
            .any(|p| trimmed.starts_with(p));
        if !is_fn {
            i += 1;
            continue;
        }
        let name = trimmed
            .split("fn ")
            .nth(1)
            .and_then(|s| s.split(['(', '<']).next())
            .unwrap_or("?");
        let mut depth = 0i32;
        let mut opened = false;
        let mut j = i;
        while j < lines.len() {
            depth += lines[j].matches('{').count() as i32 - lines[j].matches('}').count() as i32;
            opened |= lines[j].contains('{');
            if opened && depth <= 0 {
                break;
            }
            j += 1;
        }
        let len = j + 1 - i;
        if len > max {
            out.push(format!(
                "{}:{} {name} ({len} lignes)",
                path.display(),
                i + 1
            ));
        }
        i = j + 1;
    }
    out
}

#[test]
fn no_function_exceeds_one_hundred_lines() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut sources = Vec::new();
    rust_sources(&root.join("crates"), &mut sources);
    assert!(sources.len() > 50, "sources trouvées : {}", sources.len());
    let offenders: Vec<String> = sources
        .iter()
        .flat_map(|p| long_functions(p, MAX_LINES))
        .collect();
    assert!(
        offenders.is_empty(),
        "fonctions trop longues :\n{}",
        offenders.join("\n")
    );
}
