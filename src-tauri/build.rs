fn main() {
    // Recompile embedded Windows resources when artwork changes on its own.
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=icons/icon.png");
    tauri_build::build();
}
