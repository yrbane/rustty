//! Embarque l'icône dans l'exécutable Windows (explorateur, barre des tâches,
//! raccourcis). Ailleurs, rien à faire au moment de la compilation.

fn main() {
    println!("cargo:rerun-if-changed=../../assets/icons/rustty.ico");
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("../../assets/icons/rustty.ico");
        resource
            .compile()
            .expect("compilation de la ressource d'icône Windows");
    }
}
