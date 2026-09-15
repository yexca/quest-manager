fn main() {
    build_wireless_helper();
    // Recompile embedded Windows resources when artwork changes on its own.
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=icons/icon.png");
    tauri_build::build();
}

fn build_wireless_helper() {
    use std::{fs, path::PathBuf, process::Command};
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("..");
    let source = root.join("device-tools/wireless");
    let output = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("wireless-helper");
    fs::create_dir_all(output.join("smali")).unwrap();
    for file in ["apktool.yml", "smali/QuestWireless.smali"] {
        println!("cargo:rerun-if-changed={}", source.join(file).display());
        fs::copy(source.join(file), output.join(file)).unwrap();
    }
    let tools = root.join("env/apk-tools");
    let mut command = Command::new(tools.join("jre/bin/java.exe"));
    command
        .arg(format!("-Djava.io.tmpdir={}", output.display()))
        .arg("-jar")
        .arg(tools.join("apktool.jar"))
        .arg("b")
        .arg(&output)
        .arg("--no-apk")
        .arg("-p")
        .arg(output.join("framework"));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let result = command
        .output()
        .expect("Run run-install.ps1 to install the project Java and Apktool.");
    assert!(
        result.status.success(),
        "Wireless helper assembly failed: {}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.join("build/apk/classes.dex").is_file());
}
