//! Wireless protocol checks against a host-only executable; no real device access.
use super::*;
use std::{fs, sync::OnceLock};

fn scratch() -> PathBuf {
    let mut random = [0; 8];
    getrandom::fill(&mut random).unwrap();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../env/test-artifacts")
        .join(format!("wireless-{:x}", u64::from_le_bytes(random)));
    fs::create_dir_all(&path).unwrap();
    path
}

pub(crate) struct Fixture {
    pub(crate) root: PathBuf,
    pub(crate) adb: Adb,
}
impl Fixture {
    pub(crate) fn new(mode: &str) -> Self {
        static EXE: OnceLock<PathBuf> = OnceLock::new();
        let exe = EXE.get_or_init(|| {
            let path = scratch().join("mock-wireless-adb.exe");
            let status = std::process::Command::new("rustc")
                .args(["--edition=2024"])
                .arg(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("../tests/fixtures/mock-wireless-adb.rs"),
                )
                .arg("-o")
                .arg(&path)
                .status()
                .unwrap();
            assert!(status.success());
            path
        });
        let root = scratch();
        let path = root.join("mock-wireless-adb.exe");
        fs::copy(exe, &path).unwrap();
        fs::write(root.join("mode"), mode).unwrap();
        use sha2::Digest;
        fs::write(
            root.join("helper-hash"),
            format!(
                "{:x}",
                sha2::Sha256::digest(include_bytes!(concat!(
                    env!("OUT_DIR"),
                    "/wireless-helper/build/apk/classes.dex"
                )))
            ),
        )
        .unwrap();
        Self {
            root,
            adb: Adb::new(path),
        }
    }
    pub(crate) fn commands(&self) -> String {
        fs::read_to_string(self.root.join("commands")).unwrap_or_default()
    }
    async fn connect(&self) -> Result<WirelessResult, String> {
        self.adb.connect_wireless("192.0.2.10:5555").await
    }
    async fn usb(&self, device: &str) -> Result<WirelessResult, String> {
        self.adb
            .wireless_connection(WirelessRequest::Usb {
                device: device.into(),
            })
            .await
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // Windows may briefly retain an executable handle after client cancellation.
        for _ in 0..20 {
            if fs::remove_dir_all(&self.root).is_ok() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn qr_mdns_matches_exact_identity_and_type_and_refuses_ambiguity() {
    let output = "List of discovered mdns services\nstudio-DEMO-OLD _adb-tls-pairing._tcp 192.0.2.11:37123\nstudio-DEMO-QR-other _adb-tls-pairing._tcp 192.0.2.12:37123\nstudio-DEMO-QR _adb-tls-connect._tcp 192.0.2.10:5555\nstudio-DEMO-QR _adb-tls-pairing._tcp. 192.0.2.10:37123\n";
    assert_eq!(
        mdns_address(output, "studio-DEMO-QR", true)
            .unwrap()
            .as_deref(),
        Some("192.0.2.10:37123")
    );
    assert!(
        mdns_address(output, "studio-DEMO-MISSING", true)
            .unwrap()
            .is_none()
    );
    assert!(
        mdns_address(
            &format!("{output}studio-DEMO-QR _adb-tls-pairing._tcp 192.0.2.11:37123"),
            "studio-DEMO-QR",
            true
        )
        .is_err()
    );
    for bad in [
        "example.com:5555",
        "127.0.0.1:5555",
        "192.0.2.10:0",
        "192.0.2.10:5555;reboot",
    ] {
        assert!(
            mdns_address(
                &format!("studio-DEMO-QR _adb-tls-pairing._tcp {bad}"),
                "studio-DEMO-QR",
                true
            )
            .is_err()
        );
    }
    assert_eq!(
        mdns_address(
            "EXAMPLE-GUID _adb-tls-connect._tcp [2001:db8::10]:5555",
            "EXAMPLE-GUID",
            false
        )
        .unwrap()
        .as_deref(),
        Some("[2001:db8::10]:5555")
    );
}

#[tokio::test]
async fn disconnect_targets_one_verified_wifi_transport_and_preserves_pairing() {
    for serial in ["192.0.2.10:37001", "DEMO-GUID._adb-tls-connect._tcp"] {
        let fixture = Fixture::new("disconnect-success");
        fs::write(fixture.root.join("connected"), serial).unwrap();
        fs::write(fixture.root.join("paired"), "").unwrap();
        fixture
            .adb
            .disconnect_wireless(DisconnectWirelessRequest {
                device: serial.into(),
                physical_id: "DEMO-HEADSET".into(),
            })
            .await
            .unwrap();
        assert!(fixture.root.join("paired").exists());
        let commands = fixture.commands();
        assert_eq!(
            commands
                .lines()
                .filter(|line| line.starts_with("disconnect"))
                .collect::<Vec<_>>(),
            vec![format!("disconnect {serial}")]
        );
        assert!(!commands.contains("kill-server"));
    }
}

#[tokio::test]
async fn disconnect_rejects_usb_unknown_wrong_device_and_automatic_reconnection() {
    for (serial, identity) in [
        ("DEMO-USB-TARGET", "DEMO-HEADSET"),
        ("192.0.2.11:37001", "DEMO-HEADSET"),
        ("192.0.2.10:37001", "DEMO-OTHER"),
        ("", "DEMO-HEADSET"),
        ("--all", "DEMO-HEADSET"),
    ] {
        let fixture = Fixture::new("disconnect-success");
        fs::write(fixture.root.join("connected"), "192.0.2.10:37001").unwrap();
        assert!(
            fixture
                .adb
                .disconnect_wireless(DisconnectWirelessRequest {
                    device: serial.into(),
                    physical_id: identity.into(),
                })
                .await
                .is_err()
        );
        assert!(
            !fixture
                .commands()
                .lines()
                .any(|line| line.starts_with("disconnect"))
        );
    }
    let fixture = Fixture::new("disconnect-reconnect");
    fs::write(fixture.root.join("connected"), "192.0.2.10:37001").unwrap();
    assert!(
        fixture
            .adb
            .disconnect_wireless(DisconnectWirelessRequest {
                device: "192.0.2.10:37001".into(),
                physical_id: "DEMO-HEADSET".into(),
            })
            .await
            .unwrap_err()
            .contains("reconnected")
    );
}

#[test]
fn wireless_addresses_reject_commands_and_invalid_endpoints() {
    assert_eq!(
        wireless_address(" 192.0.2.10:5555 ").unwrap(),
        "192.0.2.10:5555"
    );
    assert_eq!(
        wireless_address("[2001:db8::10]:37123").unwrap(),
        "[2001:db8::10]:37123"
    );
    for bad in [
        "",
        "-s",
        "example.com:5555",
        "192.0.2.10",
        "192.0.2.10:0",
        "192.0.2.10:65536",
        "192.0.2.10:5555;reboot",
        "192.0.2.10:5555\nkill-server",
        "0.0.0.0:5555",
        "127.0.0.1:5555",
        "255.255.255.255:5555",
        "[ff02::1]:5555",
    ] {
        assert!(wireless_address(bad).is_err(), "{bad}");
    }
    assert!(
        serde_json::from_str::<WirelessRequest>(
            r#"{"method":"address","address":"192.0.2.10:5555"}"#
        )
        .is_err()
    );
    assert!(
        serde_json::from_str::<WirelessRequest>(
            r#"{"method":"usb","device":"DEMO-USB","command":"reboot"}"#
        )
        .is_err()
    );
}

#[tokio::test]
async fn reconnect_without_usb_uses_trust_and_verifies_physical_identity() {
    for service in [Some("DEMO-GUID".to_string()), None] {
        let fixture = Fixture::new("reconnect-success");
        let result = fixture
            .adb
            .reconnect_wireless(ReconnectWirelessRequest {
                physical_id: "DEMO-HEADSET".into(),
                address: if service.is_none() {
                    Some("192.0.2.10:5555".into())
                } else {
                    None
                },
                service,
            })
            .await
            .unwrap();
        assert_eq!(result.status, "connected");
        assert_eq!(result.serial.as_deref(), Some("192.0.2.10:5555"));
        let commands = fixture.commands();
        assert!(commands.contains("connect 192.0.2.10:5555"));
        assert!(commands.contains("-s 192.0.2.10:5555 shell getprop ro.serialno"));
        assert!(
            !commands
                .lines()
                .any(|line| line.starts_with("pair ") || line.starts_with("disconnect "))
        );
        assert!(!commands.contains("app_process"));
    }
    let fixture = Fixture::new("reconnect-success");
    fs::write(fixture.root.join("connected"), "192.0.2.10:5555").unwrap();
    assert_eq!(
        fixture
            .adb
            .reconnect_wireless(ReconnectWirelessRequest {
                physical_id: "DEMO-HEADSET".into(),
                service: None,
                address: None,
            })
            .await
            .unwrap()
            .status,
        "connected"
    );
    assert!(
        !fixture
            .commands()
            .lines()
            .any(|line| line.starts_with("connect "))
    );
}

#[tokio::test]
async fn reconnect_never_turns_missing_network_or_ambiguity_into_pairing() {
    for (mode, service, expected) in [
        ("reconnect-missing", None, "notFound"),
        ("reconnect-success", None, "chooseAddress"),
        ("reconnect-unique", None, "connected"),
        ("reconnect-ambiguous", Some("DEMO-GUID"), "chooseAddress"),
        ("reconnect-auth", Some("DEMO-GUID"), "pairingRequired"),
        ("refused", Some("DEMO-GUID"), "unavailable"),
        (
            "reconnect-wrong-device",
            Some("DEMO-GUID"),
            "identityMismatch",
        ),
        ("reconnect-query-fail", Some("DEMO-GUID"), "unavailable"),
    ] {
        let fixture = Fixture::new(mode);
        let result = fixture
            .adb
            .reconnect_wireless(ReconnectWirelessRequest {
                physical_id: "DEMO-HEADSET".into(),
                service: service.map(str::to_string),
                address: None,
            })
            .await
            .unwrap();
        assert_eq!(result.status, expected);
        if expected == "connected" {
            assert_eq!(result.serial.as_deref(), Some("192.0.2.10:5555"));
            assert!(fixture.commands().contains("connect 192.0.2.10:5555"));
        } else {
            assert!(result.serial.is_none());
        }
        assert!(
            !fixture
                .commands()
                .lines()
                .any(|line| line.starts_with("pair "))
        );
        if matches!(expected, "notFound" | "chooseAddress") {
            assert!(
                !fixture
                    .commands()
                    .lines()
                    .any(|line| line.starts_with("connect "))
            );
        }
        if expected == "identityMismatch" || mode == "reconnect-query-fail" {
            let commands = fixture.commands();
            let cleanup: Vec<_> = commands
                .lines()
                .filter(|line| line.starts_with("disconnect "))
                .collect();
            assert_eq!(cleanup, vec!["disconnect 192.0.2.10:5555"]);
            assert!(!fixture.commands().contains("disconnect\n"));
        }
    }
}

#[tokio::test]
async fn reconnect_validates_addresses_before_adb_and_filters_discovery() {
    let fixture = Fixture::new("reconnect-success");
    for address in [
        "192.0.2.10:5555;reboot",
        "example.com:5555",
        "127.0.0.1:5555",
        "192.0.2.10:0",
    ] {
        assert!(
            fixture
                .adb
                .reconnect_wireless(ReconnectWirelessRequest {
                    physical_id: "DEMO-HEADSET".into(),
                    service: None,
                    address: Some(address.into()),
                })
                .await
                .is_err()
        );
    }
    assert!(fixture.commands().is_empty());
    let endpoints = reconnect_endpoints(
        "DEMO-GUID _adb-tls-connect._tcp 192.0.2.10:5555\nDEMO-GUID _adb-tls-connect._tcp. 192.0.2.10:5555\nDEMO-PAIR _adb-tls-pairing._tcp 192.0.2.10:37123\nDEMO-INVALID _adb-tls-connect._tcp 127.0.0.1:5555\nDEMO-V6 _adb-tls-connect._tcp [2001:db8::10]:5555\n",
    );
    assert_eq!(endpoints.len(), 2);
    assert_eq!(endpoints[0].service, "DEMO-GUID");
    assert_eq!(endpoints[1].address, "[2001:db8::10]:5555");
}

#[tokio::test]
async fn wireless_connect_requires_success_text_and_ready_transport() {
    for mode in ["success", "already"] {
        let fixture = Fixture::new(mode);
        assert_eq!(
            fixture.connect().await.unwrap().serial.as_deref(),
            Some("192.0.2.10:5555")
        );
    }
    for mode in ["refused", "unauthorized", "offline", "missing"] {
        let fixture = Fixture::new(mode);
        assert!(fixture.connect().await.is_err(), "{mode}");
        assert!(!fixture.commands().contains("-s"));
    }
    let fixture = Fixture::new("connect-query-fail");
    assert!(fixture.connect().await.is_err());
    assert_eq!(
        fixture.commands().lines().collect::<Vec<_>>(),
        vec![
            "connect 192.0.2.10:5555",
            "devices -l",
            "disconnect 192.0.2.10:5555",
        ]
    );
}

#[tokio::test]
async fn wireless_pair_uses_private_stdin_then_connects_to_the_paired_identity() {
    for mode in ["success", "pair-fail"] {
        let fixture = Fixture::new(mode);
        let result = fixture
            .adb
            .wireless_connection(WirelessRequest::Pair {
                address: "192.0.2.10:37123".into(),
                code: "123456".into(),
            })
            .await;
        if mode == "success" {
            assert_eq!(result.unwrap().serial.as_deref(), Some("192.0.2.10:5555"));
        } else {
            assert!(!result.unwrap_err().contains("123456"));
        }
        assert!(fixture.commands().starts_with("pair 192.0.2.10:37123\n"));
        assert!(!fixture.commands().contains("123456"));
    }
    let fixture = Fixture::new("success");
    for code in ["12345", "1234567", "123\n45", "abcdef", "１２３４５６"] {
        assert!(
            fixture
                .adb
                .wireless_connection(WirelessRequest::Pair {
                    address: "192.0.2.10:37123".into(),
                    code: code.into()
                })
                .await
                .is_err()
        );
    }
    assert!(fixture.commands().is_empty());
}

#[tokio::test]
async fn wireless_usb_targets_only_the_selected_authorized_connection() {
    let fixture = Fixture::new("usb-success");
    assert_eq!(
        fixture
            .usb("DEMO-USB-TARGET")
            .await
            .unwrap()
            .serial
            .as_deref(),
        Some("192.0.2.10:37001")
    );
    assert!(
        fixture
            .commands()
            .contains("pair 192.0.2.10:37123\nconnect 192.0.2.10:37001\n")
    );
    assert!(!fixture.commands().contains("tcpip"));
    assert!(fixture.root.join("usb-finished").is_file());
    assert!(fixture.root.join("usb-cleaned").is_file());
    for (mode, target) in [
        ("success", "DEMO-MISSING"),
        ("usb-unauthorized", "DEMO-USB-TARGET"),
        ("usb-offline", "DEMO-USB-TARGET"),
        ("success", "192.0.2.10:5555"),
    ] {
        let fixture = Fixture::new(mode);
        assert!(fixture.usb(target).await.is_err());
        assert_eq!(fixture.commands(), "devices -l\n");
    }
}

#[tokio::test]
async fn wireless_usb_checks_routes_before_mutation_and_reports_partial_setup() {
    for mode in ["no-route", "ambiguous-route"] {
        let fixture = Fixture::new(mode);
        assert!(fixture.usb("DEMO-USB-TARGET").await.is_err());
        assert!(!fixture.commands().contains("umask"));
    }
    let fixture = Fixture::new("usb-helper-fail");
    assert!(fixture.usb("DEMO-USB-TARGET").await.is_err());
    assert!(!fixture.commands().contains("connect "));
    let fixture = Fixture::new("refused");
    let error = fixture.usb("DEMO-USB-TARGET").await.unwrap_err();
    assert!(error.contains("Could not connect"));
    assert_eq!(fixture.commands().matches("connect ").count(), 1);
    assert!(!fixture.commands().contains("pair "));
    assert!(fixture.root.join("usb-cleaned").is_file());
    for mode in ["usb-wrong-device", "usb-no-wifi"] {
        let fixture = Fixture::new(mode);
        assert!(fixture.usb("DEMO-USB-TARGET").await.is_err());
    }
    let fixture = Fixture::new("usb-cleanup-fail");
    assert!(
        fixture
            .usb("DEMO-USB-TARGET")
            .await
            .unwrap()
            .message
            .contains("cleanup")
    );
}

#[tokio::test]
async fn wireless_usb_releases_a_new_transport_when_identity_verification_fails() {
    for mode in ["usb-wrong-device", "usb-wrong-guid"] {
        let fixture = Fixture::new(mode);
        assert!(fixture.usb("DEMO-USB-TARGET").await.is_err(), "{mode}");
        let commands = fixture.commands();
        let disconnects: Vec<_> = commands
            .lines()
            .filter(|line| line.starts_with("disconnect "))
            .collect();
        assert_eq!(disconnects, vec!["disconnect 192.0.2.10:37001"]);
        assert!(!commands.contains("disconnect\n"));
    }
}

#[tokio::test]
async fn wireless_usb_releases_a_transport_when_inventory_query_fails() {
    let fixture = Fixture::new("usb-connect-query-fail");
    assert!(fixture.usb("DEMO-USB-TARGET").await.is_err());
    let commands = fixture.commands();
    let disconnects: Vec<_> = commands
        .lines()
        .filter(|line| line.starts_with("disconnect "))
        .collect();
    assert_eq!(disconnects, vec!["disconnect 192.0.2.10:37001"]);
    assert!(!commands.contains("disconnect\n"));
}

#[tokio::test]
async fn wireless_usb_reuses_existing_trust_without_pairing_again() {
    for mode in ["usb-existing-ready", "usb-existing-trust"] {
        let fixture = Fixture::new(mode);
        let result = fixture.usb("DEMO-USB-TARGET").await.unwrap();
        assert!(result.message.contains("Already paired"));
        assert!(result.serial.is_some());
        assert!(!fixture.commands().contains("pair "));
        assert!(!fixture.commands().contains("mdns "));
        if mode == "usb-existing-ready" {
            assert!(!fixture.commands().contains("umask"));
            assert!(!fixture.commands().contains("connect "));
        } else {
            assert!(fixture.root.join("usb-finished").is_file());
            assert!(fixture.root.join("usb-cleaned").is_file());
        }
    }
    let fixture = Fixture::new("usb-unrelated-ready");
    let result = fixture.usb("DEMO-USB-TARGET").await.unwrap();
    assert_eq!(result.serial.as_deref(), Some("192.0.2.10:37001"));
    assert!(fixture.commands().contains("pair "));
    let fixture = Fixture::new("usb-collision");
    assert!(fixture.usb("DEMO-USB-TARGET").await.is_err());
    assert!(!fixture.root.join("usb-cleaned").exists());
    assert!(!fixture.root.join("usb-staged").exists());
    let fixture = Fixture::new("usb-corrupt-helper");
    assert!(
        fixture
            .usb("DEMO-USB-TARGET")
            .await
            .unwrap_err()
            .contains("transfer could not be verified")
    );
    assert!(fixture.root.join("usb-cleaned").is_file());
    assert!(
        !fixture
            .commands()
            .contains("app_process /system/bin QuestWireless start")
    );
    assert!(!fixture.commands().contains("pair "));
}

#[test]
fn wifi_network_selection_rejects_missing_masked_or_injected_bssids() {
    assert_eq!(
        wifi_bssid("WifiInfo: BSSID: 02:11:22:33:44:55, RSSI: -40").unwrap(),
        "02:11:22:33:44:55"
    );
    for bad in [
        "",
        "BSSID: 02:00:00:00:00:00",
        "BSSID: 02:11:22:33:44:55;reboot",
        "BSSID: 02:11:22:33:44:55 BSSID: 02:11:22:33:44:66",
    ] {
        assert!(wifi_bssid(bad).is_err());
    }
}
