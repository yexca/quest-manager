// Host-only ADB protocol fixture. It cannot forward commands to a device.
use std::{env, fs, io::Write, path::PathBuf, process};

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    process::exit(1)
}
fn words(input: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    for c in input.chars() {
        if quote == Some(c) {
            quote = None;
        } else if quote.is_none() && (c == '\'' || c == '"') {
            quote = Some(c);
        } else if quote.is_none() && c.is_whitespace() {
            if !word.is_empty() {
                result.push(std::mem::take(&mut word));
            }
        } else {
            word.push(c);
        }
    }
    if !word.is_empty() {
        result.push(word);
    }
    assert!(quote.is_none());
    result
}
fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    assert_eq!(&args[..2], &["-s", "DEMO-OBB-TRANSPORT"]);
    let root = env::current_exe().unwrap().parent().unwrap().join("state");
    fs::create_dir_all(&root).unwrap();
    writeln!(
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(root.join("commands"))
            .unwrap(),
        "{}",
        args.join(" ")
    )
    .unwrap();
    let mode = fs::read_to_string(root.join("mode")).unwrap_or_default();
    let remote = |path: &str| -> PathBuf {
        assert!(path.starts_with("/sdcard/"));
        assert!(!path.contains(".."));
        root.join("remote").join(path.trim_start_matches('/'))
    };
    match args[2].as_str() {
        "install" => {
            assert_eq!(args[3], "-r");
            assert!(PathBuf::from(&args[4]).is_file());
            if mode == "lightning-main-failure" || (mode == "lightning-service-failure" && args[4].ends_with("1.apk")) { fail("Failure [EXAMPLE_INSTALL_ERROR]"); }
            let original = fs::read(root.join("expected.apk")).unwrap();
            let installed = fs::read(&args[4]).unwrap();
            if mode == "prepared" {
                assert_ne!(installed, original);
            } else {
                assert_eq!(installed, original);
            }
            if mode == "install-failure" {
                fail("Failure [EXAMPLE_INSTALL_ERROR]");
            }
            fs::write(root.join("installed"), b"yes").unwrap();
            println!("Success");
        }
        "push" => {
            let target = remote(&args[4]);
            fs::copy(&args[3], &target).unwrap();
            if mode == "push-failure" && args[4].contains("obb-1.partial") {
                fail("Example interrupted transfer");
            }
            println!("[100%] copied");
        }
        "shell" => {
            let command = &args[3];
            let tokens = words(command);
            let paths: Vec<_> = tokens
                .iter()
                .filter(|w| w.starts_with("/sdcard/"))
                .collect();
            if command.starts_with("pm list packages") {
                if command.contains(" -3 ") && ["lightning-existing", "lightning-newer"].contains(&mode.as_str()) {
                    // Fixed product IDs exercise recognition; all paths, versions and state are fictional.
                    println!("package:/data/app/example/base.apk=com.threethan.launcher installer=com.android.shell versionCode:1");
                    println!("package:/data/app/example/service.apk=com.threethan.launcher.service.navigator installer=com.android.shell versionCode:{}", if mode == "lightning-newer" { 2 } else { 1 });
                }
            } else if command.starts_with("pm path") {
                if !root.join("installed").exists() {
                    fail("Example package is absent");
                }
                println!("package:/data/app/example/base.apk");
            } else if command.contains("elif [ -e") && command.contains("readlink -f") {
                let path = paths[0];
                if mode == "redirected-directory" {
                    println!("/storage/emulated/0/Download");
                } else if remote(path).is_dir() {
                    println!("{}", path.replace("/sdcard", "/storage/emulated/0"));
                } else {
                    println!("missing");
                }
            } else if command.contains("printf conflict") {
                let path = remote(paths[0]);
                println!(
                    "{}",
                    if path.is_file() {
                        "file"
                    } else if path.exists() {
                        "conflict"
                    } else {
                        "missing"
                    }
                );
            } else if command.contains("sha256sum") {
                let data = fs::read(remote(paths[0])).unwrap();
                let expected = fs::read(root.join("expected-bytes")).unwrap();
                if ["checksum-failure", "cleanup-failure"].contains(&mode.as_str())
                    || data != expected
                {
                    println!("{}  -", "0".repeat(64));
                } else {
                    println!(
                        "{}  -",
                        fs::read_to_string(root.join("expected-hash")).unwrap()
                    );
                }
            } else if command.contains("printf exists") {
                if remote(paths[0]).exists() {
                    print!("exists");
                }
            } else if command.starts_with("mkdir -p") {
                fs::create_dir_all(remote(paths[0])).unwrap();
            } else if command.starts_with("set -C") {
                fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(remote(paths[0]))
                    .unwrap();
            } else if command.starts_with("mv -n") {
                let source = remote(paths[0]);
                let target = remote(paths[1]);
                if mode == "publish-collision" {
                    fs::write(&target, b"external file").unwrap();
                }
                if target.exists() {
                    fail("Example destination already exists");
                }
                fs::rename(source, target).unwrap();
            } else if command.starts_with("rm -f") {
                if mode == "cleanup-failure" {
                    fail("Example cleanup failure");
                }
                let target = remote(paths[0]);
                if target.exists() {
                    fs::remove_file(target).unwrap();
                }
            } else {
                fail(&format!("Unrecognized mock shell command: {command}"));
            }
        }
        _ => fail("Unrecognized mock ADB operation"),
    }
}
