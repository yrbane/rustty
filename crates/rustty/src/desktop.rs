//! Intégration au bureau Linux : fichier `.desktop` et icônes hicolor, pour
//! que GNOME, KDE et les autres affichent l'icône de rustty dans la barre des
//! tâches et les menus (la fenêtre s'annonce avec l'identifiant `rustty`).

use std::io;
use std::path::{Path, PathBuf};

/// Tailles des icônes PNG installées dans le thème hicolor.
pub const ICON_SIZES: [u32; 7] = [16, 32, 48, 64, 128, 256, 512];

const ICON_PNGS: [&[u8]; 7] = [
    include_bytes!("../../../assets/icons/rustty-16.png"),
    include_bytes!("../../../assets/icons/rustty-32.png"),
    include_bytes!("../../../assets/icons/rustty-48.png"),
    include_bytes!("../../../assets/icons/rustty-64.png"),
    include_bytes!("../../../assets/icons/rustty-128.png"),
    include_bytes!("../../../assets/icons/rustty-256.png"),
    include_bytes!("../../../assets/icons/rustty-512.png"),
];
const ICON_SVG: &[u8] = include_bytes!("../../../assets/icon.svg");

/// L'argument `Exec=` selon la spécification desktop-entry : `%` doublé,
/// guillemets si le chemin contient un caractère réservé, et `\"`, `` ` ``,
/// `$`, `\` précédés d'un `\` à l'intérieur des guillemets ; puis les
/// échappements du type « string » sur le tout.
fn exec_argument(exec: &Path) -> String {
    let raw = exec.display().to_string().replace('%', "%%");
    let reserved = |c: char| " \t\n\"'\\><~|&;$*?#()`".contains(c);
    if !raw.contains(reserved) {
        return raw;
    }
    let mut quoted = String::with_capacity(raw.len() + 2);
    quoted.push('"');
    for c in raw.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            quoted.push('\\');
        }
        quoted.push(c);
    }
    quoted.push('"');
    string_escape(&quoted)
}

/// Échappements du type « string » de la spécification : `\\`, `\n`, `\t`,
/// `\r` (une valeur ne doit jamais s'étendre sur plusieurs lignes).
fn string_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

/// Le fichier `.desktop` qui lance `exec`.
pub fn desktop_entry(exec: &Path) -> String {
    let exec = exec_argument(exec);
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=rustty\n\
         GenericName=Terminal\n\
         Comment=Émulateur de terminal GPU inspiré de kitty\n\
         Exec={exec}\n\
         Icon=rustty\n\
         StartupWMClass=rustty\n\
         Terminal=false\n\
         Categories=System;TerminalEmulator;\n"
    )
}

/// Écrit le `.desktop` et les icônes sous `data_home` (en général
/// `~/.local/share`) ; rend les fichiers écrits.
pub fn install(data_home: &Path, exec: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files: Vec<(PathBuf, &[u8])> = Vec::new();
    let entry = desktop_entry(exec);
    files.push((
        data_home.join("applications/rustty.desktop"),
        entry.as_bytes(),
    ));
    for (size, png) in ICON_SIZES.iter().zip(ICON_PNGS) {
        files.push((
            data_home.join(format!("icons/hicolor/{size}x{size}/apps/rustty.png")),
            png,
        ));
    }
    files.push((
        data_home.join("icons/hicolor/scalable/apps/rustty.svg"),
        ICON_SVG,
    ));
    let mut written = Vec::with_capacity(files.len());
    for (path, bytes) in files {
        write_file(&path, bytes)?;
        written.push(path);
    }
    Ok(written)
}

fn write_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let with_path = |e: io::Error| io::Error::new(e.kind(), format!("{} : {e}", path.display()));
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(with_path)?;
    }
    // Écriture dans un temporaire puis renommage : jamais de fichier tronqué.
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes)
        .and_then(|()| std::fs::rename(&tmp, path))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })
        .map_err(with_path)
}

/// `$XDG_DATA_HOME`, sinon `~/.local/share`.
pub fn data_home() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("rustty-desktop-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn entry_names_the_binary_and_the_icon() {
        let entry = desktop_entry(Path::new("/opt/rustty/bin/rustty"));
        assert!(entry.starts_with("[Desktop Entry]\n"));
        for line in [
            "Type=Application",
            "Name=rustty",
            "Exec=/opt/rustty/bin/rustty",
            "Icon=rustty",
            "StartupWMClass=rustty",
            "Terminal=false",
            "Categories=System;TerminalEmulator;",
        ] {
            assert!(entry.lines().any(|l| l == line), "{line} absent :\n{entry}");
        }
    }

    #[test]
    fn exec_paths_with_spaces_are_quoted() {
        let entry = desktop_entry(Path::new("/home/a b/rustty"));
        assert!(entry.contains("Exec=\"/home/a b/rustty\""), "{entry}");
    }

    #[test]
    fn exec_escapes_percent() {
        let entry = desktop_entry(Path::new("/opt/100%/rustty"));
        assert!(entry.contains("Exec=/opt/100%%/rustty\n"), "{entry}");
    }

    #[test]
    fn exec_quotes_and_escapes_reserved_characters() {
        // Deux couches : citation de l'argument, puis échappement de la chaîne.
        for (path, exec) in [
            (r#"/a"b/r"#, r#"Exec="/a\\"b/r""#),
            ("/a$b/r", r#"Exec="/a\\$b/r""#),
            ("/a`b/r", r#"Exec="/a\\`b/r""#),
            (r"/a\b/r", r#"Exec="/a\\\\b/r""#),
            ("/a'b/r", r#"Exec="/a'b/r""#),
            ("/a b%/r", r#"Exec="/a b%%/r""#),
        ] {
            let entry = desktop_entry(Path::new(path));
            assert!(entry.contains(&format!("{exec}\n")), "{path} -> {entry}");
        }
    }

    #[test]
    fn exec_with_a_newline_stays_on_one_line() {
        let entry = desktop_entry(Path::new("/a\nb\tc/r"));
        let execs: Vec<_> = entry.lines().filter(|l| l.starts_with("Exec=")).collect();
        assert_eq!(execs, [r#"Exec="/a\nb\tc/r""#], "{entry}");
        assert!(entry.lines().any(|l| l == "Icon=rustty"), "{entry}");
    }

    #[test]
    fn install_leaves_no_temp_files() {
        let home = temp_dir("tmp");
        let written = install(&home, Path::new("/usr/bin/rustty")).unwrap();
        let mut count = 0;
        let mut stack = vec![home.clone()];
        while let Some(dir) = stack.pop() {
            for e in std::fs::read_dir(dir).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    count += 1;
                    assert_ne!(p.extension().and_then(|x| x.to_str()), Some("tmp"), "{p:?}");
                }
            }
        }
        assert_eq!(count, written.len());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn install_creates_every_file_under_data_home() {
        let home = temp_dir("ok");
        let written = install(&home, Path::new("/usr/bin/rustty")).unwrap();
        assert_eq!(written.len(), 1 + ICON_SIZES.len() + 1);
        let desktop = home.join("applications/rustty.desktop");
        assert!(
            std::fs::read_to_string(&desktop)
                .unwrap()
                .contains("Exec=/usr/bin/rustty")
        );
        for size in ICON_SIZES {
            let png = home.join(format!("icons/hicolor/{size}x{size}/apps/rustty.png"));
            let bytes = std::fs::read(&png).unwrap();
            assert_eq!(&bytes[1..4], b"PNG", "{}", png.display());
        }
        assert!(
            home.join("icons/hicolor/scalable/apps/rustty.svg")
                .is_file()
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn install_into_an_unwritable_place_is_an_error() {
        let home = temp_dir("ko");
        std::fs::create_dir_all(home.parent().unwrap()).unwrap();
        std::fs::write(&home, b"un fichier, pas un dossier").unwrap();
        let err = install(&home, Path::new("/usr/bin/rustty")).unwrap_err();
        assert!(
            err.to_string().contains(&home.display().to_string()),
            "{err}"
        );
        let _ = std::fs::remove_file(&home);
    }
}
