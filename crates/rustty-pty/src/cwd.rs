//! Dossier courant d'un processus, lu auprès du système : `/proc` sous Linux,
//! `proc_pidinfo` sous macOS. Les autres OS ne l'exposent pas simplement.

use std::path::PathBuf;

/// Dossier courant du processus `pid`, seulement s'il existe encore.
pub fn process_cwd(pid: u32) -> Option<PathBuf> {
    raw_cwd(pid).filter(|dir| dir.is_dir())
}

/// Sous Linux, un dossier supprimé se lit `<chemin> (deleted)` : le filtre
/// `is_dir` de l'appelant l'écarte.
#[cfg(target_os = "linux")]
fn raw_cwd(pid: u32) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/{pid}/cwd")).ok()
}

#[cfg(target_os = "macos")]
fn raw_cwd(pid: u32) -> Option<PathBuf> {
    use std::ffi::CStr;
    use std::os::unix::ffi::OsStrExt;

    let mut info = std::mem::MaybeUninit::<libc::proc_vnodepathinfo>::zeroed();
    let size = std::mem::size_of::<libc::proc_vnodepathinfo>() as libc::c_int;
    // SAFETY : le tampon a exactement la taille annoncée à proc_pidinfo.
    let written = unsafe {
        libc::proc_pidinfo(
            pid as libc::c_int,
            libc::PROC_PIDVNODEPATHINFO,
            0,
            info.as_mut_ptr().cast(),
            size,
        )
    };
    if written != size {
        return None;
    }
    // SAFETY : proc_pidinfo a rempli toute la structure.
    let info = unsafe { info.assume_init() };
    // SAFETY : `vip_path` est un tampon de MAXPATHLEN octets terminé par NUL.
    let path = unsafe { CStr::from_ptr(info.pvi_cdir.vip_path.as_ptr().cast()) };
    let bytes = path.to_bytes();
    (!bytes.is_empty()).then(|| PathBuf::from(std::ffi::OsStr::from_bytes(bytes)))
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn raw_cwd(_pid: u32) -> Option<PathBuf> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn own_cwd_is_found() {
        let expected = std::env::current_dir().unwrap().canonicalize().unwrap();
        assert_eq!(process_cwd(std::process::id()), Some(expected));
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    #[test]
    fn cwd_is_unknown_elsewhere() {
        assert_eq!(process_cwd(std::process::id()), None);
    }

    #[test]
    fn unknown_pid_gives_none() {
        assert_eq!(process_cwd(u32::MAX), None);
    }
}
