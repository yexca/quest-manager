# ADR-0008: Explicit Wireless Connection Setup

Status: Accepted

## Context

The app operates over ready ADB Wi-Fi transports, but disconnected users need a
setup entry. Quest settings vary and may hide Android's debugging scanner or
pairing-code screen. USB authorization can provide access to the platform ADB
service without installing an application. Existing trust should be reused.

## Decision

Offer three tabs: USB setup, QR code and Pairing code. Use strict pair/selected-USB
business IPC, with QR start/status/cancel commands. Keep device operations in
`adb.rs`, queue exclusion in `tasks.rs`, and registration in `lib.rs`. Numeric
pairing endpoints and codes are validated; arbitrary shell text is never accepted.
All setup is explicit under the global mutation permit. Busy tasks block setup;
new tasks wait on the same permit. Closing with setup active uses the exit guard.

USB captures physical identity and reuses a ready matching Wi-Fi transport first.
Otherwise a first-party DEX helper enables system TLS debugging for the current
Wi-Fi network and returns the dynamic connection port. The parent tries existing
trust first, requesting new pairing only after an authentication failure. A
network failure is not proof of pairing. Wi-Fi must return the captured USB
identity; a readable pairing GUID must also agree after new pairing.

The helper is a dedicated exception to general `/sdcard` file management. Stage
only embedded first-party bytes in a random owned directory under
`/data/local/tmp/quest-manager-wireless-*`; refuse collisions, transfer via
binary-preserving `exec-in`, verify SHA-256 and mark the DEX read-only. Run as the
already authorized shell user and resolve named platform methods, never guessed
Binder transaction numbers. No APK is installed. The existing pinned Apktool/JRE
assemble Smali during Cargo builds; no new toolchain dependency is introduced.

USB has bounded identity/network preflight, setup, child shutdown and awaited
cleanup stages. The shell wrapper also limits the helper lifetime. Stop only a
pairing listener requested by the operation, then remove owned temporary files;
report unconfirmed cleanup. Abrupt exits can leave staging files. Trust and enabled
debugging are not rolled back. See [workflows](../architecture/workflows.md) for
the concrete deadlines. The global permit stays held through cleanup.

Code pairing uses private stdin, followed by discovery and verified connection,
with a 90-second overall deadline. QR uses one in-memory session with a two-minute
monotonic deadline. `qr_pairing.rs` generates fresh service names/secrets from OS
randomness and locally encodes `WIFI:T:ADB;S:studio-<random>;P:<secret>;;` as SVG.
Only explicit setup starts ADB mDNS polling. Match the exact pairing service,
pair once, and discover the returned GUID's connection. Verify the device GUID;
when shell hides that property, require the exact authenticated TLS service
transport for the GUID. Do not accept a numeric address alone in that case.

Clear the QR when pairing starts, cancellation is requested or the session ends.
Secrets stay in worker/private-stdin memory, never process arguments, logs or
persisted app data. Status reads issue no device commands. Cancellation/deadline
drop the worker/client, but submitted pairing can finish in the shared server.
Stale session IDs cannot read or cancel a replacement session.

Explicit connection success requires ADB's success response and a matching ready
transport. Refresh discovery after older polls finish and select that connection
for future operations. Existing task targets never change. Ordinary discovery
still prefers USB. Do not scan networks, reset keys, kill the shared server,
root, reboot or automatically pair/enable debugging as recovery.

## Consequences

Users can establish or reuse Wi-Fi through an authorized USB cable even when
headset settings hide pairing UI. QR/code and platform-method availability remain
headset capabilities, not Android-version promises. The USB fallback currently
requires one IPv4 route on `wlan0`; code/QR endpoints also accept bracketed IPv6.
ADB may reconnect trusted TLS transports after wake; no app-managed task resume,
retry loop or trust persistence is introduced. Pairing cannot start in deep sleep;
the UI asks users to wake the headset and keep its display on during setup.
ADB/headset trust persists independently and is managed in headset settings.

Host-only mock tests cover sequencing, identity, reuse, failures, queue exclusion
and cleanup. Hardware radio behavior and platform support require opt-in tests.

Protocol references: [AOSP ADB Wi-Fi architecture](https://android.googlesource.com/platform/packages/modules/adb/+/HEAD/docs/dev/adb_wifi.md),
[ADB pairing client](https://android.googlesource.com/platform/packages/modules/adb/+/HEAD/client/adb_wifi.cpp),
and [Android 14 IAdbManager](https://android.googlesource.com/platform/frameworks/base/+/refs/tags/android-14.0.0_r1/core/java/android/debug/IAdbManager.aidl).

## Connection Management Extension

Manage connections exposes actual transport entries while the main device list
summarizes each connection method. Reuse performs a fresh discovery check before
setup. Explicit disconnect is a dedicated typed IPC under the same mutation
permit and busy-task exclusion, with fresh identity/transport checks and an
exact serial argument. Pairing is retained and automatic ADB reconnection is
reported honestly; there is no persistent blocklist or automatic duplicate
disconnection.

Connect/Reconnect is a separate explicit operation from pairing. A previously
observed service may resolve a new connection port; unidentified discovered
services require user selection. Every new connection verifies physical
identity. Network absence or timeout never proves lost pairing, and no app
profile field is treated as authoritative pairing state. Reconnect retains
connection preferences and performs no automatic pairing, debugging setup,
subnet scan or shared-server reset.
