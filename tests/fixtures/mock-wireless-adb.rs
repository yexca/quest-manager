// Host-only wireless ADB fixture. No sockets, real ADB, or headset access.
use std::{env, fs, io::{BufRead, Read, Write}};
fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    let root = env::current_exe().unwrap().parent().unwrap().to_path_buf();
    let mode = fs::read_to_string(root.join("mode")).unwrap();
    writeln!(fs::OpenOptions::new().create(true).append(true).open(root.join("commands")).unwrap(), "{}", args.join(" ")).unwrap();
    let connected = root.join("connected");
    match args[0].as_str() {
        "disconnect" => {
            assert_eq!(args.len(), 2, "Never disconnect all network transports");
            assert_eq!(fs::read_to_string(&connected).unwrap(), args[1]);
            if mode != "disconnect-reconnect" { fs::remove_file(&connected).unwrap(); }
            println!("disconnected {}", args[1]);
        }
        "mdns" => {
            assert_eq!(args, ["mdns", "services"]);
            println!("List of discovered mdns services");
            println!("studio-DEMO-OLD _adb-tls-pairing._tcp 192.0.2.11:37123");
            if mode != "reconnect-missing" && mode != "reconnect-unique" { println!("DEMO-OTHER _adb-tls-connect._tcp 192.0.2.11:5555"); }
            if mode != "qr-idle" {
                println!("studio-DEMO-QR _adb-tls-pairing._tcp 192.0.2.10:37123");
            }
            if let Ok(service) = fs::read_to_string(root.join("usb-service")) {
                println!("{service} _adb-tls-pairing._tcp 192.0.2.10:37123");
            }
            if mode == "qr-ambiguous" {
                println!("studio-DEMO-QR _adb-tls-pairing._tcp 192.0.2.12:37123");
            }
            if mode != "qr-no-connect" && mode != "reconnect-missing" {
                println!("DEMO-GUID _adb-tls-connect._tcp 192.0.2.10:5555");
            }
            if mode == "reconnect-ambiguous" { println!("DEMO-GUID _adb-tls-connect._tcp 192.0.2.12:5555"); }
        }
        "connect" => {
            assert_eq!(args.len(), 2);
            if mode == "refused" { println!("cannot connect to {}: Example refusal", args[1]); return; }
            if mode == "reconnect-auth" { println!("failed to authenticate to {}", args[1]); return; }
            if args[1] == "192.0.2.10:37001" && mode != "usb-existing-trust" && mode != "usb-wrong-device"
                && !root.join("paired").exists() {
                println!("failed to authenticate to {}", args[1]); return;
            }
            fs::write(connected, &args[1]).unwrap();
            println!("{} to {}", if mode == "already" { "already connected" } else { "connected" }, args[1]);
        }
        "pair" => {
            assert_eq!(args.len(), 2, "Pairing code must not be a process argument");
            let mut code = String::new();
            std::io::stdin().read_to_string(&mut code).unwrap();
            let expected = fs::read_to_string(root.join("usb-secret")).map(|secret| format!("{secret}\n"))
                .unwrap_or_else(|_| if mode.starts_with("qr-") { "EXAMPLE-QR-SECRET\n".into() } else { "123456\n".into() });
            assert_eq!(code, expected);
            if mode == "pair-fail" || mode == "qr-pair-fail" { println!("Example invalid code: {}", code.trim()); return; }
            if mode == "qr-pair-wait" { fs::write(root.join("pair-started"), "").unwrap(); std::thread::sleep(std::time::Duration::from_secs(30)); }
            fs::write(root.join("paired"), "").unwrap();
            if matches!(mode.as_str(), "qr-auto" | "qr-hidden-guid-auto") { fs::write(connected, "DEMO-GUID._adb-tls-connect._tcp").unwrap(); }
            println!("Enter pairing code: Successfully paired to {} [guid={}]", args[1], if mode == "qr-unknown-guid" { "bad;identity" } else { "DEMO-GUID" });
        }
        "devices" => {
            assert_eq!(args, ["devices", "-l"]);
            if (mode == "connect-query-fail" || mode == "usb-connect-query-fail") && connected.exists() {
                eprintln!("device inventory failed");
                std::process::exit(1);
            }
            println!("List of devices attached");
            if !mode.starts_with("reconnect-") {
                println!("DEMO-USB-OTHER device usb:1 model:Quest_2");
                println!("DEMO-USB-TARGET {} usb:2 model:Quest_3", if mode == "usb-unauthorized" { "unauthorized" } else if mode == "usb-offline" { "offline" } else { "device" });
            }
            if matches!(mode.as_str(), "usb-existing-ready" | "usb-unrelated-ready") {
                println!("DEMO-GUID._adb-tls-connect._tcp device model:Quest_3");
            }
            if let Ok(address) = fs::read_to_string(connected) {
                if mode != "missing" {
                    println!("{address} {} model:Quest_3", if mode == "unauthorized" { "unauthorized" } else if mode == "offline" { "offline" } else { "device" });
                }
            }
        }
        "-s" => {
            if args[2] == "exec-in" {
                assert_eq!(args[1], "DEMO-USB-TARGET");
                assert!(args[3].starts_with("umask 077; cat"));
                let mut dex = Vec::new();
                std::io::stdin().read_to_end(&mut dex).unwrap();
                assert!(dex.starts_with(b"dex\n"));
                fs::write(root.join("usb-staged"), "").unwrap();
                return;
            }
            assert_eq!(args[2], "shell");
            if args[3] == "getprop ro.serialno" {
                if mode == "reconnect-query-fail" {
                    eprintln!("identity query failed");
                    std::process::exit(1);
                }
                if mode == "reconnect-wrong-device" { println!("DEMO-OTHER"); return; }
                if args[1] == "192.0.2.10:5555" { println!("DEMO-HEADSET"); return; }
                if args[1] == "192.0.2.11:5555" { println!("DEMO-OTHER"); return; }
                if args[1] == "DEMO-USB-OTHER" { println!("DEMO-OTHER"); return; }
                assert!(args[1] == "DEMO-USB-TARGET" || args[1] == "192.0.2.10:37001" || args[1] == "DEMO-GUID._adb-tls-connect._tcp");
                println!("{}", if (mode == "usb-wrong-device" && args[1].contains(':')) || (mode == "usb-unrelated-ready" && args[1].contains("_adb-tls-")) { "DEMO-OTHER" } else { "DEMO-HEADSET" });
                return;
            }
            if args[3] == "getprop persist.adb.wifi.guid" {
                if mode == "qr-query-fail" { eprintln!("identity query failed"); std::process::exit(1); }
                println!("{}", if mode == "qr-wrong-guid" || mode == "usb-wrong-guid" { "DEMO-OTHER" } else if mode.starts_with("qr-hidden-guid") || mode.starts_with("usb-") { "" } else { "DEMO-GUID" });
                return;
            }
            assert_eq!(args[1], "DEMO-USB-TARGET", "Never substitute another USB headset");
            match args[2].as_str() {
                "shell" => {
                    if args[3] == "ip -4 route show dev wlan0" {
                        if mode != "no-route" { println!("192.0.2.0/24 proto kernel scope link src 192.0.2.10"); }
                        if mode == "ambiguous-route" { println!("198.51.100.0/24 proto kernel scope link src 198.51.100.10"); }
                    } else if args[3] == "cmd wifi status" {
                        if mode != "usb-no-wifi" { println!("WifiInfo: SSID: EXAMPLE-NETWORK, BSSID: 02:11:22:33:44:55, RSSI: -40"); }
                    } else if args[3].starts_with("umask 077; mkdir") {
                        if mode == "usb-collision" { std::process::exit(1); }
                    } else if args[3].starts_with("touch ") {
                        assert!(args[3].ends_with("/pairing'"));
                    } else if args[3].starts_with("chmod 400 ") {
                        assert!(args[3].contains("sha256sum"));
                        println!("{}", if mode == "usb-corrupt-helper" { "invalid".into() } else { fs::read_to_string(root.join("helper-hash")).unwrap() });
                    } else if args[3].starts_with("trap ") {
                        if mode == "usb-helper-fail" { return; }
                        let mut input = std::io::stdin().lock();
                        let mut network = String::new(); let mut service = String::new(); let mut secret = String::new();
                        input.read_line(&mut network).unwrap(); input.read_line(&mut service).unwrap(); input.read_line(&mut secret).unwrap();
                        assert_eq!(network.trim(), "02:11:22:33:44:55");
                        assert!(service.trim().starts_with("studio-") && secret.trim().len() == 32);
                        println!("37001"); std::io::stdout().flush().unwrap();
                        let mut request = String::new(); input.read_line(&mut request).unwrap();
                        if request.trim() == "pair" {
                            fs::write(root.join("usb-service"), service.trim()).unwrap();
                            fs::write(root.join("usb-secret"), secret.trim()).unwrap();
                            println!("pairing"); std::io::stdout().flush().unwrap();
                            let mut done = String::new(); input.read_line(&mut done).unwrap();
                        }
                        fs::write(root.join("usb-finished"), "").unwrap();
                    } else if args[3].starts_with("if [ -f ") {
                        fs::write(root.join("usb-cleaned"), "").unwrap();
                        if mode == "usb-cleanup-fail" { std::process::exit(1); }
                    } else { panic!("Unsupported fixture shell command"); }
                }
                _ => panic!("Unsupported fixture command"),
            }
        }
        _ => panic!("Unsupported fixture command"),
    }
}
