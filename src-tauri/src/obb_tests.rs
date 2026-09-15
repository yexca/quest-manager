//! Complete composite installs against a host-only ADB fixture; no headset access.
use super::*;
use sha2::{Digest, Sha256};
use std::{fs, sync::OnceLock};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}
fn scratch() -> PathBuf {
    let mut random = [0; 8];
    getrandom::fill(&mut random).unwrap();
    let path = root()
        .join("env/test-artifacts")
        .join(format!("obb-{:x}", u64::from_le_bytes(random)));
    fs::create_dir_all(&path).unwrap();
    path
}
fn executable() -> &'static PathBuf {
    static EXE: OnceLock<PathBuf> = OnceLock::new();
    EXE.get_or_init(|| {
        let output = scratch().join("mock-adb.exe");
        let status = std::process::Command::new("rustc")
            .arg("--edition=2024")
            .arg(root().join("tests/fixtures/mock-adb.rs"))
            .arg("-o")
            .arg(&output)
            .status()
            .unwrap();
        assert!(status.success());
        output
    })
}

async fn scenario(mode: &str) -> (TaskSnapshot, PathBuf) {
    let work = scratch();
    fs::copy(executable(), work.join("mock-adb.exe")).unwrap();
    let state = work.join("state");
    fs::create_dir(&state).unwrap();
    let bytes = b"Example expansion data\0\x80";
    fs::write(state.join("mode"), mode).unwrap();
    fs::write(state.join("expected-bytes"), bytes).unwrap();
    fs::write(
        state.join("expected-hash"),
        format!("{:x}", Sha256::digest(bytes)),
    )
    .unwrap();
    let first = work.join("main.1.dev.questmanager.verification.obb");
    let second = work.join("audio's 日本語.obb");
    fs::write(&first, bytes).unwrap();
    fs::write(&second, bytes).unwrap();
    let files = obb::inspect(vec![
        first.to_string_lossy().into(),
        second.to_string_lossy().into(),
    ])
    .unwrap();
    let directory = state.join("remote/sdcard/Android/obb/dev.questmanager.verification");
    if ["identical", "conflict"].contains(&mode) {
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join(&files[0].name),
            if mode == "identical" {
                bytes.as_slice()
            } else {
                b"different"
            },
        )
        .unwrap();
    }
    if mode == "changed-source" {
        fs::write(&first, b"changed source size").unwrap();
    }
    let installer = Installer::new(
        root().join("env/apk-tools"),
        root().join("env/aapt2/aapt2.exe"),
        work.join("apk-install"),
    );
    let apk = if mode == "unreadable-apk" {
        let path = work.join("unreadable.apk");
        fs::write(&path, b"Example unreadable APK").unwrap();
        path
    } else {
        root().join("tests/fixtures/verification.apk")
    };
    let request = TaskRequest {
        device: "DEMO-OBB-TRANSPORT".into(),
        kind: TaskKind::Install,
        lightning: None,
        source: Some(apk.to_string_lossy().into()),
        destination: None,
        package_name: Some(
            if mode == "wrong-package" {
                "com.example.wrong"
            } else {
                "dev.questmanager.verification"
            }
            .into(),
        ),
        install_options: (mode == "prepared").then(|| InstallOptions {
            source_stamp: crate::apk::source_stamp(&apk).unwrap(),
            display_name: None,
            icon_png: None,
            compatibility: true,
        }),
        obb: Some(ObbInstall {
            apk_source_stamp: crate::apk::source_stamp(&apk).unwrap(),
            files: files.into_iter().map(|f| f.input).collect(),
        }),
    };
    fs::copy(&apk, state.join("expected.apk")).unwrap();
    let manager = TaskManager::with_installer(installer);
    let initial = manager
        .start_inner(
            Arc::new(|_| {}),
            Adb::new(work.join("mock-adb.exe")),
            request,
        )
        .unwrap();
    let snapshot = tokio::time::timeout(Duration::from_secs(90), async {
        loop {
            let task = manager
                .snapshots()
                .into_iter()
                .find(|s| s.id == initial.id)
                .unwrap();
            if !["queued", "running"].contains(&task.status.as_str()) {
                break task;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(snapshot.device, "DEMO-OBB-TRANSPORT");
    let staging = work.join("apk-install/staging");
    assert!(
        fs::read_dir(staging).unwrap().next().is_none(),
        "APK staging was not cleaned"
    );
    (snapshot, work)
}

#[tokio::test]
async fn composite_obb_install_and_failure_boundaries_without_a_device() {
    for mode in [
        "success",
        "prepared",
        "identical",
        "conflict",
        "changed-source",
        "wrong-package",
        "redirected-directory",
        "install-failure",
        "push-failure",
        "checksum-failure",
        "publish-collision",
        "cleanup-failure",
        "unreadable-apk",
    ] {
        let (task, work) = scenario(mode).await;
        let state = work.join("state");
        let commands = fs::read_to_string(state.join("commands")).unwrap_or_default();
        let before_install = [
            "conflict",
            "changed-source",
            "wrong-package",
            "redirected-directory",
            "unreadable-apk",
        ]
        .contains(&mode);
        assert_eq!(
            task.status,
            if ["success", "identical", "prepared"].contains(&mode) {
                "success"
            } else {
                "failed"
            },
            "{mode}: {}",
            task.detail
        );
        assert_eq!(
            task.apk_installed,
            !before_install && mode != "install-failure",
            "{mode}: {}",
            task.detail
        );
        if before_install {
            assert!(!commands.contains(" install -r "), "{mode}");
        }
        if mode == "install-failure" {
            assert!(!commands.contains(" push "));
        }
        if mode == "identical" {
            assert_eq!(commands.matches(" push ").count(), 1);
        }
        if mode == "success" {
            assert_eq!(commands.matches(" push ").count(), 2);
            assert!(task.detail.contains("All 2 OBB"));
            let remote = state.join("remote/sdcard/Android/obb/dev.questmanager.verification");
            assert_eq!(
                fs::read(remote.join("audio's 日本語.obb")).unwrap(),
                fs::read(state.join("expected-bytes")).unwrap()
            );
        }
        if mode == "push-failure" {
            assert!(task.detail.contains("incomplete (1/2"), "{}", task.detail);
        }
        if mode == "publish-collision" {
            assert_eq!(fs::read(state.join("remote/sdcard/Android/obb/dev.questmanager.verification/main.1.dev.questmanager.verification.obb")).unwrap(), b"external file");
        }
        let remote = state.join("remote/sdcard/Android/obb/dev.questmanager.verification");
        if mode == "cleanup-failure" {
            assert!(
                task.detail.contains("Temporary OBB data may remain"),
                "{}",
                task.detail
            );
            assert!(fs::read_dir(&remote).unwrap().any(|e| {
                e.unwrap()
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".partial")
            }));
        } else if remote.is_dir() {
            assert!(
                fs::read_dir(remote).unwrap().all(|e| !e
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".partial")),
                "{mode}"
            );
        }
        assert!(work.starts_with(root().join("env/test-artifacts")));
        fs::remove_dir_all(work).unwrap();
    }
}

#[tokio::test]
async fn optional_setup_orders_components_and_preserves_partial_completion() {
    for (mode, expected_installs, changed) in [
        ("lightning-success", 2, true),
        ("lightning-main-failure", 1, false),
        ("lightning-service-failure", 2, true),
        ("lightning-existing", 0, false),
        ("lightning-newer", 0, false),
        ("lightning-service-only", 0, false),
    ] {
        let work = scratch();
        fs::copy(executable(), work.join("mock-adb.exe")).unwrap();
        let state = work.join("state");
        fs::create_dir(&state).unwrap();
        fs::write(state.join("mode"), mode).unwrap();
        let fixture = root().join("tests/fixtures/verification.apk");
        fs::copy(&fixture, state.join("expected.apk")).unwrap();
        for index in 0..2 {
            fs::copy(&fixture, work.join(format!("{index}.apk"))).unwrap();
        }
        let manager = TaskManager::new();
        let id = "EXAMPLE-LIGHTNING-TASK".to_string();
        let cancelled = Arc::new(AtomicBool::new(false));
        manager.inner.records.lock().unwrap().insert(
            id.clone(),
            TaskRecord {
                cancelled: cancelled.clone(),
                snapshot: TaskSnapshot {
                    id: id.clone(),
                    revision: 0,
                    device: "DEMO-OBB-TRANSPORT".into(),
                    kind: TaskKind::Install,
                    package_name: None,
                    apk_installed: false,
                    includes_obb: false,
                    label: "Example optional setup".into(),
                    status: "running".into(),
                    detail: String::new(),
                    progress: None,
                    created_at: 0,
                },
            },
        );
        let context = Context {
            manager: manager.clone(),
            emit: Arc::new(|_| {}),
            adb: Adb::new(work.join("mock-adb.exe")),
            id,
            cancelled,
            device: "DEMO-OBB-TRANSPORT".into(),
        };
        let assets: Vec<_> = [lightning::LAUNCHER, lightning::NAVIGATOR]
            .into_iter()
            .enumerate()
            .map(|(i, package)| lightning::Release {
                package: package.into(),
                tag: "1.0".into(),
                asset_id: i as u64,
                size: 1,
                published_at: String::new(),
                sha256: None,
                url: String::new(),
            })
            .collect();
        let verified = assets
            .iter()
            .enumerate()
            .filter(|(i, _)| mode != "lightning-service-only" || *i == 1)
            .map(|(i, asset)| (asset, work.join(format!("{i}.apk")), "1".into()))
            .collect();
        let result = context.apply_lightning(verified).await;
        let commands = fs::read_to_string(state.join("commands")).unwrap();
        assert_eq!(
            commands
                .lines()
                .filter(|line| line.contains(" install -r "))
                .count(),
            expected_installs,
            "{mode}: {result:?}"
        );
        assert_eq!(manager.snapshots()[0].apk_installed, changed, "{mode}");
        match mode {
            "lightning-success" | "lightning-existing" => {
                assert!(result.is_ok(), "{mode}: {result:?}")
            }
            "lightning-service-failure" => {
                let error = result.unwrap_err();
                assert!(error.contains("Lightning Launcher installed"));
                assert!(error.contains("were kept"));
            }
            _ => assert!(result.is_err(), "{mode}"),
        }
        assert!(!commands.contains("settings put") && !commands.contains("uninstall"));
        fs::remove_dir_all(work).unwrap();
    }
}
