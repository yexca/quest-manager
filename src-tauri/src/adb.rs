use crate::metadata::{ApkFile, AppDetails, parse_details};
use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf, process::Stdio, time::Duration};
use tokio::process::Command;

#[derive(Clone)]
pub struct Adb {
    pub path: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transport {
    pub serial: String,
    pub kind: String,
    pub state: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub model: String,
    pub transports: Vec<Transport>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub model: String,
    pub android_version: String,
    pub battery_level: Option<u8>,
    pub charging: bool,
    pub storage_total: u64,
    pub storage_used: u64,
    pub storage_available: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPackage {
    pub package_name: String,
    pub version_code: String,
    pub system: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub size: u64,
    pub modified_at: f64,
}

impl Adb {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn command(&self, args: &[String]) -> Command {
        let mut command = Command::new(&self.path);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x08000000);
        command
    }

    pub async fn run(&self, args: Vec<String>) -> Result<Vec<u8>, String> {
        let output = tokio::time::timeout(Duration::from_secs(30), self.command(&args).output())
            .await
            .map_err(|_| {
                "The device did not respond within 30 seconds. Check its connection.".to_string()
            })?
            .map_err(|e| {
                format!(
                    "Could not start ADB: {e}. Run run-install.ps1 or check the application files."
                )
            })?;
        if !output.status.success() {
            return Err(friendly_error(&format!(
                "{}\n{}",
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout)
            )));
        }
        Ok(output.stdout)
    }

    pub async fn shell(&self, device: &str, command: &str) -> Result<String, String> {
        validate_device(device)?;
        let output = self
            .run(vec![
                "-s".into(),
                device.into(),
                "shell".into(),
                command.into(),
            ])
            .await?;
        Ok(String::from_utf8_lossy(&output).trim().to_string())
    }

    pub async fn devices(&self) -> Result<Vec<Device>, String> {
        let raw = self.run(vec!["devices".into(), "-l".into()]).await?;
        let connections = parse_devices(&String::from_utf8_lossy(&raw));
        let mut devices: BTreeMap<String, Device> = BTreeMap::new();
        for (transport, model) in connections {
            let id = if transport.state == "device" {
                self.shell(&transport.serial, "getprop ro.serialno")
                    .await
                    .ok()
                    .filter(|id| !id.is_empty())
                    .unwrap_or_else(|| transport.serial.clone())
            } else {
                transport.serial.clone()
            };
            let device = devices.entry(id.clone()).or_insert(Device {
                id,
                model,
                transports: Vec::new(),
            });
            device.transports.push(transport);
        }
        for device in devices.values_mut() {
            device
                .transports
                .sort_by_key(|t| (t.state != "device", t.kind != "usb"));
        }
        Ok(devices.into_values().collect())
    }

    pub async fn device_info(&self, device: &str) -> Result<DeviceInfo, String> {
        let (properties, battery, storage) = tokio::try_join!(
            self.shell(
                device,
                "getprop ro.product.model; getprop ro.build.version.release"
            ),
            self.shell(device, "dumpsys battery"),
            self.shell(device, "df -k /sdcard")
        )?;
        let mut properties = properties.lines();
        let (storage_total, storage_used, storage_available) = parse_storage(&storage)?;
        Ok(DeviceInfo {
            model: properties.next().unwrap_or("Quest").trim().to_string(),
            android_version: properties.next().unwrap_or("Unknown").trim().to_string(),
            battery_level: battery.lines().find_map(|l| {
                l.trim()
                    .strip_prefix("level:")
                    .and_then(|v| v.trim().parse().ok())
            }),
            charging: battery
                .lines()
                .any(|l| (l.contains("powered:")) && l.trim().ends_with("true")),
            storage_total,
            storage_used,
            storage_available,
        })
    }

    pub async fn apps(
        &self,
        device: &str,
        include_system: bool,
    ) -> Result<Vec<AppPackage>, String> {
        let user = self
            .shell(device, "pm list packages -3 --show-versioncode")
            .await?;
        let mut apps = parse_packages(&user, false);
        if include_system {
            apps.extend(parse_packages(
                &self
                    .shell(device, "pm list packages -s --show-versioncode")
                    .await?,
                true,
            ));
        }
        apps.sort_by(|a, b| {
            a.package_name
                .to_lowercase()
                .cmp(&b.package_name.to_lowercase())
        });
        Ok(apps)
    }

    pub async fn apk_paths(&self, device: &str, package: &str) -> Result<Vec<String>, String> {
        validate_package(package)?;
        let output = self
            .shell(device, &format!("pm path {}", shell_quote(package)))
            .await?;
        let paths: Vec<_> = output
            .lines()
            .filter_map(|l| l.strip_prefix("package:").map(|p| p.trim().to_string()))
            .collect();
        if paths.is_empty() {
            return Err("This application is no longer installed.".into());
        }
        Ok(paths)
    }

    pub async fn app_details(&self, device: &str, package: &str) -> Result<AppDetails, String> {
        validate_package(package)?;
        let dump_command = format!("dumpsys package {}", shell_quote(package));
        let (paths, dump, user) = tokio::try_join!(
            self.apk_paths(device, package),
            self.shell(device, &dump_command),
            self.shell(device, "am get-current-user")
        )?;
        let user = user
            .parse::<u32>()
            .map_err(|_| "Could not determine the active Android user.")?;
        let mut details = parse_details(package, &dump, user);
        if details.enabled.is_none() {
            details.enabled = self
                .shell(
                    device,
                    &format!("pm list packages -e --user {user} {}", shell_quote(package)),
                )
                .await
                .ok()
                .map(|output| {
                    parse_packages(&output, false)
                        .iter()
                        .any(|app| app.package_name == package)
                });
        }
        let quoted_paths = paths
            .iter()
            .map(|p| shell_quote(p))
            .collect::<Vec<_>>()
            .join(" ");
        let stats = self
            .shell(device, &format!("stat -c '%s %Y' -- {quoted_paths}"))
            .await
            .ok();
        let stats: Vec<_> = stats.as_deref().unwrap_or("").lines().collect();
        details.apk_files = paths
            .iter()
            .enumerate()
            .map(|(index, path)| {
                let mut fields = stats.get(index).copied().unwrap_or("").split_whitespace();
                ApkFile {
                    path: path.clone(),
                    size: fields.next().and_then(|v| v.parse().ok()),
                    modified: fields.next().and_then(|v| v.parse().ok()),
                }
            })
            .collect();
        details.apk_size = details
            .apk_files
            .iter()
            .try_fold(0u64, |total, file| total.checked_add(file.size?));
        details.apk_paths = paths;
        Ok(details)
    }

    // Resolve symlinks on the device as well as validating the lexical path.
    pub async fn shared_path(
        &self,
        device: &str,
        path: &str,
        allow_root: bool,
    ) -> Result<String, String> {
        let path = normalize_remote(path)?;
        if !allow_root && path == "/sdcard" {
            return Err("The storage root cannot be changed.".into());
        }
        let actual = self
            .shell(device, &format!("readlink -f -- {}", shell_quote(&path)))
            .await?;
        let canonical = normalize_remote(&actual)
            .map_err(|_| "This path resolves outside shared storage.".to_string())?;
        if !allow_root && canonical == "/sdcard" {
            return Err("The storage root cannot be changed.".into());
        }
        Ok(path)
    }

    pub async fn files(&self, device: &str, path: &str) -> Result<Vec<FileEntry>, String> {
        let path = self.shared_path(device, path, true).await?;
        // NUL-separated fields preserve spaces, quotes, tabs and newlines in names.
        let output = self
            .run(vec![
                "-s".into(),
                device.into(),
                "shell".into(),
                format!(
                    "find -H {} -mindepth 1 -maxdepth 1 -printf '%f\\0%M\\0%s\\0%T@\\0'",
                    shell_quote(&path)
                ),
            ])
            .await?;
        parse_files(&output, &path)
    }

    pub async fn ensure_missing(&self, device: &str, path: &str) -> Result<(), String> {
        let quoted = shell_quote(path);
        let exists = self
            .shell(
                device,
                &format!("if [ -e {quoted} ] || [ -L {quoted} ]; then printf exists; fi"),
            )
            .await?;
        if !exists.is_empty() {
            return Err("An item with that name already exists. Choose another name or remove the existing item first.".into());
        }
        Ok(())
    }
}

pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub fn validate_device(value: &str) -> Result<(), String> {
    if value.is_empty() || value.starts_with('-') || value.chars().any(char::is_control) {
        return Err("Select a connected device first.".into());
    }
    Ok(())
}

pub fn validate_package(value: &str) -> Result<(), String> {
    if value.is_empty()
        || !value.contains('.')
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'_')
    {
        return Err("Invalid application package name.".into());
    }
    Ok(())
}

pub fn normalize_remote(value: &str) -> Result<String, String> {
    if value.contains('\0') {
        return Err("A path cannot contain a NUL character.".into());
    }
    let value = value
        .strip_prefix("/storage/emulated/0")
        .filter(|rest| rest.is_empty() || rest.starts_with('/'))
        .map(|rest| format!("/sdcard{rest}"))
        .unwrap_or_else(|| value.to_string());
    let mut parts = Vec::new();
    for part in value.split('/') {
        match part {
            "" | "." => (),
            ".." => {
                if parts.len() <= 1 {
                    return Err("Only shared storage (/sdcard) is available.".into());
                }
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    if !value.starts_with('/') || parts.first() != Some(&"sdcard") {
        return Err("Only shared storage (/sdcard) is available.".into());
    }
    Ok(format!("/{}", parts.join("/")))
}

pub fn validate_filename(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\0')
    {
        return Err("Enter a file or folder name without slashes.".into());
    }
    Ok(())
}

pub fn validate_windows_filename(value: &str) -> Result<(), String> {
    validate_filename(value)?;
    let stem = value.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if value
        .chars()
        .any(|c| c.is_control() || "<>:\\|?*\"".contains(c))
        || value.ends_with(['.', ' '])
        || reserved.contains(&stem.as_str())
    {
        return Err("This name cannot be saved on Windows. Rename it on the device first.".into());
    }
    Ok(())
}

pub fn friendly_error(raw: &str) -> String {
    let raw = raw.trim();
    let hint = if raw.contains("unauthorized") {
        "Unlock your headset and allow USB debugging."
    } else if raw.contains("offline")
        || raw.contains("no devices")
        || raw.contains("not found") && raw.contains("device")
    {
        "The device is disconnected. Reconnect it and refresh."
    } else if raw.contains("INSTALL_FAILED_UPDATE_INCOMPATIBLE") {
        "The installed app and this APK have different signatures."
    } else if raw.contains("INSTALL_FAILED_VERSION_DOWNGRADE") {
        "This APK is older than the installed version."
    } else if raw.contains("INSTALL_FAILED_INSUFFICIENT_STORAGE") || raw.contains("No space left") {
        "There is not enough free storage on the destination."
    } else if raw.contains("INSTALL_FAILED_NO_MATCHING_ABIS") {
        "This APK does not support the headset's CPU architecture."
    } else if raw.contains("INSTALL_FAILED_MISSING_SPLIT") {
        "This app requires multiple APK files. Single-APK installation cannot install it."
    } else if raw.contains("Permission denied") {
        "ADB does not have permission to access this item."
    } else {
        "The operation failed."
    };
    if raw.is_empty() {
        hint.into()
    } else {
        format!("{hint}\n{}", raw.chars().take(1500).collect::<String>())
    }
}

fn parse_devices(raw: &str) -> Vec<(Transport, String)> {
    raw.lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let serial = words.next()?;
            if serial == "List" || serial.starts_with('*') {
                return None;
            }
            let state = words.next()?;
            if ![
                "device",
                "offline",
                "unauthorized",
                "recovery",
                "sideload",
                "bootloader",
                "no",
            ]
            .contains(&state)
            {
                return None;
            }
            let model = words
                .find_map(|w| w.strip_prefix("model:"))
                .unwrap_or("Android device")
                .replace('_', " ");
            Some((
                Transport {
                    serial: serial.into(),
                    kind: if serial.contains(':') || serial.contains("_adb-tls-") {
                        "wifi".into()
                    } else {
                        "usb".into()
                    },
                    state: state.into(),
                },
                model,
            ))
        })
        .collect()
}

fn parse_packages(raw: &str, system: bool) -> Vec<AppPackage> {
    raw.lines()
        .filter_map(|line| {
            let mut words = line.strip_prefix("package:")?.split_whitespace();
            Some(AppPackage {
                package_name: words.next()?.to_string(),
                version_code: words
                    .find_map(|s| s.strip_prefix("versionCode:"))
                    .unwrap_or("Unknown")
                    .to_string(),
                system,
            })
        })
        .collect()
}

fn parse_storage(raw: &str) -> Result<(u64, u64, u64), String> {
    let fields: Vec<_> = raw
        .lines()
        .last()
        .unwrap_or("")
        .split_whitespace()
        .collect();
    let number = |index: usize| {
        fields
            .get(index)
            .and_then(|s| s.parse::<u64>().ok())
            .and_then(|v| v.checked_mul(1024))
            .ok_or_else(|| "Could not read device storage information.".to_string())
    };
    Ok((number(1)?, number(2)?, number(3)?))
}

fn parse_files(raw: &[u8], parent: &str) -> Result<Vec<FileEntry>, String> {
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let fields: Vec<_> = raw.split(|&b| b == 0).collect();
    if fields.last() != Some(&&b""[..]) || (fields.len() - 1) % 4 != 0 {
        return Err("The device returned an unsupported directory listing.".into());
    }
    let mut entries = Vec::new();
    for group in fields[..fields.len() - 1].chunks_exact(4) {
        let name = std::str::from_utf8(group[0])
            .map_err(|_| "A filename is not valid UTF-8.".to_string())?
            .to_string();
        validate_filename(&name)?;
        let kind = match group[1].first() {
            Some(b'd') => "directory",
            Some(b'-') => "file",
            Some(b'l') => "symlink",
            _ => "other",
        };
        let size = String::from_utf8_lossy(group[2])
            .parse()
            .map_err(|_| "Invalid file size in device response.".to_string())?;
        let modified_at = String::from_utf8_lossy(group[3])
            .parse::<f64>()
            .map_err(|_| "Invalid file timestamp in device response.".to_string())?
            * 1000.0;
        entries.push(FileEntry {
            path: format!("{parent}/{name}"),
            name,
            kind: kind.into(),
            size,
            modified_at,
        });
    }
    entries.sort_by(|a, b| {
        (a.kind != "directory", a.name.to_lowercase())
            .cmp(&(b.kind != "directory", b.name.to_lowercase()))
    });
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_paths_cannot_escape_shared_storage() {
        assert_eq!(
            normalize_remote("/storage/emulated/0/Download/../Movies").unwrap(),
            "/sdcard/Movies"
        );
        for bad in [
            "/data/data",
            "/sdcard/../data",
            "/sdcard2/file",
            "sdcard/a",
            "/storage/emulated/01/a",
            "/sdcard/\0",
        ] {
            assert!(normalize_remote(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn shell_arguments_are_quoted_as_single_literals() {
        assert_eq!(shell_quote("a'b;$(id)"), "'a'\"'\"'b;$(id)'");
        assert!(validate_package("com.game;id").is_err());
        assert!(validate_package("jp.co.game_2").is_ok());
    }

    #[test]
    fn directory_parser_preserves_unusual_names() {
        let entries = parse_files(b"quote'\tline\n.apk\0-rw-rw----\x00123\x001700000000.5\0folder\0drwxrwx---\x004096\x001700000001\0", "/sdcard").unwrap();
        assert_eq!(entries[0].name, "folder");
        assert_eq!(entries[1].name, "quote'\tline\n.apk");
        assert_eq!(entries[1].size, 123);
        assert_eq!(entries[1].modified_at, 1_700_000_000_500.0);
        assert!(parse_files(b"bad\0listing", "/sdcard").is_err());
    }

    #[test]
    fn windows_download_names_cannot_be_paths_or_devices() {
        for bad in ["..\\escape", "CON.txt", "a:b", "file.", "LPT1", "bad\nname"] {
            assert!(validate_windows_filename(bad).is_err(), "{bad}");
        }
        assert!(validate_windows_filename("日本語 file.apk").is_ok());
    }

    #[test]
    fn device_and_storage_parsers_handle_adb_output() {
        // Fictional identifiers and an RFC 5737 documentation address.
        let devices = parse_devices(
            "List of devices attached\nEXAMPLE-USB device product:example model:Demo_Headset\n192.0.2.10:5555 device model:Demo_Headset\nEXAMPLE-UNAUTHORIZED unauthorized\n",
        );
        assert_eq!(devices.len(), 3);
        assert_eq!(devices[1].0.kind, "wifi");
        assert_eq!(devices[2].0.state, "unauthorized");
        assert_eq!(parse_storage("Filesystem 1K-blocks Used Available Use% Mounted on\n/dev/fuse 120 100 20 84% /storage/emulated").unwrap(), (122880, 102400, 20480));
    }

    #[tokio::test]
    #[ignore = "Read-only test requiring an authorized Quest connected to ADB"]
    async fn connected_device_smoke() {
        let adb = Adb::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../env/platform-tools/adb.exe"),
        );
        let devices = adb.devices().await.unwrap();
        let device = devices
            .iter()
            .find(|d| d.model.contains("Quest"))
            .expect("Connect an authorized Quest headset");
        let transport = device
            .transports
            .iter()
            .find(|t| t.state == "device")
            .expect("Authorize USB debugging");
        let info = adb.device_info(&transport.serial).await.unwrap();
        let apps = adb.apps(&transport.serial, false).await.unwrap();
        let files = adb.files(&transport.serial, "/sdcard").await.unwrap();
        assert!(info.storage_total > 0);
        assert!(!apps.is_empty());
        assert!(
            files
                .iter()
                .any(|entry| entry.name == "Android" && entry.kind == "directory")
        );
        let details = adb
            .app_details(&transport.serial, &apps[0].package_name)
            .await
            .unwrap();
        assert!(!details.apk_paths.is_empty());
        assert!(adb.files(&transport.serial, "/data/data").await.is_err());
        println!("Read-only device smoke test passed.");
    }
}
