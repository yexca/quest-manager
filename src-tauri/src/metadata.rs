use crate::adb::Adb;
use crate::apk;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use tokio::sync::Semaphore;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppAssets {
    pub display_name: Option<String>,
    pub icon_data_url: Option<String>,
    pub vr_features: Vec<String>,
    pub signing_schemes: Vec<String>,
    pub certificate_sha256: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApkFile {
    pub path: String,
    pub size: Option<u64>,
    pub modified: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Permission {
    pub name: String,
    pub granted: Option<bool>,
    pub kind: String,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDetails {
    pub package_name: String,
    pub version_name: String,
    pub version_code: String,
    pub apk_paths: Vec<String>,
    pub apk_files: Vec<ApkFile>,
    pub apk_size: Option<u64>,
    pub first_install_time: Option<String>,
    pub last_update_time: Option<String>,
    pub installer: Option<String>,
    pub min_sdk: Option<String>,
    pub target_sdk: Option<String>,
    pub primary_abi: Option<String>,
    pub secondary_abi: Option<String>,
    pub uid: Option<String>,
    pub android_user: u32,
    pub enabled: Option<bool>,
    pub state_flags: Vec<String>,
    pub permissions: Vec<Permission>,
    pub assets: AppAssets,
}

// dumpsys is a diagnostic format, so absent fields remain unknown. Only the
// installed package block and selected Android user's state are authoritative.
pub fn parse_details(package: &str, dump: &str, user: u32) -> AppDetails {
    let marker = format!("Package [{package}]");
    let lines: Vec<_> = dump.lines().collect();
    let start = lines
        .iter()
        .position(|line| line.trim().starts_with(&marker));
    let block: Vec<_> = start
        .map(|index| {
            lines[index + 1..]
                .iter()
                .copied()
                .take_while(|line| {
                    !line.trim().starts_with("Package [")
                        && !line.starts_with("Hidden system packages:")
                        && !line.starts_with("Dexopt state:")
                })
                .collect()
        })
        .unwrap_or_default();
    let value = |key: &str| -> Option<String> {
        block
            .iter()
            .find_map(|line| {
                line.split_whitespace()
                    .find_map(|word| word.strip_prefix(key))
            })
            .filter(|v| !v.is_empty() && *v != "null")
            .map(str::to_string)
    };
    let timestamp = |key: &str| -> Option<String> {
        block
            .iter()
            .find_map(|line| line.trim().strip_prefix(key))
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    };
    let user_marker = format!("User {user}:");
    let user_start = block
        .iter()
        .position(|line| line.trim().starts_with(&user_marker));
    let user_lines: Vec<_> = user_start
        .map(|i| {
            block[i..]
                .iter()
                .copied()
                .take_while(|line| {
                    !line.trim().starts_with("User ") || line.trim().starts_with(&user_marker)
                })
                .collect()
        })
        .unwrap_or_default();
    let state = user_lines.first().copied().unwrap_or("");
    let state_value = |key: &str| {
        state
            .split_whitespace()
            .find_map(|word| word.strip_prefix(key))
    };
    let enabled = state_value("enabled=").and_then(|value| match value {
        "1" => Some(true),
        "2" | "3" | "4" => Some(false),
        // A default setting is not proof that the manifest enables the app.
        _ => None,
    });
    let mut permissions: BTreeMap<String, Permission> = BTreeMap::new();
    let mut section = "";
    let common_end = block
        .iter()
        .position(|line| line.trim().starts_with("User "))
        .unwrap_or(block.len());
    for line in block[..common_end].iter().chain(user_lines.iter()) {
        let line = line.trim();
        if [
            "requested permissions:",
            "install permissions:",
            "runtime permissions:",
        ]
        .contains(&line)
        {
            section = line;
            continue;
        }
        if line.ends_with(':') || line.starts_with("User ") {
            section = "";
        }
        let Some(name) = line
            .split([':', ' '])
            .next()
            .filter(|name| name.contains('.') && !name.contains('='))
        else {
            continue;
        };
        if section.is_empty() {
            continue;
        }
        let entry = permissions.entry(name.to_string()).or_insert(Permission {
            name: name.into(),
            granted: None,
            kind: "Requested".into(),
        });
        if line.contains("granted=true") || line.contains("granted=false") {
            entry.granted = Some(line.contains("granted=true"));
            entry.kind = if section == "runtime permissions:" {
                "Runtime"
            } else {
                "Install"
            }
            .into();
        }
    }
    AppDetails {
        package_name: package.into(),
        version_name: value("versionName=").unwrap_or_else(|| "Unknown".into()),
        version_code: value("versionCode=").unwrap_or_else(|| "Unknown".into()),
        first_install_time: timestamp("firstInstallTime="),
        last_update_time: timestamp("lastUpdateTime="),
        installer: value("installerPackageName="),
        min_sdk: value("minSdk="),
        target_sdk: value("targetSdk="),
        primary_abi: value("primaryCpuAbi="),
        secondary_abi: value("secondaryCpuAbi="),
        uid: value("appId=").or_else(|| value("userId=")),
        android_user: user,
        enabled,
        state_flags: ["hidden", "suspended", "stopped", "notLaunched"]
            .into_iter()
            .filter(|key| state_value(&format!("{key}=")) == Some("true"))
            .map(str::to_string)
            .collect(),
        permissions: permissions.into_values().collect(),
        ..Default::default()
    }
}

pub struct MetadataService {
    aapt: PathBuf,
    cache: PathBuf,
    // Serialize extraction and clearing; queued callers cannot flood ADB.
    gate: Arc<Semaphore>,
}

impl MetadataService {
    pub fn new(aapt: PathBuf, cache: PathBuf) -> Self {
        Self {
            aapt,
            cache,
            gate: Arc::new(Semaphore::new(1)),
        }
    }

    pub async fn details(
        &self,
        adb: &Adb,
        device: &str,
        package: &str,
    ) -> Result<AppDetails, String> {
        let permit = self
            .gate
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| "Metadata reader is unavailable.")?;
        let mut details = adb.app_details(device, package).await?;
        // Include the transport and every APK's identity. Permission/state data is
        // queried fresh and is never served from the persistent artwork cache.
        let key = cache_key(device, &details);
        let cache_file = self.cache.join(format!("{key}.json"));
        let cached = std::fs::metadata(&cache_file)
            .ok()
            .filter(|m| m.len() <= 4 * 1024 * 1024)
            .and_then(|_| std::fs::read(&cache_file).ok())
            .and_then(|bytes| serde_json::from_slice::<AppAssets>(&bytes).ok());
        if let Some(assets) = cached {
            details.assets = assets;
            return Ok(details);
        }
        let Some(base) = details
            .apk_files
            .iter()
            .find(|file| file.path.ends_with("/base.apk"))
            .or(details.apk_files.first())
            .cloned()
        else {
            return Ok(details);
        };
        let adb = adb.clone();
        let device = device.to_string();
        let aapt = self.aapt.clone();
        let cache = self.cache.clone();
        details.assets = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            match apk::read_assets(&adb, &device, &base, &aapt, &cache) {
                Ok(assets) => {
                    // Never cache transient failures or an unknown APK identity.
                    if base.size.is_some() && base.modified.is_some() && assets.display_name.is_some() {
                        let _ = std::fs::create_dir_all(&cache);
                        if let Ok(data) = serde_json::to_vec(&assets) { let _ = std::fs::write(&cache_file, data); }
                        prune_cache(&cache);
                    }
                    assets
                },
                Err(_) => AppAssets { notes: vec!["Artwork could not be read. Refresh to retry; package details are still available.".into()], ..Default::default() },
            }
        }).await.map_err(|_| "The application metadata reader stopped unexpectedly.")?;
        Ok(details)
    }

    pub async fn clear(&self) -> Result<(), String> {
        let _permit = self
            .gate
            .acquire()
            .await
            .map_err(|_| "Metadata reader is unavailable.")?;
        if let Ok(entries) = std::fs::read_dir(&self.cache) {
            for entry in entries
                .flatten()
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
            {
                std::fs::remove_file(entry.path())
                    .map_err(|_| "Could not clear the application metadata cache.")?;
            }
        }
        Ok(())
    }
}

fn cache_key(device: &str, details: &AppDetails) -> String {
    let mut hash = Sha256::new();
    hash.update(b"metadata-v1\0");
    for value in [device, &details.package_name, &details.version_code] {
        hash.update(value.as_bytes());
        hash.update([0]);
    }
    for file in &details.apk_files {
        hash.update(file.path.as_bytes());
        hash.update(file.size.unwrap_or(0).to_le_bytes());
        hash.update(file.modified.unwrap_or(0).to_le_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn prune_cache(cache: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(cache) else {
        return;
    };
    let mut entries: Vec<_> = entries
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|e| {
            e.metadata()
                .ok()
                .map(|m| (e.path(), m.len(), m.modified().ok()))
        })
        .collect();
    entries.sort_by_key(|e| e.2);
    let mut total: u64 = entries.iter().map(|e| e.1).sum();
    let mut count = entries.len();
    for (path, size, _) in entries {
        if total <= 64 * 1024 * 1024 && count <= 512 {
            break;
        }
        if std::fs::remove_file(path).is_ok() {
            total -= size;
            count -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_current_package_and_user_without_leaking_other_grants() {
        let dump = "Packages:\n  Package [com.example.game] (example):\n    versionCode=23 minSdk=26 targetSdk=34\n    versionName=2.3\n    firstInstallTime=2025-01-01 10:00:00\n    installerPackageName=null\n    requested permissions:\n      android.permission.CAMERA\n      com.example.permission.PLAY\n    User 0: hidden=false stopped=true enabled=2\n      runtime permissions:\n        android.permission.CAMERA: granted=false, flags=[]\n    User 10: hidden=false enabled=1\n      runtime permissions:\n        android.permission.CAMERA: granted=true, flags=[]\n  Package [com.example.other] (example):\n    versionName=99\n";
        let details = parse_details("com.example.game", dump, 0);
        assert_eq!(details.version_name, "2.3");
        assert_eq!(details.target_sdk.as_deref(), Some("34"));
        assert_eq!(
            details.first_install_time.as_deref(),
            Some("2025-01-01 10:00:00")
        );
        assert_eq!(details.enabled, Some(false));
        assert_eq!(details.installer, None);
        assert_eq!(details.permissions.len(), 2);
        assert_eq!(details.permissions[0].granted, Some(false));
        assert_eq!(details.permissions[1].granted, None);
        assert_eq!(
            parse_details("com.example.missing", dump, 0).version_name,
            "Unknown"
        );
    }

    #[test]
    fn cache_changes_when_device_or_same_version_apk_changes() {
        let mut details = AppDetails {
            package_name: "com.example.game".into(),
            version_code: "1".into(),
            apk_files: vec![ApkFile {
                path: "/data/app/example/base.apk".into(),
                size: Some(100),
                modified: Some(1),
            }],
            ..Default::default()
        };
        let key = cache_key("DEMO-USB", &details);
        assert_ne!(key, cache_key("DEMO-WIFI", &details));
        details.apk_files[0].modified = Some(2);
        assert_ne!(key, cache_key("DEMO-USB", &details));
    }

    #[tokio::test]
    #[ignore = "Read-only application metadata test requiring an authorized Quest"]
    async fn connected_device_metadata() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let adb = Adb::new(root.join("env/platform-tools/adb.exe"));
        let devices = adb.devices().await.expect("Could not discover devices.");
        let device = devices
            .iter()
            .find(|device| device.model.to_lowercase().contains("quest"))
            .expect("No Quest is connected.");
        let transport = device
            .transports
            .iter()
            .find(|transport| transport.state == "device")
            .expect("The Quest is not authorized.");
        let apps = adb
            .apps(&transport.serial, false)
            .await
            .expect("Could not read applications.");
        let app = apps
            .first()
            .expect("No third-party application is installed.");
        let service = MetadataService::new(
            root.join("env/aapt2/aapt2.exe"),
            root.join("env/test-artifacts/app-metadata"),
        );
        let details = service
            .details(&adb, &transport.serial, &app.package_name)
            .await
            .expect("Could not read application metadata.");
        assert!(
            details.apk_size.is_some_and(|size| size > 0),
            "APK sizes were unavailable."
        );
        assert!(
            details.assets.display_name.is_some(),
            "The application label was unavailable."
        );
        assert!(
            details.enabled.is_some(),
            "The application state was unavailable."
        );
        let cached = service
            .details(&adb, &transport.serial, &app.package_name)
            .await
            .expect("Could not read cached metadata.");
        assert!(
            cached.assets.display_name == details.assets.display_name,
            "Cached artwork did not match."
        );
        assert!(
            cached.assets.icon_data_url == details.assets.icon_data_url,
            "Cached icon did not match."
        );
        service
            .clear()
            .await
            .expect("Could not clear test metadata.");
        println!("Application metadata, APK byte ranges, and cache round trip passed.");
    }
}
