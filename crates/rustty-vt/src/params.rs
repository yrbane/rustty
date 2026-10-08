//! Lecture des paramètres CSI. Les séquences sont indexées à 1 et « 0 » vaut
//! presque toujours « défaut », sauf pour ED/EL/SGR qui lisent la valeur brute.

use vte::Params;

pub(crate) fn args(params: &Params) -> Vec<u16> {
    params
        .iter()
        .map(|sub| sub.first().copied().unwrap_or(0))
        .collect()
}

/// Paramètre `i`, ou `default` s'il est absent ou nul.
pub(crate) fn arg_or(p: &[u16], i: usize, default: u16) -> u16 {
    match p.get(i) {
        Some(&v) if v != 0 => v,
        _ => default,
    }
}

/// Paramètre `i` tel quel, 0 s'il est absent.
pub(crate) fn raw(p: &[u16], i: usize) -> u16 {
    p.get(i).copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arg_or_treats_missing_and_zero_as_default() {
        let p = [0u16, 5];
        assert_eq!(arg_or(&p, 0, 1), 1);
        assert_eq!(arg_or(&p, 1, 1), 5);
        assert_eq!(arg_or(&p, 7, 3), 3);
    }

    #[test]
    fn raw_keeps_zero_and_defaults_missing_to_zero() {
        let p = [0u16, 5];
        assert_eq!(raw(&p, 0), 0);
        assert_eq!(raw(&p, 1), 5);
        assert_eq!(raw(&p, 9), 0);
    }
}
