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

/// Le fichier `.desktop` qui lance `exec`.
pub fn desktop_entry(exec: &Path) -> String {
    let exec = exec.display().to_string();
    let exec = if exec.contains(' ') {
        format!("\"{exec}\"")
    } else {
        exec
    };
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
    std::fs::write(path, bytes).map_err(with_path)
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
