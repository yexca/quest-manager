# Getting Started

## Prerequisites

Use Windows x64 with the system Node.js and npm versions recorded in
[toolchain.versions.json](../toolchain.versions.json). Install the Visual Studio
2022 C++ desktop workload or corresponding Build Tools, a Windows SDK, and
Microsoft Edge WebView2 Runtime. These system components are checked by the
bootstrap script but are not installed into `env`.

The first setup downloads the pinned Rust toolchain, ADB, AAPT2, and application
dependencies. Allow at least 8 GB for tools and build caches. See
[Dependencies](development/dependencies.md) for exact version sources.

## Start the Desktop App

Open PowerShell in the repository root:

```powershell
.\run-install.ps1
.\run-dev.ps1
```

If the current PowerShell policy blocks local scripts, use a process-scoped
invocation without changing the machine-wide policy:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\run-install.ps1
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\run-dev.ps1
```

Enable developer mode on the Quest, attach a USB data cable, and allow USB
debugging in the headset. Choose the device in **Devices**. For wireless use,
open **Devices** and choose **Add connection**. You can
use **USB setup** to check existing authorization and connect automatically,
or **QR code** / **Pairing code** when offered by headset settings. Deep sleep
disconnects Wi-Fi; ADB can reconnect on wake with wireless debugging enabled.
See [wireless setup](product/workflows.md#wireless-setup).

Use Applications to inspect packages or choose APKs to install. Use Files to
select a destination before uploading. Check Task queue for completion and wait
for active work to finish before closing the app.

## Preview Without a Device

After dependency installation:

```powershell
npm run dev
```

Open [the browser preview](http://127.0.0.1:1420/?preview=1). The
"Preview · sample data" badge identifies fictional data; install and transfer
actions are disabled. Without `?preview=1`, a normal browser does not connect to
ADB. A preview is useful for UI inspection, not proof of device compatibility.

The development port is shared with Tauri development. Stop a standalone Vite
preview before starting `run-dev.ps1`. The server uses a strict port and will
report a conflict instead of choosing another one.

## Build and Diagnose

```powershell
.\run-build.ps1
.\run-build.ps1 -Installer
```

The portable app is `release/quest-manager.exe`; keep its `platform-tools`
directory and licenses alongside it. NSIS output is under
`env/target/release/bundle/nsis`. Review local environment records before
distributing artifacts as described in [Commit and release](development/commit-and-release.md).

See [Troubleshooting](operations/troubleshooting.md) if setup, authorization, or
an operation fails. Treat device output and screenshots as private diagnostics.
