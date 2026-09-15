//! Host-only QR session tests. The mock executable never opens sockets.
use super::*;
use crate::adb::wireless_tests::Fixture;

fn example_credentials() -> qr_pairing::Credentials {
    qr_pairing::Credentials {
        id: "EXAMPLE-SESSION".into(),
        service: "studio-DEMO-QR".into(),
        secret: "EXAMPLE-QR-SECRET".into(),
    }
}

async fn terminal(manager: &TaskManager, id: &str) -> qr_pairing::Snapshot {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let snapshot = manager.wireless_qr_status(id).unwrap();
            if !matches!(
                snapshot.status.as_str(),
                "waiting" | "pairing" | "connecting"
            ) {
                return snapshot;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn qr_session_cancellation_expiry_and_failure_release_the_queue_and_clear_image() {
    let fixture = Fixture::new("qr-idle");
    let manager = TaskManager::new();
    let first = manager.start_wireless_qr(fixture.adb.clone()).unwrap();
    assert!(first.qr_data_url.is_some());
    assert!(manager.has_active_work());
    assert!(manager.start_wireless_qr(fixture.adb.clone()).is_err());
    assert!(manager.wireless_permit().is_err());
    manager.cancel_wireless_qr(&first.id).unwrap();
    let cancelled = terminal(&manager, &first.id).await;
    assert_eq!(cancelled.status, "cancelled");
    assert!(cancelled.qr_data_url.is_none() && cancelled.serial.is_none());
    assert!(!manager.has_active_work());
    let next = manager
        .start_qr_with_lifetime(fixture.adb.clone(), Duration::from_millis(80))
        .unwrap();
    assert_ne!(first.id, next.id);
    assert_ne!(first.qr_data_url, next.qr_data_url);
    assert!(manager.cancel_wireless_qr(&first.id).is_err());
    assert!(manager.wireless_qr_status(&first.id).is_err());
    let expired = terminal(&manager, &next.id).await;
    assert_eq!(expired.status, "expired");
    assert!(expired.qr_data_url.is_none());
    assert!(!manager.has_active_work());
    assert!(!fixture.commands().contains("pair "));
    assert!(!fixture.commands().contains("connect "));
    let failed = manager
        .start_wireless_qr(Adb::new("EXAMPLE-NONEXISTENT-ADB".into()))
        .unwrap();
    assert_eq!(terminal(&manager, &failed.id).await.status, "failed");
    assert!(!manager.has_active_work());
    assert!(manager.snapshots().is_empty());
}

#[tokio::test]
async fn qr_flow_pairs_once_then_uses_only_the_paired_identity() {
    for mode in ["qr-success", "qr-auto", "qr-hidden-guid-auto"] {
        let fixture = Fixture::new(mode);
        let manager = TaskManager::new();
        let result = manager
            .run_qr(&fixture.adb, example_credentials())
            .await
            .unwrap();
        assert_eq!(
            result,
            if mode != "qr-success" {
                "DEMO-GUID._adb-tls-connect._tcp"
            } else {
                "192.0.2.10:5555"
            }
        );
        let commands = fixture.commands();
        assert_eq!(commands.matches("pair ").count(), 1);
        assert_eq!(
            commands.matches("connect ").count(),
            usize::from(mode == "qr-success")
        );
        assert!(!commands.contains("EXAMPLE-QR-SECRET"));
        assert!(!commands.contains("192.0.2.11") && !commands.contains("DEMO-OTHER"));
        assert!(commands.contains("getprop persist.adb.wifi.guid"));
    }
}

#[tokio::test]
async fn qr_flow_refuses_ambiguous_scanners_and_unverified_connections() {
    for mode in [
        "qr-ambiguous",
        "qr-pair-fail",
        "qr-unknown-guid",
        "qr-wrong-guid",
        "qr-hidden-guid",
    ] {
        let fixture = Fixture::new(mode);
        let error = TaskManager::new()
            .run_qr(&fixture.adb, example_credentials())
            .await
            .unwrap_err();
        assert!(!error.contains("EXAMPLE-QR-SECRET"));
        if mode != "qr-wrong-guid" && mode != "qr-hidden-guid" {
            assert!(!fixture.commands().contains("connect "));
        }
        if mode == "qr-ambiguous" {
            assert!(!fixture.commands().contains("pair "));
        }
    }
    let fixture = Fixture::new("qr-no-connect");
    assert!(
        tokio::time::timeout(
            Duration::from_millis(500),
            TaskManager::new().run_qr(&fixture.adb, example_credentials())
        )
        .await
        .is_err()
    );
    assert!(!fixture.commands().contains("connect "));
}

#[tokio::test]
async fn qr_dropped_flow_stops_an_inflight_pair_client() {
    let fixture = Fixture::new("qr-pair-wait");
    let manager = TaskManager::new();
    let worker = manager.run_qr(&fixture.adb, example_credentials());
    let observed = async {
        while !fixture.root.join("pair-started").exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };
    tokio::time::timeout(Duration::from_secs(3), async {
        tokio::select! { _ = worker => panic!("Pairing should remain pending"), _ = observed => {} }
    })
    .await
    .unwrap();
    assert!(!fixture.commands().contains("connect "));
}
