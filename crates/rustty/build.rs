//! Embarque l'icône dans l'exécutable Windows (explorateur, barre des tâches,
//! raccourcis). Ailleurs, rien à faire au moment de la compilation.

fn main() {
    println!("cargo:rerun-if-changed=../../assets/icons/rustty.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    embed_icon();
}

/// Compilation sur Windows : l'icône est embarquée dans l'exécutable.
#[cfg(windows)]
fn embed_icon() {
    let mut resource = winresource::WindowsResource::new();
    resource.set_icon("../../assets/icons/rustty.ico");
    resource
        .compile()
        .expect("compilation de la ressource d'icône Windows");
}

/// Compilation croisée depuis un autre système : le compilateur de
/// ressources Windows n'est pas disponible, l'icône ne peut pas être embarquée.
#[cfg(not(windows))]
fn embed_icon() {
    println!(
        "cargo:warning=icône Windows non embarquée : compilation croisée sans compilateur de ressources (compiler sous Windows pour l'obtenir)"
    );
}
