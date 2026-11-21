//! Intégration au bureau macOS : le bundle `rustty.app`, pour que rustty
//! apparaisse dans le Launchpad, Spotlight et le Dock avec son icône, comme
//! les autres applications.
//!
//! Le bundle contient une copie de l'exécutable : macOS rattache une fenêtre
//! à son application par le chemin du binaire, un lien vers `~/.cargo/bin`
//! ne suffirait pas.

use std::io;
use std::path::{Path, PathBuf};

use crate::desktop::write_file;

/// Nom du bundle dans le dossier des applications.
pub const BUNDLE_NAME: &str = "rustty.app";

/// Identifiant de l'application pour macOS (préférences, Dock, autorisations).
pub const BUNDLE_ID: &str = "io.github.yrbane.rustty";

const ICON_ICNS: &[u8] = include_bytes!("../../../assets/icons/rustty.icns");

/// Le fichier `Info.plist` du bundle, pour la version `version` de rustty.
pub fn info_plist(version: &str) -> String {
    let version = xml_escape(version);
    let build = xml_escape(numeric_version(&version));
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>rustty</string>
    <key>CFBundleDisplayName</key>
    <string>rustty</string>
    <key>CFBundleIdentifier</key>
    <string>{BUNDLE_ID}</string>
    <key>CFBundleExecutable</key>
    <string>rustty</string>
    <key>CFBundleIconFile</key>
    <string>rustty</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleShortVersionString</key>
    <string>{version}</string>
    <key>CFBundleVersion</key>
    <string>{build}</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
"#
    )
}

/// La partie « 1.2.3 » d'une version : macOS n'accepte que des entiers
/// séparés par des points dans `CFBundleVersion`.
fn numeric_version(version: &str) -> &str {
    let end = version
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(version.len());
    version[..end].trim_end_matches('.')
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Écrit `rustty.app` dans `applications` (en général `~/Applications`) avec
/// une copie de l'exécutable `exec` ; rend les fichiers écrits.
pub fn install(applications: &Path, exec: &Path, version: &str) -> io::Result<Vec<PathBuf>> {
    let contents = applications.join(BUNDLE_NAME).join("Contents");
    let plist = contents.join("Info.plist");
    let icon = contents.join("Resources/rustty.icns");
    let binary = contents.join("MacOS/rustty");
    write_file(&plist, info_plist(version).as_bytes())?;
    write_file(&icon, ICON_ICNS)?;
    copy_executable(exec, &binary)?;
    Ok(vec![plist, icon, binary])
}

/// Copie `from` vers `to` via un temporaire puis un renommage : un rustty en
/// cours d'exécution depuis le bundle garde son ancien fichier intact.
fn copy_executable(from: &Path, to: &Path) -> io::Result<()> {
    let with_path = |e: io::Error| io::Error::new(e.kind(), format!("{} : {e}", to.display()));
    if let Some(dir) = to.parent() {
        std::fs::create_dir_all(dir).map_err(with_path)?;
    }
    let mut tmp = to.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::copy(from, &tmp)
        .and_then(|_| std::fs::rename(&tmp, to))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })
        .map_err(with_path)
}

/// `~/Applications` : le dossier des applications de l'utilisateur, indexé
/// par le Launchpad et Spotlight, où l'on écrit sans droits d'administration.
pub fn applications_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .map(|home| home.join("Applications"))
}

/// Vrai si `exe` est l'exécutable d'un bundle (`X.app/Contents/MacOS/x`).
pub fn is_bundled(exe: &Path) -> bool {
    let mut parents = exe.ancestors().skip(1);
    let macos = parents.next().and_then(Path::file_name);
    let contents = parents.next().and_then(Path::file_name);
    let app = parents.next().and_then(Path::extension);
    macos == Some("MacOS".as_ref())
        && contents == Some("Contents".as_ref())
        && app == Some("app".as_ref())
}

/// Vrai si ce rustty tourne depuis un bundle : il a alors été lancé par le
/// Finder, le Dock ou Spotlight, avec leur environnement minimal.
pub fn launched_from_bundle() -> bool {
    cfg!(target_os = "macos") && std::env::current_exe().is_ok_and(|exe| is_bundled(&exe))
}

/// La locale à donner aux programmes quand l'environnement n'en fixe aucune
/// (cas d'un lancement par le Finder) : celle des réglages de macOS si le
/// système la connaît, sinon `en_US.UTF-8`. Sans elle, les shells traitent
/// les caractères accentués comme des octets.
pub fn locale_from(apple_locale: Option<&str>, known: impl Fn(&str) -> bool) -> String {
    const FALLBACK: &str = "en_US.UTF-8";
    // « fr_FR », parfois suivi de « @rg=… » ou d'autres précisions.
    let name = apple_locale.map(|l| l.split('@').next().unwrap_or("").trim());
    let well_formed = |l: &&str| {
        let mut parts = l.split('_');
        let (lang, region) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
        parts.next().is_none()
            && matches!(lang.len(), 2 | 3)
            && lang.chars().all(|c| c.is_ascii_lowercase())
            && region.len() == 2
            && region.chars().all(|c| c.is_ascii_uppercase())
    };
    name.filter(well_formed)
        .map(|l| format!("{l}.UTF-8"))
        .filter(|l| known(l))
        .unwrap_or_else(|| FALLBACK.to_string())
}

/// `LANG` pour les shells d'un rustty lancé depuis le bundle, ou `None` si
/// l'environnement fixe déjà la locale ou si rustty vient d'un terminal.
pub fn bundle_locale() -> Option<String> {
    if !launched_from_bundle() {
        return None;
    }
    let set = |name| std::env::var_os(name).is_some_and(|v| !v.is_empty());
    if set("LANG") || set("LC_ALL") || set("LC_CTYPE") {
        return None;
    }
    let apple = std::process::Command::new("/usr/bin/defaults")
        .args(["read", "-g", "AppleLocale"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok());
    Some(locale_from(apple.as_deref(), |l| {
        Path::new("/usr/share/locale").join(l).is_dir()
    }))
}

static LOGIN_PATH: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();

const PATH_MARKER: &str = "__RUSTTY_PATH__";

/// Lancé depuis le bundle, demande en arrière-plan son `PATH` au shell de
/// connexion : celui du Finder ne contient ni Homebrew ni `/usr/local/bin`,
/// où vivent les éditeurs (`code`, `subl`…). Sans effet ailleurs.
pub fn warm_login_path() {
    if launched_from_bundle() {
        std::thread::spawn(|| {
            LOGIN_PATH.get_or_init(read_login_path);
        });
    }
}

/// Le `PATH` du shell de connexion, s'il a été demandé et obtenu.
pub fn login_path() -> Option<&'static str> {
    LOGIN_PATH.get()?.as_deref()
}

fn read_login_path() -> Option<String> {
    let shell = std::env::var("SHELL").ok().filter(|s| !s.is_empty())?;
    let output = std::process::Command::new(shell)
        .args(["-l", "-c"])
        .arg(format!("printf '\\n{PATH_MARKER}%s\\n' \"$PATH\""))
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    path_after_marker(&String::from_utf8_lossy(&output.stdout))
}

/// Le `PATH` dans la sortie du shell, après le marqueur : le profil de
/// l'utilisateur peut avoir écrit autre chose avant.
fn path_after_marker(output: &str) -> Option<String> {
    output
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix(PATH_MARKER))
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_string)
}

/// Signe le bundle localement (signature ad hoc, sans certificat). Sans
/// signature, macOS n'a pas d'identité à laquelle rattacher les autorisations
/// (accès aux dossiers protégés, etc.) et les redemande.
pub fn sign(bundle: &Path) -> io::Result<()> {
    let output = std::process::Command::new("/usr/bin/codesign")
        .args(["--force", "--sign", "-"])
        .arg(bundle)
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        let reason = String::from_utf8_lossy(&output.stderr);
        Err(io::Error::other(reason.trim().to_string()))
    }
}

/// Signale le bundle à LaunchServices pour qu'il apparaisse tout de suite
/// dans le Launchpad et Spotlight ; sans effet si l'outil manque.
pub fn register(bundle: &Path) {
    const LSREGISTER: &str = "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister";
    let _ = std::process::Command::new(LSREGISTER)
        .arg("-f")
        .arg(bundle)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_names_the_executable_the_icon_and_the_version() {
        let plist = info_plist("0.1.0-alpha.192");
        assert!(plist.starts_with("<?xml"));
        for (key, value) in [
            ("CFBundleExecutable", "rustty"),
            ("CFBundleIconFile", "rustty"),
            ("CFBundleIdentifier", BUNDLE_ID),
            ("CFBundlePackageType", "APPL"),
            ("CFBundleShortVersionString", "0.1.0-alpha.192"),
            ("CFBundleVersion", "0.1.0"),
        ] {
            let pair = format!("<key>{key}</key>\n    <string>{value}</string>");
            assert!(plist.contains(&pair), "{key} absent :\n{plist}");
        }
        assert!(plist.contains("<key>NSHighResolutionCapable</key>\n    <true/>"));
    }

    #[test]
    fn numeric_version_keeps_only_the_dotted_integers() {
        assert_eq!(numeric_version("0.1.0-alpha.192"), "0.1.0");
        assert_eq!(numeric_version("1.2.3"), "1.2.3");
        assert_eq!(numeric_version("2.0.x"), "2.0");
    }

    #[test]
    fn plist_escapes_xml() {
        assert!(info_plist("1.0-<a&b>").contains("<string>1.0-&lt;a&amp;b&gt;</string>"));
    }

    #[test]
    fn install_builds_the_bundle_and_leaves_no_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let exec = dir.path().join("source");
        std::fs::write(&exec, b"binaire").unwrap();
        let apps = dir.path().join("Applications");
        let written = install(&apps, &exec, "1.2.3").unwrap();
        let contents = apps.join("rustty.app/Contents");
        assert_eq!(
            written,
            [
                contents.join("Info.plist"),
                contents.join("Resources/rustty.icns"),
                contents.join("MacOS/rustty"),
            ]
        );
        assert_eq!(std::fs::read(&written[2]).unwrap(), b"binaire");
        assert_eq!(&std::fs::read(&written[1]).unwrap()[..4], b"icns");
        assert!(
            std::fs::read_to_string(&written[0])
                .unwrap()
                .contains("1.2.3")
        );
        for sub in ["", "Resources", "MacOS"] {
            for entry in std::fs::read_dir(contents.join(sub)).unwrap() {
                let path = entry.unwrap().path();
                assert_ne!(path.extension().and_then(|x| x.to_str()), Some("tmp"));
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn install_keeps_the_binary_executable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let exec = dir.path().join("source");
        std::fs::write(&exec, b"binaire").unwrap();
        std::fs::set_permissions(&exec, std::fs::Permissions::from_mode(0o755)).unwrap();
        let written = install(dir.path(), &exec, "1.2.3").unwrap();
        let mode = std::fs::metadata(&written[2]).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0o111);
    }

    #[test]
    fn install_twice_replaces_the_binary() {
        let dir = tempfile::tempdir().unwrap();
        let exec = dir.path().join("source");
        std::fs::write(&exec, b"ancien").unwrap();
        install(dir.path(), &exec, "1.0.0").unwrap();
        std::fs::write(&exec, b"nouveau").unwrap();
        let written = install(dir.path(), &exec, "1.0.1").unwrap();
        assert_eq!(std::fs::read(&written[2]).unwrap(), b"nouveau");
    }

    #[test]
    fn install_from_a_missing_executable_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = install(dir.path(), &dir.path().join("absent"), "1.0.0").unwrap_err();
        assert!(err.to_string().contains("MacOS"), "{err}");
    }

    #[test]
    fn bundled_executables_are_recognised() {
        for (path, bundled) in [
            (
                "/Users/a/Applications/rustty.app/Contents/MacOS/rustty",
                true,
            ),
            ("/Applications/Mon Terminal.app/Contents/MacOS/rustty", true),
            ("/Users/a/.cargo/bin/rustty", false),
            ("/tmp/rustty.app/Contents/Resources/rustty", false),
            ("/tmp/MacOS/rustty", false),
            ("rustty", false),
        ] {
            assert_eq!(is_bundled(Path::new(path)), bundled, "{path}");
        }
    }

    #[test]
    fn login_path_is_read_after_the_marker() {
        let out = "Bienvenue !\n\n__RUSTTY_PATH__/usr/local/bin:/usr/bin\n";
        assert_eq!(
            path_after_marker(out).as_deref(),
            Some("/usr/local/bin:/usr/bin")
        );
        assert_eq!(path_after_marker("bruit\n"), None);
        assert_eq!(path_after_marker("__RUSTTY_PATH__\n"), None);
    }

    #[test]
    fn locale_follows_the_system_setting_when_it_is_known() {
        let all = |_: &str| true;
        assert_eq!(locale_from(Some("fr_FR\n"), all), "fr_FR.UTF-8");
        assert_eq!(locale_from(Some("fr_FR@rg=chzzzz"), all), "fr_FR.UTF-8");
    }

    #[test]
    fn locale_falls_back_to_english_utf8() {
        let all = |_: &str| true;
        assert_eq!(locale_from(None, all), "en_US.UTF-8");
        assert_eq!(locale_from(Some(""), all), "en_US.UTF-8");
        assert_eq!(locale_from(Some("zh-Hans_CN"), all), "en_US.UTF-8");
        assert_eq!(locale_from(Some("fr_FR/../x"), all), "en_US.UTF-8");
        assert_eq!(locale_from(Some("en_FR"), |_| false), "en_US.UTF-8");
    }
}
