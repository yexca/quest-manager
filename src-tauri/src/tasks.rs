use crate::adb::{
    Adb, friendly_error, normalize_remote, shell_quote, validate_device, validate_filename,
    validate_package, validate_windows_filename,
};
use crate::apk_install::{InstallOptions, Installer, cleanup_result};
use crate::lightning::{self, Lightning};
use crate::obb::{self, CheckedObb, ObbInstall};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::Emitter;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    sync::Semaphore,
};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskKind {
    Install,
    Uninstall,
    Upload,
    Download,
    Export,
    Mkdir,
    Rename,
    Delete,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRequest {
    pub device: String,
    pub kind: TaskKind,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub package_name: Option<String>,
    pub install_options: Option<InstallOptions>,
    pub obb: Option<ObbInstall>,
    pub lightning: Option<lightning::Selection>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSnapshot {
    pub id: String,
    pub revision: u64,
    pub device: String,
    pub kind: TaskKind,
    // An optional refresh hint. Installation still targets only the local APK.
    pub package_name: Option<String>,
    // Retained even when a later OBB transfer or local cleanup fails.
    pub apk_installed: bool,
    pub includes_obb: bool,
    pub label: String,
    pub status: String,
    pub detail: String,
    pub progress: Option<f64>,
    pub created_at: u64,
}

struct TaskRecord {
    snapshot: TaskSnapshot,
    cancelled: Arc<AtomicBool>,
}
type TaskEmitter = Arc<dyn Fn(TaskSnapshot) + Send + Sync>;
struct Inner {
    records: Mutex<BTreeMap<String, TaskRecord>>,
    queue: Arc<Semaphore>,
    counter: AtomicU64,
}

#[derive(Clone)]
pub struct TaskManager {
    inner: Arc<Inner>,
    installer: Option<Installer>,
    lightning: Lightning,
}

impl TaskManager {
    #[cfg(test)]
    pub fn with_installer(installer: Installer) -> Self {
        Self {
            installer: Some(installer),
            ..Self::new()
        }
    }
    pub fn with_lightning(installer: Installer, lightning: Lightning) -> Self {
        Self {
            installer: Some(installer),
            lightning,
            ..Self::new()
        }
    }
    pub fn new() -> Self {
        Self {
            installer: None,
            lightning: Lightning::default(),
            inner: Arc::new(Inner {
                records: Mutex::new(BTreeMap::new()),
                queue: Arc::new(Semaphore::new(1)),
                counter: AtomicU64::new(0),
            }),
        }
    }

    pub fn snapshots(&self) -> Vec<TaskSnapshot> {
        self.inner
            .records
            .lock()
            .unwrap()
            .values()
            .map(|r| r.snapshot.clone())
            .collect()
    }

    pub fn has_active(&self) -> bool {
        self.inner
            .records
            .lock()
            .unwrap()
            .values()
            .any(|record| matches!(record.snapshot.status.as_str(), "queued" | "running"))
    }

    pub fn clear_completed(&self) -> Vec<String> {
        let mut removed = Vec::new();
        self.inner.records.lock().unwrap().retain(|id, record| {
            let active = matches!(record.snapshot.status.as_str(), "queued" | "running");
            if !active {
                removed.push(id.clone());
            }
            active
        });
        removed
    }

    fn update(
        &self,
        emit: &TaskEmitter,
        id: &str,
        status: Option<&str>,
        detail: &str,
        progress: Option<f64>,
    ) {
        let snapshot = {
            let mut records = self.inner.records.lock().unwrap();
            let Some(record) = records.get_mut(id) else {
                return;
            };
            record.snapshot.revision += 1;
            if let Some(status) = status {
                record.snapshot.status = status.into();
            }
            record.snapshot.detail = detail.chars().take(1800).collect();
            record.snapshot.progress = progress;
            record.snapshot.clone()
        };
        emit(snapshot);
    }

    pub fn cancel(&self, id: &str) -> Result<(), String> {
        let records = self.inner.records.lock().unwrap();
        let record = records.get(id).ok_or("This task no longer exists.")?;
        if !["queued", "running"].contains(&record.snapshot.status.as_str()) {
            return Ok(());
        }
        if record.snapshot.status == "running"
            && !matches!(
                record.snapshot.kind,
                TaskKind::Upload | TaskKind::Download | TaskKind::Export
            )
        {
            return Err("This operation cannot be cancelled after it starts.".into());
        }
        record.cancelled.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub fn start(
        &self,
        app: tauri::AppHandle,
        adb: Adb,
        request: TaskRequest,
    ) -> Result<TaskSnapshot, String> {
        self.start_inner(
            Arc::new(move |snapshot| {
                let _ = app.emit("task-updated", snapshot);
            }),
            adb,
            request,
        )
    }

    fn start_inner(
        &self,
        emit: TaskEmitter,
        adb: Adb,
        request: TaskRequest,
    ) -> Result<TaskSnapshot, String> {
        validate_device(&request.device)?;
        validate_request(&request)?;
        // Pin the reviewed assets before queueing; later catalog refreshes cannot retarget them.
        let lightning_assets = request
            .lightning
            .as_ref()
            .map(|selection| self.lightning.resolve(selection))
            .transpose()?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let id = format!(
            "{now}-{}",
            self.inner.counter.fetch_add(1, Ordering::Relaxed)
        );
        let label = task_label(&request);
        let snapshot = TaskSnapshot {
            id: id.clone(),
            revision: 0,
            device: request.device.clone(),
            kind: request.kind,
            package_name: request.package_name.clone(),
            apk_installed: false,
            includes_obb: request.obb.is_some(),
            label,
            status: "queued".into(),
            detail: "Waiting for the previous task".into(),
            progress: None,
            created_at: now,
        };
        let cancelled = Arc::new(AtomicBool::new(false));
        self.inner.records.lock().unwrap().insert(
            id.clone(),
            TaskRecord {
                snapshot: snapshot.clone(),
                cancelled: cancelled.clone(),
            },
        );
        emit(snapshot.clone());
        let manager = self.clone();
        tauri::async_runtime::spawn(async move {
            let context = Context {
                manager,
                emit,
                adb,
                id,
                cancelled,
                device: request.device.clone(),
            };
            let permit = tokio::select! {
                permit = context.manager.inner.queue.acquire() => permit.ok(),
                () = wait_cancelled(context.cancelled.clone()) => None,
            };
            if permit.is_none() || context.cancelled.load(Ordering::Relaxed) {
                context.report(Some("cancelled"), "Cancelled before starting", None);
                return;
            }
            context.report(Some("running"), "Preparing…", None);
            let result = if let Some(assets) = lightning_assets {
                context.install_lightning(assets).await
            } else {
                context.execute(request).await
            };
            match result {
                Ok(detail) => context.report(Some("success"), &detail, Some(100.0)),
                Err(error) if error == "Task cancelled." => context.report(
                    Some("cancelled"),
                    "Cancelled. Temporary transfer files were cleaned up where possible.",
                    None,
                ),
                Err(error) => context.report(Some("failed"), &error, None),
            }
            drop(permit);
        });
        Ok(snapshot)
    }
}

#[derive(Clone)]
struct Context {
    manager: TaskManager,
    emit: TaskEmitter,
    adb: Adb,
    id: String,
    cancelled: Arc<AtomicBool>,
    device: String,
}

impl Context {
    async fn install_lightning(&self, assets: Vec<lightning::Release>) -> Result<String, String> {
        let installer = self
            .manager
            .installer
            .as_ref()
            .ok_or("APK verification is unavailable.")?;
        let parent = installer.storage.join("downloads");
        tokio::fs::create_dir_all(&parent)
            .await
            .map_err(|e| e.to_string())?;
        let directory = parent.join(&self.id);
        tokio::fs::create_dir(&directory)
            .await
            .map_err(|e| format!("Could not create download directory: {e}"))?;
        let result = async {
            let mut verified = Vec::new();
            for (index, asset) in assets.iter().enumerate() {
                let path = directory.join(format!("{index}.apk"));
                lightning::download(asset, &path, &|message, progress| {
                    self.report(None, message, progress)
                })
                .await?;
                self.report(
                    None,
                    "Verifying the downloaded APK and its original signature…",
                    None,
                );
                let apk = installer.inspect(path.to_string_lossy().into()).await?;
                if apk.package_name != asset.package
                    || apk.split
                    || apk.version_code.is_empty()
                    || !lightning::matches_version(&asset.tag, &apk.version_name)
                {
                    return Err(
                        "Downloaded APK identity could not be verified. Installation was stopped."
                            .into(),
                    );
                }
                installer.verify_original(&path, &directory).await?;
                verified.push((asset, path, apk.version_code));
            }
            self.apply_lightning(verified).await
        }
        .await;
        cleanup_result(&directory, result).await
    }

    async fn apply_lightning(
        &self,
        verified: Vec<(&lightning::Release, PathBuf, String)>,
    ) -> Result<String, String> {
        let has_navigator = verified
            .iter()
            .any(|(a, _, _)| a.package == lightning::NAVIGATOR);
        let mut completed = Vec::new();
        let result = async {
            // Preflight every package before the first mutation, on the captured transport.
            let installed = self.adb.apps(&self.device, true).await?;
            if !verified.iter().any(|(a, _, _)| a.package == lightning::LAUNCHER)
                && !installed.iter().any(|a| lightning::VARIANTS.contains(&a.package_name.as_str())) {
                return Err("Install Lightning Launcher before adding the Navigator service.".into());
            }
            for (asset, _, version) in &verified {
                if let Some(current) = installed.iter().find(|a| a.package_name == asset.package)
                    && current.version_code.parse::<u64>().ok().zip(version.parse::<u64>().ok()).is_some_and(|(a, b)| a > b) {
                    return Err(format!("{} is newer on the headset (code {} > {}). Choose a newer release; automatic downgrade is disabled.", asset.package, current.version_code, version));
                }
            }
            for (asset, path, version) in verified {
                let name = if asset.package == lightning::LAUNCHER { "Lightning Launcher" } else { "Navigator service" };
                if installed.iter().any(|a| a.package_name == asset.package && a.version_code == version) {
                    completed.push(format!("{name} already installed"));
                    continue;
                }
                self.report(None, &format!("Installing {name} {}…", asset.tag), None);
                self.process(vec!["install".into(), "-r".into(), path.to_string_lossy().into()], false).await?;
                {
                    let mut records = self.manager.inner.records.lock().unwrap();
                    if let Some(record) = records.get_mut(&self.id) { record.snapshot.apk_installed = true; }
                }
                // An absent refresh hint invalidates both components, including partial completion.
                completed.push(format!("{name} installed"));
                self.adb.apk_paths(&self.device, &asset.package).await?;
            }
            let guide = if has_navigator {
                " Open Lightning Launcher > Settings > Shortcuts > Activate, then enable the service in the headset's Accessibility settings. Installation does not enable it."
            } else { " Open Lightning Launcher from Unknown Sources on your headset." };
            Ok(format!("{}.{}", completed.join(". "), guide))
        }.await;
        result.map_err(|error| if completed.is_empty() { error } else {
            format!("{}. The remaining setup did not finish: {error} Completed installations were kept.", completed.join(". "))
        })
    }

    fn report(&self, status: Option<&str>, detail: &str, progress: Option<f64>) {
        self.manager
            .update(&self.emit, &self.id, status, detail, progress);
    }

    async fn process(&self, arguments: Vec<String>, cancellable: bool) -> Result<String, String> {
        if self.cancelled.load(Ordering::Relaxed) {
            return Err("Task cancelled.".into());
        }
        let mut args = vec!["-s".into(), self.device.clone()];
        args.extend(arguments);
        let mut child = self
            .adb
            .command(&args)
            .spawn()
            .map_err(|e| format!("Could not start ADB: {e}"))?;
        let stdout = child.stdout.take().ok_or("Could not read ADB output.")?;
        let stderr = child.stderr.take().ok_or("Could not read ADB output.")?;
        let output_reader = tokio::spawn(read_output(stdout, self.clone()));
        let error_reader = tokio::spawn(read_output(stderr, self.clone()));
        let result = tokio::select! {
            status = child.wait() => status.map_err(|e| format!("Could not wait for ADB: {e}")),
            () = wait_cancelled(self.cancelled.clone()), if cancellable => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                Err("Task cancelled.".into())
            },
            () = tokio::time::sleep(Duration::from_secs(3600)) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                Err("The operation timed out after one hour. Check the device before retrying.".into())
            }
        };
        let stdout = output_reader.await.unwrap_or_default();
        let stderr = error_reader.await.unwrap_or_default();
        let status = result?;
        let combined = format!("{}\n{}", stdout.trim(), stderr.trim())
            .trim()
            .to_string();
        if !status.success() || combined.contains("Failure [") {
            return Err(friendly_error(&combined));
        }
        Ok(combined)
    }

    async fn execute(&self, request: TaskRequest) -> Result<String, String> {
        use TaskKind::*;
        let source = request.source.as_deref().unwrap_or("");
        let destination = request.destination.as_deref().unwrap_or("");
        let package = request.package_name.as_deref().unwrap_or("");
        match request.kind {
            Install => self.install(&request).await,
            Uninstall => {
                let apps = self.adb.apps(&self.device, false).await?;
                if !apps.iter().any(|a| a.package_name == package) {
                    return Err("Only installed third-party apps can be uninstalled here.".into());
                }
                self.process(vec!["uninstall".into(), package.into()], false)
                    .await?;
                Ok("Application uninstalled".into())
            }
            Upload => self.upload(source, destination).await,
            Download => self.download(source, destination).await,
            Export => self.export(package, destination).await,
            Mkdir => {
                let parent = self.adb.shared_path(&self.device, source, true).await?;
                validate_filename(destination)?;
                let target = format!("{parent}/{destination}");
                self.adb.ensure_missing(&self.device, &target).await?;
                self.process(
                    vec!["shell".into(), format!("mkdir -- {}", shell_quote(&target))],
                    false,
                )
                .await?;
                Ok("Folder created".into())
            }
            Rename => {
                let source = self.mutation_path(source).await?;
                validate_filename(destination)?;
                let parent = source.rsplit_once('/').ok_or("Invalid path.")?.0;
                let target = format!("{parent}/{destination}");
                self.adb.ensure_missing(&self.device, &target).await?;
                self.move_remote(&source, &target).await?;
                Ok("Item renamed".into())
            }
            Delete => {
                let source = self.mutation_path(source).await?;
                self.process(
                    vec![
                        "shell".into(),
                        format!("rm -rf -- {}", shell_quote(&source)),
                    ],
                    false,
                )
                .await?;
                Ok("Item deleted".into())
            }
        }
    }

    async fn install(&self, request: &TaskRequest) -> Result<String, String> {
        let apk = local_source(request.source.as_deref().unwrap_or(""))?;
        if !apk.is_file()
            || !apk
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("apk"))
        {
            return Err("Select a single .apk file to install.".into());
        }
        let prepared = if request.install_options.is_some() || request.obb.is_some() {
            let installer = self
                .manager
                .installer
                .as_ref()
                .ok_or("APK inspection is unavailable.")?;
            if let Some(obb) = &request.obb
                && crate::apk::source_stamp(&apk)? != obb.apk_source_stamp
            {
                return Err(
                    "The APK changed after preview. Select it again before adding OBB files."
                        .into(),
                );
            }
            self.report(None, "Preparing the APK for installation…", None);
            Some(if let Some(options) = &request.install_options {
                installer
                    .prepare(&apk, options, &self.id, &|message| {
                        self.report(None, message, None)
                    })
                    .await?
            } else {
                installer
                    .stage_original(
                        &apk,
                        &request.obb.as_ref().unwrap().apk_source_stamp,
                        &self.id,
                    )
                    .await?
            })
        } else {
            None
        };

        let result = async {
            let mut obb_files = Vec::new();
            let package = if let Some(selection) = &request.obb {
                let package = &prepared.as_ref().ok_or("OBB installation requires APK inspection.")?.preview.package_name;
                obb::package_directory(package)?;
                if request.package_name.as_deref() != Some(package) {
                    return Err("The APK package differs from the review. Select it again before adding OBB files.".into());
                }
                self.report(None, "Checking OBB files and existing headset data…", None);
                let selection = selection.clone();
                obb_files = crate::apk_install::blocking(move || obb::check(&selection)).await?;
                let directory = self.adb.obb_directory(&self.device, package).await?;
                // Check every collision before changing the installed application.
                for file in &obb_files {
                    self.existing_obb(&format!("{directory}/{}", file.file.name), &file.sha256).await?;
                }
                Some(package.as_str())
            } else { None };
            self.report(None, "Transferring APK and installing on the headset…", None);
            let install_path = prepared.as_ref().map(|p| p.path.as_path()).unwrap_or(&apk);
            self.process(vec!["install".into(), "-r".into(), install_path.to_string_lossy().into()], false).await?;
            let installed_snapshot = {
                let mut records = self.manager.inner.records.lock().unwrap();
                records.get_mut(&self.id).map(|record| {
                    record.snapshot.apk_installed = true;
                    if let Some(package) = package { record.snapshot.package_name = Some(package.into()); }
                    record.snapshot.revision += 1;
                    record.snapshot.detail = "APK installed. Finalizing installation…".into();
                    record.snapshot.progress = None;
                    record.snapshot.clone()
                })
            };
            if let Some(snapshot) = installed_snapshot { (self.emit)(snapshot); }
            if let Some(package) = package {
                self.report(None, "APK installed. Preparing OBB transfers…", None);
                let mut completed = 0;
                let transfer = async {
                    self.adb.apk_paths(&self.device, package).await?;
                    let directory = self.adb.obb_directory(&self.device, package).await?;
                    self.process(vec!["shell".into(), format!("mkdir -p -- {}", shell_quote(&directory))], false).await?;
                    for (index, file) in obb_files.iter().enumerate() {
                        self.install_obb(package, file, index, obb_files.len()).await?;
                        completed += 1;
                    }
                    Ok::<(), String>(())
                }.await;
                if let Err(error) = transfer {
                    return Err(obb_failure(completed, obb_files.len(), &error));
                }
            }
            let signature = if request.install_options.is_some() { " with a local compatibility signature. Keep the signing-key backup for future updates" } else { "" };
            Ok(if request.obb.is_some() {
                format!("Application installed{signature}. All {} OBB file(s) verified in {}.", obb_files.len(), obb::package_directory(package.unwrap())?)
            } else { format!("Application installed{signature}.") })
        }.await;
        if let Some(prepared) = prepared {
            cleanup_result(&prepared.directory, result).await
        } else {
            result
        }
    }

    async fn existing_obb(&self, target: &str, expected: &str) -> Result<bool, String> {
        if !self.adb.obb_file_exists(&self.device, target).await? {
            return Ok(false);
        }
        if self.adb.obb_sha256(&self.device, target).await? != expected {
            return Err(format!(
                "A different OBB file already exists at {target}. It was not overwritten. Review the existing file in Files before trying again."
            ));
        }
        Ok(true)
    }

    async fn install_obb(
        &self,
        package: &str,
        file: &CheckedObb,
        index: usize,
        total: usize,
    ) -> Result<(), String> {
        let directory = self.adb.obb_directory(&self.device, package).await?;
        let target = format!("{directory}/{}", file.file.name);
        self.report(
            None,
            &format!("OBB {} of {total}: checking {}…", index + 1, file.file.name),
            None,
        );
        if self.existing_obb(&target, &file.sha256).await? {
            return Ok(());
        }
        if crate::apk::source_stamp(Path::new(&file.file.input.source))?
            != file.file.input.source_stamp
        {
            return Err(format!(
                "{} changed after selection. Select it again.",
                file.file.name
            ));
        }
        let staging = format!("{directory}/.quest-manager-{}-obb-{index}.partial", self.id);
        self.adb.ensure_missing(&self.device, &staging).await?;
        // Atomically reserve this task's file before ADB push, with shell noclobber.
        self.process(
            vec![
                "shell".into(),
                format!("set -C; : > {}", shell_quote(&staging)),
            ],
            false,
        )
        .await.map_err(|error| format!("{error}\nCould not confirm ownership of the temporary OBB file at {staging}. Inspect this path before retrying; it was not removed."))?;
        let transfer = async {
            self.report(
                None,
                &format!(
                    "Uploading OBB {} of {total}: {}…",
                    index + 1,
                    file.file.name
                ),
                None,
            );
            self.process(
                vec![
                    "push".into(),
                    file.file.input.source.clone(),
                    staging.clone(),
                ],
                false,
            )
            .await?;
            self.adb.obb_directory(&self.device, package).await?;
            self.report(
                None,
                &format!(
                    "Verifying OBB {} of {total}: {}…",
                    index + 1,
                    file.file.name
                ),
                None,
            );
            if self.adb.obb_sha256(&self.device, &staging).await? != file.sha256 {
                return Err(format!(
                    "OBB checksum mismatch for {}. The file was not published.",
                    file.file.name
                ));
            }
            self.adb.ensure_missing(&self.device, &target).await?;
            self.move_remote(&staging, &target).await
        }
        .await;
        if let Err(error) = transfer {
            // Never follow a changed parent during cleanup; only remove our temporary file.
            let cleanup = async {
                self.adb.obb_directory(&self.device, package).await?;
                self.adb
                    .shell(&self.device, &format!("rm -f -- {}", shell_quote(&staging)))
                    .await
            }
            .await;
            return Err(if cleanup.is_err() {
                format!(
                    "{error}\nTemporary OBB data may remain at {staging}; cleanup could not be completed."
                )
            } else {
                error
            });
        }
        Ok(())
    }

    async fn mutation_path(&self, path: &str) -> Result<String, String> {
        let path = self.adb.shared_path(&self.device, path, false).await?;
        if [
            "/sdcard/Android",
            "/sdcard/Android/data",
            "/sdcard/Android/obb",
        ]
        .contains(&path.as_str())
        {
            return Err(
                "This shared-storage container is protected. Manage the items inside it instead."
                    .into(),
            );
        }
        Ok(path)
    }

    async fn move_remote(&self, source: &str, target: &str) -> Result<(), String> {
        let source = shell_quote(source);
        let target = shell_quote(target);
        // -n refuses replacement; the final test detects its otherwise-successful no-op.
        self.adb
            .shell(
                &self.device,
                &format!("mv -n -- {source} {target} && [ ! -e {source} ] && [ ! -L {source} ]"),
            )
            .await?;
        Ok(())
    }

    async fn upload(&self, source: &str, destination: &str) -> Result<String, String> {
        let source = local_source(source)?;
        let name = source
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("This local filename is not supported.")?;
        validate_filename(name)?;
        let parent = self
            .adb
            .shared_path(&self.device, destination, true)
            .await?;
        let target = format!("{parent}/{name}");
        let staging = format!("{parent}/.quest-manager-{}.partial", self.id);
        self.adb.ensure_missing(&self.device, &target).await?;
        self.adb.ensure_missing(&self.device, &staging).await?;
        let transfer = async {
            self.report(None, "Uploading to the headset…", None);
            self.process(
                vec![
                    "push".into(),
                    source.to_string_lossy().into(),
                    staging.clone(),
                ],
                true,
            )
            .await?;
            if self.cancelled.load(Ordering::Relaxed) {
                return Err("Task cancelled.".into());
            }
            self.adb.ensure_missing(&self.device, &target).await?;
            self.move_remote(&staging, &target).await?;
            Ok(format!("Uploaded to {target}"))
        }
        .await;
        if let Err(error) = &transfer {
            let cleanup = self
                .adb
                .shell(
                    &self.device,
                    &format!("rm -rf -- {}", shell_quote(&staging)),
                )
                .await;
            if cleanup.is_err() {
                return Err(format!(
                    "{}\nTemporary files may remain at {staging} because the device could not be reached.",
                    error
                ));
            }
        }
        transfer
    }

    async fn download(&self, source: &str, destination: &str) -> Result<String, String> {
        let source = self.adb.shared_path(&self.device, source, false).await?;
        let name = source.rsplit('/').next().ok_or("Invalid source path.")?;
        validate_windows_filename(name)?;
        // Check every descendant before pull to prevent Windows path interpretation.
        self.validate_download_tree(&source).await?;
        let parent = local_directory(destination)?;
        let target = parent.join(name);
        let staging = parent.join(format!(".quest-manager-{}.partial", self.id));
        ensure_local_missing(&target)?;
        ensure_local_missing(&staging)?;
        self.report(None, "Downloading from the headset…", None);
        let result = async {
            self.process(
                vec![
                    "pull".into(),
                    "-a".into(),
                    source,
                    staging.to_string_lossy().into(),
                ],
                true,
            )
            .await?;
            if self.cancelled.load(Ordering::Relaxed) {
                return Err("Task cancelled.".into());
            }
            ensure_local_missing(&target)?;
            tokio::fs::rename(&staging, &target)
                .await
                .map_err(|e| format!("Could not finish the download: {e}"))?;
            Ok(format!("Saved to {}", target.display()))
        }
        .await;
        finish_local_transfer(result, &staging).await
    }

    async fn validate_download_tree(&self, source: &str) -> Result<(), String> {
        let listing = self
            .adb
            .run(vec![
                "-s".into(),
                self.device.clone(),
                "shell".into(),
                format!("find -H {} -printf '%P\\0%M\\0'", shell_quote(source)),
            ])
            .await?;
        let fields: Vec<_> = listing.split(|&c| c == 0).collect();
        if fields.last() != Some(&&b""[..]) || (fields.len() - 1) % 2 != 0 {
            return Err("Could not validate the download filenames.".into());
        }
        let mut names = std::collections::HashSet::new();
        for pair in fields[..fields.len() - 1].chunks_exact(2) {
            let name =
                std::str::from_utf8(pair[0]).map_err(|_| "A filename is not valid UTF-8.")?;
            if !name.is_empty() {
                for component in name.split('/') {
                    validate_windows_filename(component)?;
                }
                if !names.insert(name.to_lowercase()) {
                    return Err("This folder contains names that differ only by letter case and cannot be safely saved together on Windows.".into());
                }
            }
            if pair[1].first() == Some(&b'l') {
                return Err("Folder downloads containing symbolic links are not supported. Download the regular files individually.".into());
            }
        }
        Ok(())
    }

    async fn export(&self, package: &str, destination: &str) -> Result<String, String> {
        let paths = self.adb.apk_paths(&self.device, package).await?;
        let parent = local_directory(destination)?;
        let target = parent.join(format!("{package}-{}", self.id));
        let staging = parent.join(format!(".quest-manager-{}.partial", self.id));
        ensure_local_missing(&target)?;
        ensure_local_missing(&staging)?;
        tokio::fs::create_dir(&staging)
            .await
            .map_err(|e| format!("Could not create the export folder: {e}"))?;
        let result = async {
            for (index, path) in paths.iter().enumerate() {
                let name = path.rsplit('/').next().ok_or("Invalid APK path.")?;
                validate_windows_filename(name)?;
                self.report(
                    None,
                    &format!("Exporting APK {} of {}…", index + 1, paths.len()),
                    None,
                );
                self.process(
                    vec![
                        "pull".into(),
                        path.clone(),
                        staging.join(name).to_string_lossy().into(),
                    ],
                    true,
                )
                .await?;
            }
            if self.cancelled.load(Ordering::Relaxed) {
                return Err("Task cancelled.".into());
            }
            ensure_local_missing(&target)?;
            tokio::fs::rename(&staging, &target)
                .await
                .map_err(|e| format!("Could not finish the export: {e}"))?;
            Ok(format!(
                "Exported {} APK file(s) to {}",
                paths.len(),
                target.display()
            ))
        }
        .await;
        finish_local_transfer(result, &staging).await
    }
}

async fn finish_local_transfer(
    result: Result<String, String>,
    staging: &Path,
) -> Result<String, String> {
    if let Err(error) = &result
        && staging.exists()
    {
        let cleanup = if staging.is_dir() {
            tokio::fs::remove_dir_all(staging).await
        } else {
            tokio::fs::remove_file(staging).await
        };
        if let Err(cleanup_error) = cleanup {
            return Err(format!(
                "{}\nCould not remove temporary item {}: {cleanup_error}",
                error,
                staging.display()
            ));
        }
    }
    result
}

async fn wait_cancelled(cancelled: Arc<AtomicBool>) {
    while !cancelled.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
}

async fn read_output(mut reader: impl AsyncRead + Unpin, context: Context) -> String {
    let mut buffer = [0; 4096];
    let mut tail = String::new();
    while let Ok(length) = reader.read(&mut buffer).await {
        if length == 0 {
            break;
        }
        let chunk = String::from_utf8_lossy(&buffer[..length]);
        tail.push_str(&chunk);
        if tail.len() > 12000 {
            let offset = tail
                .char_indices()
                .find(|(i, _)| *i >= tail.len() - 8000)
                .map(|(i, _)| i)
                .unwrap_or(0);
            tail.drain(..offset);
        }
        if let Some(line) = chunk
            .split(['\r', '\n'])
            .rev()
            .find(|s| !s.trim().is_empty())
        {
            context.report(None, line.trim(), parse_progress(line));
        }
    }
    tail
}

fn parse_progress(line: &str) -> Option<f64> {
    let prefix = line.split_once('%')?.0;
    let number: String = prefix
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    number
        .parse::<f64>()
        .ok()
        .filter(|n| (0.0..=100.0).contains(n))
}

fn local_source(source: &str) -> Result<PathBuf, String> {
    let path = Path::new(source);
    if !path.is_absolute() {
        return Err("Select an absolute local file or folder path.".into());
    }
    std::fs::canonicalize(path).map_err(|e| format!("Could not access the local item: {e}"))
}

fn local_directory(source: &str) -> Result<PathBuf, String> {
    let path = local_source(source)?;
    if !path.is_dir() {
        return Err("Select a destination folder.".into());
    }
    Ok(path)
}

fn ensure_local_missing(path: &Path) -> Result<(), String> {
    if path.try_exists().map_err(|e| e.to_string())? {
        return Err(format!(
            "{} already exists. Choose a different destination.",
            path.display()
        ));
    }
    Ok(())
}

fn validate_request(request: &TaskRequest) -> Result<(), String> {
    if let Some(selection) = &request.lightning {
        if request.kind != TaskKind::Install
            || request.source.is_some()
            || request.destination.is_some()
            || request.package_name.is_some()
            || request.install_options.is_some()
            || request.obb.is_some()
            || (selection.launcher_asset.is_none() && selection.navigator_asset.is_none())
        {
            return Err("Select a Lightning Launcher release without local APK options.".into());
        }
        return Ok(());
    }
    if let Some(obb) = &request.obb {
        if request.kind != TaskKind::Install {
            return Err("OBB files only apply to APK installation.".into());
        }
        obb::package_directory(request.package_name.as_deref().unwrap_or(""))?;
        if obb.apk_source_stamp.is_empty()
            || obb.files.is_empty()
            || obb.files.len() > 128
            || obb.files.iter().any(|file| file.source_stamp.is_empty())
        {
            return Err(
                "Select the APK and between 1 and 128 OBB files again before installing.".into(),
            );
        }
    }
    if let Some(options) = &request.install_options {
        if request.kind != TaskKind::Install {
            return Err("APK preparation options only apply to installation.".into());
        }
        options.validate()?;
    }
    let source = request.source.as_deref().unwrap_or("");
    let destination = request.destination.as_deref().unwrap_or("");
    match request.kind {
        TaskKind::Install => {
            local_source(source)?;
            if let Some(package) = &request.package_name {
                validate_package(package)?;
            }
        }
        TaskKind::Upload => {
            local_source(source)?;
            normalize_remote(destination)?;
        }
        TaskKind::Download => {
            normalize_remote(source)?;
            local_directory(destination)?;
        }
        TaskKind::Mkdir | TaskKind::Rename => {
            normalize_remote(source)?;
            validate_filename(destination)?;
        }
        TaskKind::Delete => {
            normalize_remote(source)?;
        }
        TaskKind::Export => {
            validate_package(request.package_name.as_deref().unwrap_or(""))?;
            local_directory(destination)?;
        }
        TaskKind::Uninstall => validate_package(request.package_name.as_deref().unwrap_or(""))?,
    }
    Ok(())
}

fn task_label(request: &TaskRequest) -> String {
    if let Some(selection) = &request.lightning {
        return match (
            selection.launcher_asset.is_some(),
            selection.navigator_asset.is_some(),
        ) {
            (true, true) => "Install Lightning Launcher + Navigator service",
            (true, false) => "Install Lightning Launcher",
            _ => "Install Navigator service",
        }
        .into();
    }
    let item = request
        .package_name
        .as_deref()
        .or(request.source.as_deref())
        .unwrap_or("")
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("");
    match request.kind {
        TaskKind::Install if request.obb.is_some() => format!(
            "Install {item} + {} OBB file(s)",
            request.obb.as_ref().unwrap().files.len()
        ),
        TaskKind::Mkdir => format!(
            "Create folder {}",
            request.destination.as_deref().unwrap_or("")
        ),
        TaskKind::Rename => format!("Rename {item}"),
        kind => format!("{kind:?} {item}"),
    }
}

fn obb_failure(completed: usize, total: usize, error: &str) -> String {
    format!(
        "APK installed, but OBB data is incomplete ({completed}/{total} files verified). {error}\nThe app and completed OBB files were kept. Select the APK and OBB files again to retry; identical existing OBB files will be reused."
    )
}

#[cfg(test)]
#[path = "device_tests.rs"]
mod device_tests;

#[cfg(test)]
#[path = "obb_tests.rs"]
mod obb_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_snapshot(manager: &TaskManager, id: &str, status: &str) {
        manager.inner.records.lock().unwrap().insert(
            id.into(),
            TaskRecord {
                snapshot: TaskSnapshot {
                    id: id.into(),
                    revision: 0,
                    device: "DEMO-USB-001".into(),
                    kind: TaskKind::Upload,
                    package_name: None,
                    apk_installed: false,
                    includes_obb: false,
                    label: "Upload example.txt".into(),
                    status: status.into(),
                    detail: String::new(),
                    progress: None,
                    created_at: 1,
                },
                cancelled: Arc::new(AtomicBool::new(false)),
            },
        );
    }

    #[test]
    fn exit_guard_and_clearing_use_authoritative_task_states() {
        let manager = TaskManager::new();
        assert!(!manager.has_active());
        for status in ["queued", "running", "success", "failed", "cancelled"] {
            insert_snapshot(&manager, status, status);
        }
        assert!(manager.has_active());
        assert_eq!(
            manager.clear_completed(),
            vec!["cancelled", "failed", "success"]
        );
        assert_eq!(manager.snapshots().len(), 2);
        assert!(manager.has_active());
        // Clearing does not cancel queued or running work.
        assert!(
            manager
                .inner
                .records
                .lock()
                .unwrap()
                .values()
                .all(|r| !r.cancelled.load(Ordering::Relaxed))
        );
        let emit: TaskEmitter = Arc::new(|_| {});
        manager.update(&emit, "queued", Some("cancelled"), "Cancelled", None);
        assert!(manager.has_active());
        manager.update(&emit, "running", Some("success"), "Done", Some(100.0));
        assert!(!manager.has_active());
        assert_eq!(manager.clear_completed().len(), 2);
        assert!(manager.snapshots().is_empty());
    }

    #[test]
    fn task_revisions_advance_and_cleared_records_ignore_late_progress() {
        let manager = TaskManager::new();
        insert_snapshot(&manager, "EXAMPLE-TASK-1", "queued");
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = events.clone();
        let emit: TaskEmitter = Arc::new(move |snapshot| captured.lock().unwrap().push(snapshot));
        manager.update(
            &emit,
            "EXAMPLE-TASK-1",
            Some("running"),
            "Copying",
            Some(10.0),
        );
        manager.update(&emit, "EXAMPLE-TASK-1", None, "Copying", Some(50.0));
        manager.update(
            &emit,
            "EXAMPLE-TASK-1",
            Some("success"),
            "Done",
            Some(100.0),
        );
        assert_eq!(
            events
                .lock()
                .unwrap()
                .iter()
                .map(|t| t.revision)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        manager.clear_completed();
        manager.update(&emit, "EXAMPLE-TASK-1", None, "Late output", None);
        assert!(manager.snapshots().is_empty());
        assert_eq!(events.lock().unwrap().len(), 3);
    }

    #[test]
    fn parses_transfer_progress_without_inventing_percentages() {
        assert_eq!(parse_progress("[ 42%] /sdcard/file: 12%"), Some(42.0));
        assert_eq!(parse_progress("Performing Streamed Install"), None);
        assert_eq!(parse_progress("[101%] invalid"), None);
    }

    #[test]
    fn refuses_mutations_with_unsafe_arguments() {
        let mut request = TaskRequest {
            device: "device".into(),
            kind: TaskKind::Delete,
            source: Some("/data/data".into()),
            destination: None,
            package_name: None,
            install_options: None,
            obb: None,
            lightning: None,
        };
        assert!(validate_request(&request).is_err());
        request.kind = TaskKind::Rename;
        request.source = Some("/sdcard/Download/item".into());
        request.destination = Some("../../elsewhere".into());
        assert!(validate_request(&request).is_err());
    }
}
