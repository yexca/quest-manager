# Troubleshooting

Diagnose from the failed layer: setup, desktop launch, ADB connection, or an
individual task. Inspect raw output locally; it can contain device identifiers,
package names, and personal paths. Use synthetic reproductions in public docs.

## Setup and Launch

| Symptom | Next step |
| --- | --- |
| Node/npm version mismatch | Use the system versions declared in the project; the bootstrap does not replace Node |
| Missing C++ tools, SDK, or WebView2 | Install the prerequisite named by the bootstrap, then retry `run-install.ps1` |
| PowerShell blocks a local script | Use the process-scoped invocation in [Getting started](../getting-started.md) |
| Download checksum mismatch | Recheck the pinned URL/hash and retry the download; do not disable checksum verification |
| Missing or damaged local Rust components | Rerun the bootstrap; use the project environment for any targeted diagnosis |
| Unexpected root `node_modules` | Inspect the directory/junction and preserve its contents; restore the expected link to `env/node_modules` before retrying |
| Manifests changed since installation | Rerun `run-install.ps1`; only use `-RefreshLocks` for an intended lockfile update |
| Port 1420 is occupied | Stop the known previous Vite/Tauri dev instance or identify the conflicting process before taking action |
| Browser reports that the desktop app is required | Use `run-dev.ps1` for real devices, or explicit `?preview=1` for fictional UI preview |
| Portable app cannot start ADB | Restore the matching `platform-tools` resource folder, DLLs, and files alongside the executable |
| Names or artwork remain unavailable | Keep the matching `aapt2` resource directory alongside the app; reconnect and refresh. Some adaptive/vector or split-only resources use a generic fallback |
| Cached artwork needs to be read again | Choose Clear cached artwork, then Load app details; state and permission data are always queried again |

Do not delete an unknown directory or kill unrelated processes to make a setup
error disappear. See [Local development](../development/local-dev.md).

## Connection

Check the USB data cable, developer mode, headset unlock state, and debugging
authorization. To inspect discovery with the same project ADB:

```powershell
.\env\platform-tools\adb.exe devices -l
```

This command reads connection state and can reveal real serials and addresses.
Do not paste its unredacted output into tracked files.

- `unauthorized`: allow USB debugging inside the headset, then refresh.
- `offline` or no connection: inspect the cable/connection and refresh after
  reconnecting. An established Wi-Fi connection can disappear independently.
- Duplicate-looking entries: physical identity may be unavailable before
  authorization, so USB and Wi-Fi deduplication is best effort.
- Other ADB tools in use: the app shares the default server. Coordinate any
  server-level diagnosis; do not automatically run `kill-server`, reset keys,
  enable root, or reconfigure wireless debugging.

### Wireless Setup

- No **Pair device with QR code** scanner: in Lightning Launcher, try
  **Android Settings → System → Developer options → Debugging → Wireless
  debugging**. The app's **Lightning Launcher setup** can install the launcher.
  If the scanner is still absent, use **USB setup**. Meta mobile QR pairing is
  a different protocol; Android version alone does not establish availability.
- QR remains waiting or expires: keep both devices on the same reachable
  network. Guest/client isolation or blocked mDNS can prevent discovery.
  Generate a fresh QR and scan with the headset's debugging scanner, or use
  **USB setup**. QR setup never changes firewall settings.
- Paired but did not connect: wake the headset and refresh, or use **USB setup**
  to reuse authorization. Cancellation/expiry can leave a completed pairing trusted.

Open **Devices** and choose **Add connection**.
Pairing cannot start while the headset is in deep sleep. Put it on or press its
power button to wake it, and keep the display on until setup completes. Keep it
on the same reachable local network as the computer.

- No Wireless debugging settings or pairing code: use **USB setup** with
  an authorized cable connection. Headset settings differ across OS versions.
- Pairing fails: use the address/port from the open pairing-code screen and a
  fresh six-digit code. Connection follows automatically; do not substitute the
  separate connection port for the pairing port.
- Connection refused/offline: check the current IP/port and wireless-debugging
  state. Guest-network isolation or firewall rules can prevent peer connections.
  Retry setup after correcting the cause.
- USB setup cannot determine an address: connect the headset to Wi-Fi. Setup
  requires one IPv4 source address on its `wlan0` route. If unavailable, use
  QR or code pairing in supported headset settings instead.
- USB reports already paired/authorized: the app verified an existing connection
  to that physical headset and reused it. No new pairing is needed.
- USB did not start wireless debugging: wake the headset, keep it connected to
  Wi-Fi, and retry. This failure does not establish whether it is paired.
  Existing trust, system-interface availability and network reachability are
  separate. Setup errors may leave debugging enabled.
- Temporary-helper cleanup is unconfirmed: a started helper has an on-device
  deadline, but interrupted staging can leave files. Do not delete unrelated
  files or reset ADB keys to resolve this message.
- Setup disabled: finish or cancel active tasks first. This prevents setup from
  interrupting installations or transfers. A lost USB selection is not replaced
  by another headset; refresh discovery and explicitly choose the intended one.

TLS connection ports can change. Deep sleep disconnects Wi-Fi; on the same
network, ADB can reconnect after wake while wireless debugging remains enabled.
Refresh to read current state. Failed tasks are not resumed. Restarting the
headset may require setup again. Closing Quest Manager does not
disable wireless debugging or revoke trust. Manage those in headset settings.

## APK Installation

Signature mismatch and version downgrade are Android installation failures.
Use a compatible APK; do not silently uninstall an existing application to
work around them, because uninstall can remove app data.

Check available space and device ABI for the corresponding errors. A missing
split error means the package needs an installation flow this version does not
provide. Several selected APKs are separate installs, not a split session.

`INSTALL_PARSE_FAILED_NO_CERTIFICATES` with `integer overflow` can occur when
verity verification uses an overflowing offset in a large APK. In the review,
enable **Modify and re-sign APK**, then **Compatibility install**. This replaces
the signature and normally cannot update an existing app signed by another key.
It is not a general fix for malformed packages, missing splits or ABI problems.

For a preparation error, inspect its stage in Task queue. Resource-free packages
are supported; multiple launcher entries, shared-user packages, resource path
collisions and resources over 128 MiB are rejected for editing. Compatibility-only
installation avoids resource rebuilding. Unknown/adaptive artwork can be displayed
as unavailable without preventing installation of the original.

Free local space must cover four APK sizes plus 512 MiB. Restore `apk-tools` if
Java/signing tools cannot start. Back up/restore signing keys as described in
[Privacy](../../PRIVACY.md); never generate replacement keys just to suppress an
update conflict. Cleanup failures identify task-owned staging files; preserve
keys and original APKs when cleaning up.

## Files and Transfers

Permission errors in `Android/data` or `Android/obb` can reflect headset OS
restrictions. The app cannot grant access or bypass those restrictions.

A destination collision requires a different destination/name or a deliberate
user-managed removal. Windows-invalid names, case-only collisions, and
descendant symlinks require choosing supported items or deliberately renaming
them on the device. Do not drop these checks to make a pull succeed.

On failure or cancellation, inspect Task queue. If cleanup failed, the message
identifies a possible `.quest-manager-*.partial` location. Reconnect if needed,
identify the exact task-owned temporary item, and ensure no task still uses it
before cleanup. Do not delete all files matching a broad temporary-name pattern.

After a disconnect or timeout, inspect the operation's actual state before
retrying. An install or rename may have completed even if its client lost the
result. Completed work is not rolled back, and the app has no automatic retry
or resume after restart.

See [Runtime workflows](../architecture/workflows.md),
[Product workflows](../product/workflows.md), and [Privacy](../../PRIVACY.md).
