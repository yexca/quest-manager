use crate::metadata::{ApkFile, AppDetails, parse_details};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, net::SocketAddr, path::PathBuf, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

#[cfg(test)]
#[path = "wireless_tests.rs"]
pub(crate) mod wireless_tests;

#[derive(Deserialize)]
#[serde(tag = "method", rename_all = "lowercase", deny_unknown_fields)]
pub enum WirelessRequest {
    Pair { address: String, code: String },
    Usb { device: String },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WirelessResult {
    pub serial: Option<String>,
    pub message: String,
}

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
pub struct DevicePowerSettings {
    pub stay_awake: Option<bool>,
    pub raw: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPackage {
    pub package_name: String,
    pub version_code: String,
    pub system: bool,
    pub installer: Option<String>,
    pub apk_path: Option<String>,
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

    // Called under TaskManager's global queue permit, only after an explicit action.
    pub async fn wireless_connection(
        &self,
        request: WirelessRequest,
    ) -> Result<WirelessResult, String> {
        match request {
            WirelessRequest::Pair { address, code } => {
                let address = wireless_address(&address)?;
                let code = code.trim();
                if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
                    return Err("Enter the six-digit pairing code shown in the headset.".into());
                }
                let guid = self
                    .pair_secret(&address, code)
                    .await?
                    .ok_or("Pairing completed without a usable device identity. Try USB setup.")?;
                let serial = self.wait_paired_connection(&guid).await?;
                Ok(WirelessResult {
                    serial: Some(serial),
                    message: "Paired and connected over Wi-Fi.".into(),
                })
            }
            WirelessRequest::Usb { device } => {
                validate_device(&device)?;
                if device.starts_with("emulator-") {
                    return Err("Choose a physical headset connected by USB.".into());
                }
                self.usb_tls_setup(&device).await
            }
        }
    }

    async fn usb_tls_setup(&self, device: &str) -> Result<WirelessResult, String> {
        let (identity, existing) = tokio::time::timeout(Duration::from_secs(20), async {
            let raw = self.run(vec!["devices".into(), "-l".into()]).await?;
            if !parse_devices(&String::from_utf8_lossy(&raw)).iter()
                .any(|(t, _)| t.serial == device && t.kind == "usb" && t.state == "device") {
                return Err("The selected USB connection is unavailable. Connect and authorize it, then refresh.".to_string());
            }
            let identity = self.shell(device, "getprop ro.serialno").await?;
            if identity.is_empty() { return Err("Could not verify the USB headset identity.".to_string()); }
            // Reuse only a ready wireless transport for this physical USB headset.
            for (transport, _) in parse_devices(&String::from_utf8_lossy(&raw)) {
                if transport.kind == "wifi" && transport.state == "device"
                    && self.shell(&transport.serial, "getprop ro.serialno").await.as_deref() == Ok(identity.as_str()) {
                    return Ok((identity, Some(transport.serial)));
                }
            }
            Ok((identity, None))
        }).await.map_err(|_| "USB connection checks timed out.")??;
        if let Some(serial) = existing {
            return Ok(WirelessResult {
                message: if serial.contains("_adb-tls-") {
                    "Already paired with this computer and connected over Wi-Fi. No new pairing was needed. You can unplug USB."
                } else {
                    "Already authorized and connected over Wi-Fi. No new pairing was needed. You can unplug USB."
                }.into(),
                serial: Some(serial),
            });
        }
        let (ip, bssid) = tokio::time::timeout(Duration::from_secs(15), async {
            let route =
                wifi_route_address(&self.shell(device, "ip -4 route show dev wlan0").await?)?;
            let ip = route.parse::<SocketAddr>().unwrap().ip();
            let bssid = wifi_bssid(&self.shell(device, "cmd wifi status").await?)?;
            Ok::<_, String>((ip, bssid))
        })
        .await
        .map_err(|_| "USB Wi-Fi network checks timed out.")??;
        let credentials = crate::qr_pairing::Credentials::generate()?;
        let directory = format!("/data/local/tmp/quest-manager-wireless-{}", credentials.id);
        let dex = format!("{directory}/helper.dex");
        let pairing_marker = format!("{directory}/pairing");
        let helper = format!(
            "CLASSPATH={} app_process /system/bin QuestWireless",
            shell_quote(&dex)
        );
        let helper_stop = format!(
            "sh -c {}",
            shell_quote(&format!("{helper} stop >/dev/null 2>&1"))
        );
        let cleanup = format!(
            "if [ -f {marker} ]; then timeout 5 {helper_stop}; fi; rm -f {dex} {marker}; rmdir {directory}",
            dex = shell_quote(&dex),
            marker = shell_quote(&pairing_marker),
            directory = shell_quote(&directory),
            helper_stop = helper_stop
        );
        let mut child = None;
        let mut owns_directory = false;
        let mut paired = false;
        let outcome = tokio::time::timeout(Duration::from_secs(60), async {
            self.shell(device, &format!("umask 077; mkdir {}", shell_quote(&directory))).await
                .map_err(|_| "Could not reserve a temporary USB setup directory. No existing files were changed.")?;
            owns_directory = true;
            // exec-in preserves binary stdin on Windows; ordinary shell input can alter DEX bytes.
            let stage = format!("umask 077; cat > {}", shell_quote(&dex));
            let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/wireless-helper/build/apk/classes.dex"));
            self.private_input(vec!["-s".into(), device.into(), "exec-in".into(), stage], bytes).await?;
            use sha2::Digest;
            let expected = format!("{:x}", sha2::Sha256::digest(bytes));
            let observed = self.shell(device, &format!("chmod 400 {} && sha256sum < {}", shell_quote(&dex), shell_quote(&dex))).await?;
            if observed.split_whitespace().next() != Some(expected.as_str()) {
                return Err("The temporary USB helper transfer could not be verified. Keep USB connected and try again.".into());
            }
            // Android's own deadline also cleans the helper if USB or the desktop disappears.
            let script = format!("trap {} EXIT HUP INT TERM; timeout 75 sh -c {}", shell_quote(&cleanup), shell_quote(&format!("{helper} start")));
            let mut command = self.command(&["-s".into(), device.into(), "shell".into(), script]);
            command.stdin(Stdio::piped()).stderr(Stdio::null());
            child = Some(command.spawn().map_err(|_| "Could not start USB wireless setup.")?);
            let running = child.as_mut().unwrap();
            let input = running.stdin.as_mut().ok_or("USB setup input is unavailable.")?;
            input.write_all(format!("{bssid}\n{}\n{}\n", credentials.service, credentials.secret).as_bytes()).await.map_err(|_| "Could not send USB pairing credentials.")?;
            let mut output = BufReader::new(running.stdout.take().ok_or("USB setup output is unavailable.")?).lines();
            let port = output.next_line().await.map_err(|_| "USB setup did not return a connection port.")?
                .and_then(|line| line.parse::<u16>().ok()).filter(|port| *port > 0)
                .ok_or("Wireless setup could not start. Pairing cannot start in deep sleep: put on the headset or press its power button to wake it, keep the display on, and retry. Existing pairing alone does not mean the headset is awake.")?;
            let address = SocketAddr::new(ip, port).to_string();
            // A successful authenticated connection proves that current ADB trust is reusable.
            // Connection failures are not evidence of an existing pairing.
            let connected = match self.connect_wireless(&address).await {
                Ok(result) => Some(result),
                Err(error) if error.contains("failed to authenticate") || error.contains("needs authorization") => None,
                Err(error) => return Err(error),
            };
            if let Some(result) = connected {
                self.verify_usb_identity(&address, &identity).await?;
                return Ok(result);
            }
            self.shell(device, &format!("touch {}", shell_quote(&pairing_marker))).await?;
            input.write_all(b"pair\n").await.map_err(|_| "Could not request USB pairing.")?;
            if output.next_line().await.map_err(|_| "USB pairing did not start.")?.as_deref() != Some("pairing") {
                return Err("The headset could not start pairing. Check its wireless debugging settings.".into());
            }
            let pair_address = loop {
                if let Some(address) = self.qr_service_address(&credentials.service, true).await? {
                    if address.parse::<SocketAddr>().unwrap().ip() != ip { return Err("The discovered pairing service does not match the USB headset.".into()); }
                    break address;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            };
            let guid = self.qr_pair(&pair_address, &credentials.secret).await?;
            paired = true;
            let connected = self.connect_wireless(&address).await?;
            self.verify_usb_identity(&address, &identity).await?;
            // Quest may hide persist.adb.wifi.guid from shell. USB identity remains authoritative.
            let observed = self.shell(&address, "getprop persist.adb.wifi.guid").await?;
            if !observed.is_empty() && observed != guid { return Err("The wireless pairing identity changed.".into()); }
            Ok(connected)
        }).await.unwrap_or_else(|_| Err("USB wireless setup timed out. Keep USB connected and the headset awake, then try again.".into()));
        if let Some(mut running) = child {
            drop(running.stdin.take());
            if tokio::time::timeout(Duration::from_secs(5), running.wait())
                .await
                .is_err()
            {
                let _ = running.start_kill();
            }
        }
        if !owns_directory {
            return outcome;
        }
        let cleaned = tokio::time::timeout(
            Duration::from_secs(10),
            self.shell(
                device,
                &format!("{cleanup}; test ! -e {}", shell_quote(&directory)),
            ),
        )
        .await;
        match (outcome, cleaned) {
            (Ok(mut result), Ok(Ok(_))) => {
                result.message = if paired { "Paired and connected over Wi-Fi. You can unplug USB." }
                    else { "Already paired with this computer. Reconnected over Wi-Fi without pairing again. You can unplug USB." }.into();
                Ok(result)
            }
            (Ok(mut result), _) => {
                result.message = "Connected over Wi-Fi. Temporary helper cleanup could not be confirmed. The started helper has a deadline; temporary files may remain.".into();
                Ok(result)
            }
            (Err(error), Ok(Ok(_))) => Err(error),
            (Err(error), _) => Err(format!(
                "{error} Temporary helper cleanup could not be confirmed. A started helper session has an on-device deadline."
            )),
        }
    }

    async fn verify_usb_identity(&self, serial: &str, identity: &str) -> Result<(), String> {
        if self.shell(serial, "getprop ro.serialno").await? != identity {
            return Err("The wireless connection does not match the selected USB headset.".into());
        }
        Ok(())
    }

    async fn private_input(&self, args: Vec<String>, bytes: &[u8]) -> Result<(), String> {
        let mut command = self.command(&args);
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = command
            .spawn()
            .map_err(|_| "Could not start USB helper transfer.")?;
        let mut input = child
            .stdin
            .take()
            .ok_or("USB helper input is unavailable.")?;
        input
            .write_all(bytes)
            .await
            .map_err(|_| "Could not transfer the USB helper.")?;
        drop(input);
        if !child
            .wait()
            .await
            .map_err(|_| "USB helper transfer failed.")?
            .success()
        {
            return Err("Could not stage the temporary USB helper.".into());
        }
        Ok(())
    }

    pub async fn wait_paired_connection(&self, guid: &str) -> Result<String, String> {
        tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                if let Some(serial) = self.qr_ready_serial(guid).await? { return Ok(serial); }
                if let Some(address) = self.qr_service_address(guid, false).await? { return self.qr_connect(&address, guid).await; }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }).await.map_err(|_| "Paired, but the headset has not reappeared on Wi-Fi. Keep it awake on the same network and refresh, or use USB setup.".to_string())?
    }

    async fn connect_wireless(&self, address: &str) -> Result<WirelessResult, String> {
        let output = self.run(vec!["connect".into(), address.into()]).await?;
        let output = String::from_utf8_lossy(&output);
        // adb connect can exit successfully while reporting a failed connection.
        if !output.lines().any(|line| {
            line.trim() == format!("connected to {address}")
                || line.trim() == format!("already connected to {address}")
        }) {
            return Err(format!(
                "Could not connect to {address}. Check the address, wireless debugging, and that both devices can reach each other on the same network. {}",
                output.trim().chars().take(500).collect::<String>()
            ));
        }
        let raw = self.run(vec!["devices".into(), "-l".into()]).await?;
        let connections = parse_devices(&String::from_utf8_lossy(&raw));
        let state = connections
            .iter()
            .find(|(t, _)| t.serial == address)
            .map(|(t, _)| t.state.as_str());
        match state {
            Some("device") => Ok(WirelessResult {
                serial: Some(address.into()),
                message: "Connected over Wi-Fi. You can now unplug the USB cable after any USB tasks have finished.".into(),
            }),
            Some("unauthorized") => Err("The headset needs authorization. Accept its debugging prompt, or pair using a new code, then connect again.".into()),
            _ => Err(format!("{address} is not ready yet. Keep the headset awake, check its current connection port, and connect again.")),
        }
    }

    // Shared by explicit code and QR setup. Never put a secret in arguments/errors.
    async fn pair_secret(&self, address: &str, secret: &str) -> Result<Option<String>, String> {
        let mut command = self.command(&["pair".into(), address.into()]);
        command.stdin(Stdio::piped());
        let output = tokio::time::timeout(Duration::from_secs(30), async {
            let mut child = command.spawn()?;
            let mut input = child
                .stdin
                .take()
                .ok_or_else(|| std::io::Error::other("Missing pairing input"))?;
            input.write_all(format!("{secret}\n").as_bytes()).await?;
            drop(input);
            child.wait_with_output().await
        })
        .await
        .map_err(|_| {
            "Pairing timed out. Generate a new QR code or pairing code and try again.".to_string()
        })?
        .map_err(|_| {
            "Could not complete pairing. Check wireless debugging and try a new code.".to_string()
        })?;
        let text = String::from_utf8_lossy(&output.stdout);
        let prefix = format!("Successfully paired to {address}");
        let success = text
            .lines()
            .find_map(|line| {
                line.trim()
                    .strip_prefix("Enter pairing code: ")
                    .unwrap_or(line.trim())
                    .strip_prefix(&prefix)
            })
            .filter(|rest| rest.is_empty() || rest.starts_with(" [guid="));
        if !output.status.success() || success.is_none() {
            return Err("Pairing failed. Check the pairing address or scanner, generate a new code, and keep the headset's pairing screen open.".into());
        }
        Ok(success
            .and_then(|rest| rest.strip_prefix(" [guid=")?.strip_suffix(']'))
            .filter(|guid| {
                !guid.is_empty()
                    && guid.len() <= 63
                    && guid.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            })
            .map(str::to_string))
    }

    pub async fn qr_pair(&self, address: &str, secret: &str) -> Result<String, String> {
        self.pair_secret(&wireless_address(address)?, secret)
            .await?
            .ok_or_else(|| {
                "Pairing completed without a usable device identity. Try USB setup.".into()
            })
    }

    pub async fn qr_service_address(
        &self,
        name: &str,
        pairing: bool,
    ) -> Result<Option<String>, String> {
        let output = self.run(vec!["mdns".into(), "services".into()]).await
            .map_err(|_| "ADB could not discover wireless services. Check the local network and firewall, or try USB setup.".to_string())?;
        mdns_address(&String::from_utf8_lossy(&output), name, pairing)
    }

    pub async fn qr_ready_serial(&self, guid: &str) -> Result<Option<String>, String> {
        let raw = self.run(vec!["devices".into(), "-l".into()]).await?;
        let serial = format!("{guid}._adb-tls-connect._tcp");
        if let Some((transport, _)) = parse_devices(&String::from_utf8_lossy(&raw))
            .into_iter()
            .find(|(t, _)| t.state == "device" && t.serial.trim_end_matches('.') == serial)
        {
            self.qr_verify(&transport.serial, guid).await?;
            return Ok(Some(transport.serial));
        }
        Ok(None)
    }

    pub async fn qr_connect(&self, address: &str, guid: &str) -> Result<String, String> {
        let address = wireless_address(address)?;
        self.connect_wireless(&address).await.map_err(|_| {
            "Paired, but Wi-Fi is not ready. Wake the headset and refresh, or try USB setup."
                .to_string()
        })?;
        if let Some(serial) = self.qr_ready_serial(guid).await? {
            return Ok(serial);
        }
        self.qr_verify(&address, guid).await?;
        Ok(address)
    }

    async fn qr_verify(&self, serial: &str, guid: &str) -> Result<(), String> {
        let observed = self.shell(serial, "getprop persist.adb.wifi.guid").await?;
        // Some Quest builds hide this property from shell. A ready authenticated
        // ADB transport named for the exact paired TLS service is also usable.
        if observed.trim() != guid
            && !(observed.is_empty()
                && serial.trim_end_matches('.') == format!("{guid}._adb-tls-connect._tcp"))
        {
            return Err("The connected device identity could not be verified for this pairing. Try USB setup.".into());
        }
        Ok(())
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

    pub async fn power_settings(&self, device: &str) -> Result<DevicePowerSettings, String> {
        let raw = self
            .shell(device, "settings get global stay_on_while_plugged_in")
            .await?;
        let value = raw.trim();
        let stay_awake = parse_stay_awake(value);
        Ok(DevicePowerSettings {
            stay_awake,
            raw: value.to_string(),
        })
    }

    pub async fn set_stay_awake(
        &self,
        device: &str,
        enabled: bool,
    ) -> Result<DevicePowerSettings, String> {
        let command = if enabled {
            "svc power stayon true"
        } else {
            "svc power stayon false"
        };
        self.shell(device, command).await?;
        self.power_settings(device).await
    }

    pub async fn navigator_enabled(&self, device: &str) -> Result<bool, String> {
        let user = self.shell(device, "am get-current-user").await?;
        if user.is_empty() || !user.bytes().all(|c| c.is_ascii_digit()) {
            return Err("Could not determine the active Android user.".into());
        }
        let values = self.shell(device, &format!("settings --user {user} get secure accessibility_enabled; settings --user {user} get secure enabled_accessibility_services")).await?;
        let mut lines = values.lines();
        let active = lines.next().unwrap_or("").trim();
        if !matches!(active, "0" | "1" | "null") {
            return Err("Could not read Accessibility service status.".into());
        }
        let services = lines
            .next()
            .ok_or("Could not read Accessibility service status.")?;
        Ok(active == "1"
            && services.trim().split(':').any(|service| {
                service
                    .split_once('/')
                    .is_some_and(|(package, _)| package == crate::lightning::NAVIGATOR)
            }))
    }

    pub async fn apps(
        &self,
        device: &str,
        include_system: bool,
    ) -> Result<Vec<AppPackage>, String> {
        let user = self
            .shell(device, "pm list packages -3 -f -i --show-versioncode")
            .await?;
        let mut apps = parse_packages(&user, false);
        if include_system {
            apps.extend(parse_packages(
                &self
                    .shell(device, "pm list packages -s -f -i --show-versioncode")
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

    // A package's OBB directory must resolve to that exact shared-storage location.
    // Missing components are allowed during preflight; existing links are not.
    pub async fn obb_directory(&self, device: &str, package: &str) -> Result<String, String> {
        let directory = crate::obb::package_directory(package)?;
        for path in ["/sdcard/Android", "/sdcard/Android/obb", &directory] {
            let quoted = shell_quote(path);
            let actual = self.shell(device, &format!(
                "if [ -L {quoted} ]; then exit 1; elif [ -e {quoted} ]; then [ -d {quoted} ] && readlink -f -- {quoted}; else printf missing; fi"
            )).await.map_err(|e| format!("Cannot access the OBB directory safely: {e}"))?;
            if actual != "missing" && normalize_remote(&actual)? != path {
                return Err("The OBB directory resolves to a different location.".into());
            }
        }
        Ok(directory)
    }

    pub async fn obb_file_exists(&self, device: &str, path: &str) -> Result<bool, String> {
        let quoted = shell_quote(path);
        let state = self.shell(device, &format!(
            "if [ -L {quoted} ]; then printf conflict; elif [ -e {quoted} ]; then if [ -f {quoted} ]; then printf file; else printf conflict; fi; else printf missing; fi"
        )).await?;
        match state.as_str() {
            "file" => Ok(true),
            "missing" => Ok(false),
            _ => Err(format!("The OBB destination is not a regular file: {path}")),
        }
    }

    pub async fn obb_sha256(&self, device: &str, path: &str) -> Result<String, String> {
        validate_device(device)?;
        let quoted = shell_quote(path);
        // Large expansion files may take longer than the ordinary query timeout.
        let output = tokio::time::timeout(
            Duration::from_secs(3600),
            self.command(&[
                "-s".into(),
                device.into(),
                "shell".into(),
                format!("[ ! -L {quoted} ] && [ -f {quoted} ] && sha256sum < {quoted}"),
            ])
            .output(),
        )
        .await
        .map_err(|_| "OBB verification timed out after one hour.")?
        .map_err(|e| format!("Could not verify the OBB file: {e}"))?;
        if !output.status.success() {
            return Err(friendly_error(&String::from_utf8_lossy(&output.stderr)));
        }
        crate::obb::parse_sha256(&String::from_utf8_lossy(&output.stdout))
    }
}

pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn mdns_address(output: &str, name: &str, pairing: bool) -> Result<Option<String>, String> {
    let service_type = if pairing {
        "_adb-tls-pairing._tcp"
    } else {
        "_adb-tls-connect._tcp"
    };
    let mut addresses = Vec::new();
    for line in output.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() != 3 || fields[0] != name || fields[1].trim_end_matches('.') != service_type
        {
            continue;
        }
        let address = wireless_address(fields[2])?;
        if !addresses.contains(&address) {
            addresses.push(address);
        }
    }
    if addresses.len() > 1 {
        return Err("Multiple devices advertised this QR pairing identity. Cancel and generate a new QR code.".into());
    }
    Ok(addresses.pop())
}

fn wireless_address(value: &str) -> Result<String, String> {
    let address: SocketAddr = value.trim().parse().map_err(|_| {
        "Enter an IP address and port, for example 192.0.2.10:5555 or [2001:db8::10]:5555."
            .to_string()
    })?;
    if address.port() == 0
        || address.ip().is_unspecified()
        || address.ip().is_multicast()
        || address.ip().is_loopback()
        || matches!(address.ip(), std::net::IpAddr::V4(ip) if ip.is_broadcast())
    {
        return Err("Enter the headset's network IP address and a port from 1 to 65535.".into());
    }
    Ok(address.to_string())
}

fn wifi_bssid(status: &str) -> Result<String, String> {
    let mut values = Vec::new();
    for (_, tail) in status
        .match_indices("BSSID:")
        .map(|(index, _)| (index, &status[index + 6..]))
    {
        let value = tail
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_end_matches(',')
            .to_ascii_lowercase();
        let parts: Vec<_> = value.split(':').collect();
        if parts.len() != 6
            || !parts
                .iter()
                .all(|part| part.len() == 2 && part.bytes().all(|b| b.is_ascii_hexdigit()))
            || matches!(
                value.as_str(),
                "00:00:00:00:00:00" | "02:00:00:00:00:00" | "ff:ff:ff:ff:ff:ff"
            )
        {
            return Err("Could not identify the headset's current Wi-Fi network. Connect it to Wi-Fi first.".into());
        }
        if !values.contains(&value) {
            values.push(value);
        }
    }
    if values.len() != 1 {
        return Err(
            "Could not identify one current Wi-Fi network. Check Wi-Fi inside the headset.".into(),
        );
    }
    Ok(values.remove(0))
}

fn wifi_route_address(routes: &str) -> Result<String, String> {
    let mut addresses = std::collections::BTreeSet::new();
    for line in routes.lines() {
        let words: Vec<_> = line.split_whitespace().collect();
        for pair in words.windows(2).filter(|pair| pair[0] == "src") {
            if let Ok(ip) = pair[1].parse::<std::net::Ipv4Addr>() {
                addresses.insert(wireless_address(&format!("{ip}:5555"))?);
            }
        }
    }
    if addresses.len() != 1 {
        return Err("Could not determine one Wi-Fi address for the headset. Connect it to Wi-Fi and try again, or use QR or code pairing in its Wireless debugging settings.".into());
    }
    Ok(addresses.into_iter().next().unwrap())
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
        "The installed app and this APK have different signatures. A re-signed APK cannot normally update an original installation. Nothing was uninstalled. Keep the same local signing key for subsequent modified updates."
    } else if raw.contains("INSTALL_PARSE_FAILED_NO_CERTIFICATES")
        && raw.contains("integer overflow")
    {
        "Android could not verify this APK because of a signature integer overflow. Try Modify and re-sign APK > Compatibility install. This changes the signing identity."
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
            let identity = words.next()?;
            // APK paths may contain '=' in Android's randomized directory names.
            let (apk_path, package_name) = match identity.rsplit_once('=') {
                Some((path, package)) => (Some(path.to_string()), package),
                None => (None, identity),
            };
            let fields: Vec<_> = words.collect();
            Some(AppPackage {
                package_name: package_name.to_string(),
                version_code: fields
                    .iter()
                    .find_map(|s| s.strip_prefix("versionCode:"))
                    .unwrap_or("Unknown")
                    .to_string(),
                system,
                installer: fields
                    .iter()
                    .find_map(|s| s.strip_prefix("installer="))
                    .filter(|value| !value.is_empty() && *value != "null")
                    .map(str::to_string),
                apk_path,
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

fn parse_stay_awake(raw: &str) -> Option<bool> {
    match raw.trim() {
        "0" => Some(false),
        "null" | "" => None,
        value => value.parse::<u32>().ok().map(|mask| mask != 0),
    }
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
    fn package_listing_preserves_installer_and_apk_identity() {
        let apps = parse_packages(
            "package:/data/app/~~EXAMPLE==/com.example.game-EXAMPLE==/base.apk=com.example.game installer=com.oculus.ocms versionCode:100\npackage:com.example.other versionCode:5 installer=null\npackage:com.example.unknown\n",
            false,
        );
        assert_eq!(apps[0].package_name, "com.example.game");
        assert_eq!(apps[0].version_code, "100");
        assert_eq!(apps[0].installer.as_deref(), Some("com.oculus.ocms"));
        assert_eq!(
            apps[0].apk_path.as_deref(),
            Some("/data/app/~~EXAMPLE==/com.example.game-EXAMPLE==/base.apk")
        );
        assert_eq!(apps[1].installer, None);
        assert_eq!(apps[1].apk_path, None);
        assert_eq!(apps[2].version_code, "Unknown");
        assert!(
            parse_packages(
                "package:com.example.system installer=com.android.shell versionCode:9",
                true
            )[0]
            .system
        );
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

    #[test]
    fn stay_awake_parser_handles_android_power_masks() {
        assert_eq!(parse_stay_awake("0"), Some(false));
        assert_eq!(parse_stay_awake("15"), Some(true));
        assert_eq!(parse_stay_awake("null"), None);
        assert_eq!(parse_stay_awake("unexpected"), None);
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
