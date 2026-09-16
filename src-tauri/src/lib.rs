mod adb;
mod apk;
mod apk_edit;
mod apk_install;
mod device_profiles;
mod lightning;
mod metadata;
mod obb;
mod qr_pairing;
mod tasks;

use adb::{Adb, AppPackage, Device, DeviceInfo, DevicePowerSettings, FileEntry};
use device_profiles::{DevicePreferences, DeviceProfile, DeviceProfileStore};
use metadata::{AppDetails, MetadataService};
use tasks::{TaskManager, TaskRequest, TaskSnapshot};
use tauri::{Emitter, Manager};

#[tauri::command]
async fn open_project_repository() -> Result<(), String> {
    open_repository("https://github.com/yexca/quest-manager").await
}

#[tauri::command]
async fn open_lightning_repository() -> Result<(), String> {
    open_repository("https://github.com/threethan/LightningLauncher").await
}

async fn open_repository(url: &str) -> Result<(), String> {
    // Fixed destination: the webview cannot supply URLs, paths or shell arguments.
    let powershell = std::path::PathBuf::from(
        std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()),
    )
    .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        Adb::new(powershell)
            .command(&[
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-Command".into(),
                format!("$ErrorActionPreference='Stop'; Start-Process -FilePath '{url}'"),
            ])
            .output(),
    )
    .await
    .map_err(|_| "Opening the project repository timed out.".to_string())?
    .map_err(|e| format!("Could not open the project repository: {e}"))?;
    if !output.status.success() {
        return Err(
            "Could not open your browser. Visit https://github.com/yexca/quest-manager manually."
                .into(),
        );
    }
    Ok(())
}

#[tauri::command]
async fn lightning_releases(
    service: tauri::State<'_, lightning::Lightning>,
    refresh: bool,
) -> Result<lightning::Catalog, String> {
    service.catalog(refresh).await
}

#[tauri::command]
async fn lightning_recommendation(
    service: tauri::State<'_, lightning::Lightning>,
    tag: String,
) -> Result<lightning::Recommendation, String> {
    service.recommendation(tag).await
}

#[tauri::command]
async fn navigator_enabled(adb: tauri::State<'_, Adb>, device: String) -> Result<bool, String> {
    adb.navigator_enabled(&device).await
}

#[tauri::command]
async fn inspect_apk(
    installer: tauri::State<'_, apk_install::Installer>,
    source: String,
) -> Result<apk::LocalApk, String> {
    installer.inspect(source).await
}

#[tauri::command]
async fn inspect_obb_files(sources: Vec<String>) -> Result<Vec<obb::LocalObb>, String> {
    tauri::async_runtime::spawn_blocking(move || obb::inspect(sources))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn list_devices(adb: tauri::State<'_, Adb>) -> Result<Vec<Device>, String> {
    adb.devices().await
}

#[tauri::command]
fn device_preferences(store: tauri::State<'_, DeviceProfileStore>) -> DevicePreferences {
    store.get()
}

#[tauri::command]
fn save_device_profile(
    store: tauri::State<'_, DeviceProfileStore>,
    profile: DeviceProfile,
) -> Result<DevicePreferences, String> {
    store.save_profile(profile)
}

#[tauri::command]
fn set_device_auto_switch(
    store: tauri::State<'_, DeviceProfileStore>,
    enabled: bool,
) -> Result<DevicePreferences, String> {
    store.set_auto_switch(enabled)
}

#[tauri::command]
async fn wireless_connection(
    adb: tauri::State<'_, Adb>,
    tasks: tauri::State<'_, TaskManager>,
    request: adb::WirelessRequest,
) -> Result<adb::WirelessResult, String> {
    tasks.wireless_connection(&adb, request).await
}

#[tauri::command]
async fn device_info(adb: tauri::State<'_, Adb>, device: String) -> Result<DeviceInfo, String> {
    adb.device_info(&device).await
}

#[tauri::command]
async fn device_power_settings(
    adb: tauri::State<'_, Adb>,
    device: String,
) -> Result<DevicePowerSettings, String> {
    adb.power_settings(&device).await
}

#[tauri::command]
async fn set_device_stay_awake(
    adb: tauri::State<'_, Adb>,
    tasks: tauri::State<'_, TaskManager>,
    device: String,
    enabled: bool,
) -> Result<DevicePowerSettings, String> {
    tasks.set_stay_awake(&adb, device, enabled).await
}

#[tauri::command]
async fn start_wireless_qr(
    adb: tauri::State<'_, Adb>,
    tasks: tauri::State<'_, TaskManager>,
) -> Result<qr_pairing::Snapshot, String> {
    tasks.start_wireless_qr(adb.inner().clone())
}

#[tauri::command]
fn wireless_qr_status(
    tasks: tauri::State<'_, TaskManager>,
    id: String,
) -> Result<qr_pairing::Snapshot, String> {
    tasks.wireless_qr_status(&id)
}

#[tauri::command]
fn cancel_wireless_qr(tasks: tauri::State<'_, TaskManager>, id: String) -> Result<(), String> {
    tasks.cancel_wireless_qr(&id)
}

#[tauri::command]
async fn list_apps(
    adb: tauri::State<'_, Adb>,
    device: String,
    include_system: bool,
) -> Result<Vec<AppPackage>, String> {
    adb.apps(&device, include_system).await
}

#[tauri::command]
async fn app_details(
    adb: tauri::State<'_, Adb>,
    metadata: tauri::State<'_, MetadataService>,
    device: String,
    package: String,
) -> Result<AppDetails, String> {
    metadata.details(&adb, &device, &package).await
}

#[tauri::command]
async fn clear_metadata_cache(metadata: tauri::State<'_, MetadataService>) -> Result<(), String> {
    metadata.clear().await
}

#[tauri::command]
async fn list_files(
    adb: tauri::State<'_, Adb>,
    device: String,
    path: String,
) -> Result<Vec<FileEntry>, String> {
    adb.files(&device, &path).await
}

#[tauri::command]
fn start_task(
    app: tauri::AppHandle,
    adb: tauri::State<'_, Adb>,
    tasks: tauri::State<'_, TaskManager>,
    request: TaskRequest,
) -> Result<TaskSnapshot, String> {
    tasks.start(app, adb.inner().clone(), request)
}

#[tauri::command]
fn list_tasks(tasks: tauri::State<'_, TaskManager>) -> Vec<TaskSnapshot> {
    tasks.snapshots()
}

#[tauri::command]
fn clear_completed_tasks(tasks: tauri::State<'_, TaskManager>) -> Vec<String> {
    tasks.clear_completed()
}

// Called only by the explicit exit confirmation, never by a task event.
#[tauri::command]
fn exit_with_active_tasks(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn cancel_task(tasks: tauri::State<'_, TaskManager>, id: String) -> Result<(), String> {
    tasks.cancel(&id)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && window.state::<TaskManager>().has_active_work()
            {
                api.prevent_close();
                let _ = window.emit("app-close-blocked", ());
            }
        })
        .setup(|app| {
            let resource_adb = app.path().resource_dir()?.join("platform-tools/adb.exe");
            let adb_path = if cfg!(debug_assertions) {
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../env/platform-tools/adb.exe")
            } else {
                resource_adb
            };
            app.manage(Adb::new(adb_path));
            let (aapt, cache) = if cfg!(debug_assertions) {
                let env = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../env");
                (env.join("aapt2/aapt2.exe"), env.join("cache/app-metadata"))
            } else {
                (
                    app.path().resource_dir()?.join("aapt2/aapt2.exe"),
                    app.path().app_cache_dir()?.join("app-metadata"),
                )
            };
            let (apk_tools, apk_storage) = if cfg!(debug_assertions) {
                let env = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../env");
                (env.join("apk-tools"), env.join("local-data/apk-install"))
            } else {
                (
                    app.path().resource_dir()?.join("apk-tools"),
                    app.path().app_local_data_dir()?.join("apk-install"),
                )
            };
            let installer = apk_install::Installer::new(apk_tools, aapt.clone(), apk_storage);
            let device_settings = if cfg!(debug_assertions) {
                let env = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../env");
                env.join("local-data/device-settings.json")
            } else {
                app.path()
                    .app_local_data_dir()?
                    .join("device-settings.json")
            };
            app.manage(MetadataService::new(aapt, cache));
            let lightning = lightning::Lightning::default();
            app.manage(TaskManager::with_lightning(
                installer.clone(),
                lightning.clone(),
            ));
            app.manage(lightning);
            app.manage(installer);
            app.manage(DeviceProfileStore::new(device_settings));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            open_project_repository,
            open_lightning_repository,
            lightning_releases,
            lightning_recommendation,
            navigator_enabled,
            list_devices,
            device_preferences,
            save_device_profile,
            set_device_auto_switch,
            wireless_connection,
            start_wireless_qr,
            wireless_qr_status,
            cancel_wireless_qr,
            device_info,
            device_power_settings,
            set_device_stay_awake,
            list_apps,
            app_details,
            clear_metadata_cache,
            inspect_apk,
            inspect_obb_files,
            list_files,
            start_task,
            list_tasks,
            clear_completed_tasks,
            exit_with_active_tasks,
            cancel_task
        ])
        .run(tauri::generate_context!())
        .expect("Failed to run Quest Manager");
}
