use crate::adb::{
    Adb, friendly_error, normalize_remote, shell_quote, validate_device, validate_filename,
    validate_package, validate_windows_filename,
};
use crate::apk_install::{InstallOptions, Installer, cleanup_result};
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
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSnapshot {
    pub id: String,
    pub revision: u64,
    pub device: String,
    pub kind: TaskKind,
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
}

impl TaskManager {
    pub fn with_installer(installer: Installer) -> Self {
        Self {
            installer: Some(installer),
            ..Self::new()
        }
    }
    pub fn new() -> Self {
        Self {
            installer: None,
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
            match context.execute(request).await {
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
            Install => {
                let apk = local_source(source)?;
                if !apk.is_file()
                    || !apk
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("apk"))
                {
                    return Err("Select a single .apk file to install.".into());
                }
                if let Some(options) = &request.install_options {
                    let installer = self
                        .manager
                        .installer
                        .as_ref()
                        .ok_or("APK preparation is unavailable.")?;
                    let prepared = installer
                        .prepare(&apk, options, &self.id, &|message| {
                            self.report(None, message, None)
                        })
                        .await?;
                    self.report(
                        None,
                        &format!(
                            "Verified {}. Installing the prepared APK…",
                            prepared
                                .preview
                                .assets
                                .display_name
                                .as_deref()
                                .unwrap_or(&prepared.preview.package_name)
                        ),
                        None,
                    );
                    let result = self.process(vec!["install".into(), "-r".into(), prepared.path.to_string_lossy().into_owned()], false).await
                        .map(|_| "Application installed with a local compatibility signature. Keep the signing-key backup for future updates.".into());
                    return cleanup_result(&prepared.directory, result).await;
                }
                self.report(
                    None,
                    "Transferring APK and installing on the headset…",
                    None,
                );
                self.process(
                    vec!["install".into(), "-r".into(), apk.to_string_lossy().into()],
                    false,
                )
                .await?;
                Ok("Application installed successfully".into())
            }
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
    let item = request
        .package_name
        .as_deref()
        .or(request.source.as_deref())
        .unwrap_or("")
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("");
    match request.kind {
        TaskKind::Mkdir => format!(
            "Create folder {}",
            request.destination.as_deref().unwrap_or("")
        ),
        TaskKind::Rename => format!("Rename {item}"),
        kind => format!("{kind:?} {item}"),
    }
}

#[cfg(test)]
#[path = "device_tests.rs"]
mod device_tests;

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
        };
        assert!(validate_request(&request).is_err());
        request.kind = TaskKind::Rename;
        request.source = Some("/sdcard/Download/item".into());
        request.destination = Some("../../elsewhere".into());
        assert!(validate_request(&request).is_err());
    }
}
