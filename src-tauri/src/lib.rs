mod adb;
mod tasks;

use adb::{Adb, AppDetails, AppPackage, Device, DeviceInfo, FileEntry};
use tasks::{TaskManager, TaskRequest, TaskSnapshot};
use tauri::Manager;

#[tauri::command]
async fn list_devices(adb: tauri::State<'_, Adb>) -> Result<Vec<Device>, String> {
    adb.devices().await
}

#[tauri::command]
async fn device_info(adb: tauri::State<'_, Adb>, device: String) -> Result<DeviceInfo, String> {
    adb.device_info(&device).await
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
    device: String,
    package: String,
) -> Result<AppDetails, String> {
    adb.app_details(&device, &package).await
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
fn cancel_task(tasks: tauri::State<'_, TaskManager>, id: String) -> Result<(), String> {
    tasks.cancel(&id)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let resource_adb = app.path().resource_dir()?.join("platform-tools/adb.exe");
            let adb_path = if cfg!(debug_assertions) {
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../env/platform-tools/adb.exe")
            } else {
                resource_adb
            };
            app.manage(Adb::new(adb_path));
            app.manage(TaskManager::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_devices,
            device_info,
            list_apps,
            app_details,
            list_files,
            start_task,
            list_tasks,
            cancel_task
        ])
        .run(tauri::generate_context!())
        .expect("Failed to run Quest Manager");
}
