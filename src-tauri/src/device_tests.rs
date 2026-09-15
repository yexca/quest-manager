//! Opt-in integration test: touches only a unique scratch folder and an inert test package.
use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};

async fn finish(
    manager: &TaskManager,
    adb: &Adb,
    request: TaskRequest,
) -> Result<TaskSnapshot, String> {
    let task = manager.start_inner(Arc::new(|_| {}), adb.clone(), request)?;
    tokio::time::timeout(Duration::from_secs(180), async {
        loop {
            let current = manager
                .snapshots()
                .into_iter()
                .find(|s| s.id == task.id)
                .ok_or("Task disappeared")?;
            if !["queued", "running"].contains(&current.status.as_str()) {
                return Ok(current);
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .map_err(|_| "Test task timed out".to_string())?
}

async fn successful(
    manager: &TaskManager,
    adb: &Adb,
    request: TaskRequest,
) -> Result<TaskSnapshot, String> {
    let task = finish(manager, adb, request).await?;
    if task.status != "success" {
        return Err(format!("{}: {}", task.label, task.detail));
    }
    Ok(task)
}

#[tokio::test]
#[ignore = "Creates a temporary folder and installs/removes the inert dev.questmanager.verification fixture"]
async fn connected_device_task_roundtrip() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .unwrap();
    let adb = Adb::new(root.join("env/platform-tools/adb.exe"));
    let devices = adb.devices().await.unwrap();
    let device = devices
        .iter()
        .find(|d| d.model.contains("Quest"))
        .expect("Connect a Quest");
    let serial = &device
        .transports
        .iter()
        .find(|t| t.state == "device")
        .expect("Authorize the device")
        .serial;
    let package = "dev.questmanager.verification";
    assert!(
        !adb.apps(serial, false)
            .await
            .unwrap()
            .iter()
            .any(|a| a.package_name == package),
        "The verification package already exists; refusing to overwrite it."
    );
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let name = format!(".quest-manager-test-{id}");
    let remote = format!("/sdcard/Download/{name}");
    let artifact_root = root.join("env/test-artifacts");
    std::fs::create_dir_all(&artifact_root).unwrap();
    let local = artifact_root.canonicalize().unwrap().join(&name);
    std::fs::create_dir(&local).unwrap();
    let upload = local.join("Round trip 日本語");
    std::fs::create_dir_all(upload.join("Sub folder")).unwrap();
    std::fs::create_dir(local.join("downloads")).unwrap();
    std::fs::create_dir(local.join("exports")).unwrap();
    let payload: Vec<u8> = (0..65536).map(|i| (i % 251) as u8).collect();
    let filename = "sample ' 日本語 $(echo safe).txt";
    std::fs::write(upload.join(filename), &payload).unwrap();
    std::fs::write(upload.join("Sub folder/nested.txt"), b"nested file payload").unwrap();
    let apk = local.join("Verification 日本語.apk");
    std::fs::copy(root.join("tests/fixtures/verification.apk"), &apk).unwrap();
    let installer = Installer::new(
        root.join("env/apk-tools"),
        root.join("env/aapt2/aapt2.exe"),
        local.join("apk-install"),
    );
    let manager = TaskManager::with_installer(installer.clone());
    let request = |kind,
                   source: Option<String>,
                   destination: Option<String>,
                   package_name: Option<String>| TaskRequest {
        device: serial.clone(),
        kind,
        source,
        destination,
        package_name,
        install_options: None,
        obb: None,
        lightning: None,
    };

    let result: Result<(), String> = async {
        successful(
            &manager,
            &adb,
            request(
                TaskKind::Mkdir,
                Some("/sdcard/Download".into()),
                Some(name.clone()),
                None,
            ),
        )
        .await?;
        successful(
            &manager,
            &adb,
            request(
                TaskKind::Upload,
                Some(upload.to_string_lossy().into()),
                Some(remote.clone()),
                None,
            ),
        )
        .await?;
        let duplicate = finish(
            &manager,
            &adb,
            request(
                TaskKind::Upload,
                Some(upload.to_string_lossy().into()),
                Some(remote.clone()),
                None,
            ),
        )
        .await?;
        if duplicate.status != "failed" || !duplicate.detail.contains("already exists") {
            return Err("Duplicate upload did not refuse overwriting".into());
        }
        let uploaded = format!("{remote}/Round trip 日本語");
        let entries = adb.files(serial, &uploaded).await?;
        if !entries
            .iter()
            .any(|f| f.name == filename && f.size == payload.len() as u64)
        {
            return Err("Uploaded filename or size did not round-trip".into());
        }
        successful(
            &manager,
            &adb,
            request(
                TaskKind::Download,
                Some(uploaded.clone()),
                Some(local.join("downloads").to_string_lossy().into()),
                None,
            ),
        )
        .await?;
        let downloaded = local.join("downloads/Round trip 日本語");
        if std::fs::read(downloaded.join(filename)).map_err(|e| e.to_string())? != payload {
            return Err("Downloaded bytes differ from uploaded bytes".into());
        }
        if std::fs::read(downloaded.join("Sub folder/nested.txt")).map_err(|e| e.to_string())?
            != b"nested file payload"
        {
            return Err("Nested download bytes differ".into());
        }
        successful(
            &manager,
            &adb,
            request(
                TaskKind::Rename,
                Some(uploaded),
                Some("Renamed ' folder".into()),
                None,
            ),
        )
        .await?;
        successful(
            &manager,
            &adb,
            request(
                TaskKind::Delete,
                Some(format!("{remote}/Renamed ' folder")),
                None,
                None,
            ),
        )
        .await?;
        successful(
            &manager,
            &adb,
            request(
                TaskKind::Install,
                Some(apk.to_string_lossy().into()),
                None,
                None,
            ),
        )
        .await?;
        successful(
            &manager,
            &adb,
            request(
                TaskKind::Install,
                Some(apk.to_string_lossy().into()),
                None,
                None,
            ),
        )
        .await?;
        let details = adb.app_details(serial, package).await?;
        if details.version_name != "1.0" {
            return Err("Installed fixture version did not match".into());
        }
        successful(
            &manager,
            &adb,
            request(
                TaskKind::Export,
                None,
                Some(local.join("exports").to_string_lossy().into()),
                Some(package.into()),
            ),
        )
        .await?;
        let exported = std::fs::read_dir(local.join("exports"))
            .map_err(|e| e.to_string())?
            .next()
            .ok_or("APK export folder is empty")?
            .map_err(|e| e.to_string())?
            .path()
            .join("base.apk");
        if std::fs::read(exported).map_err(|e| e.to_string())?
            != std::fs::read(&apk).map_err(|e| e.to_string())?
        {
            return Err("Exported APK does not match the installed APK".into());
        }
        successful(
            &manager,
            &adb,
            request(TaskKind::Uninstall, None, None, Some(package.into())),
        )
        .await?;

        // These prepared installs touch only the fixture that was absent at entry.
        let source = apk.to_string_lossy().into_owned();
        let preview = installer.inspect(source.clone()).await?;
        let icon = STANDARD.encode(include_bytes!("../icons/icon.png"));
        let mut prepared = request(TaskKind::Install, Some(source), None, None);
        prepared.install_options = Some(InstallOptions {
            source_stamp: preview.source_stamp,
            display_name: Some("Example modified verification".into()),
            icon_png: Some(icon.clone()),
            compatibility: true,
        });
        successful(&manager, &adb, prepared.clone()).await?;
        successful(&manager, &adb, prepared).await?;
        let conflict = finish(
            &manager,
            &adb,
            request(
                TaskKind::Install,
                Some(apk.to_string_lossy().into()),
                None,
                None,
            ),
        )
        .await?;
        if conflict.status != "failed"
            || !conflict
                .detail
                .contains("INSTALL_FAILED_UPDATE_INCOMPATIBLE")
        {
            return Err("A conflicting signature did not refuse the update".into());
        }
        // Export after the refusal to check that the modified installation survived.
        let modified_export = local.join("modified-exports");
        std::fs::create_dir(&modified_export).map_err(|e| e.to_string())?;
        successful(
            &manager,
            &adb,
            request(
                TaskKind::Export,
                None,
                Some(modified_export.to_string_lossy().into()),
                Some(package.into()),
            ),
        )
        .await?;
        let exported = std::fs::read_dir(modified_export)
            .map_err(|e| e.to_string())?
            .next()
            .ok_or("Modified APK export folder is empty")?
            .map_err(|e| e.to_string())?
            .path()
            .join("base.apk");
        let after = installer.inspect(exported.to_string_lossy().into()).await?;
        if after.assets.display_name.as_deref() != Some("Example modified verification")
            || after.assets.icon_data_url.as_deref()
                != Some(&format!("data:image/png;base64,{icon}"))
            || after.verity_signing != Some(false)
        {
            return Err(
                "The prepared installation did not retain its name, icon or compatible signature"
                    .into(),
            );
        }
        successful(
            &manager,
            &adb,
            request(TaskKind::Uninstall, None, None, Some(package.into())),
        )
        .await?;
        Ok(())
    }
    .await;

    // Cleanup is attempted even when a check fails. Never touch an existing user's package.
    if adb
        .apps(serial, false)
        .await
        .unwrap_or_default()
        .iter()
        .any(|a| a.package_name == package)
    {
        let cleanup = successful(
            &manager,
            &adb,
            request(TaskKind::Uninstall, None, None, Some(package.into())),
        )
        .await;
        assert!(
            cleanup.is_ok(),
            "Fixture package cleanup failed: {cleanup:?}"
        );
    }
    let cleanup = successful(
        &manager,
        &adb,
        request(TaskKind::Delete, Some(remote.clone()), None, None),
    )
    .await;
    assert!(cleanup.is_ok(), "Remote scratch cleanup failed: {remote}");
    assert!(local.starts_with(artifact_root.canonicalize().unwrap()));
    std::fs::remove_dir_all(&local).unwrap();
    result.unwrap();
    println!(
        "Device task round-trip passed: transfers, unusual filenames, collision refusal, original and modified APK install/update/export/uninstall, and signature conflict preservation. Test data and disposable signing keys cleaned up."
    );
}
