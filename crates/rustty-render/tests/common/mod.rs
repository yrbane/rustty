//! Aide partagée des tests GPU : un contexte sans fenêtre, ou l'abandon
//! silencieux du test quand la machine n'a aucun adaptateur.

use rustty_render::GpuContext;

pub fn gpu_or_skip() -> Option<GpuContext> {
    match GpuContext::headless() {
        Ok(ctx) => Some(ctx),
        Err(e) => {
            eprintln!("test GPU ignoré : {e}");
            None
        }
    }
}
