# Getting Started

## Prerequisites

Use Windows x64 with 64-bit PowerShell. Node.js and npm are downloaded into
`env/node` from the official ZIP verified by the SHA-256 in
[toolchain.versions.json](../toolchain.versions.json); no system Node is needed.

The bootstrap checks these system components before downloading project tools:

| Component | Accepted baseline |
| --- | --- |
| Visual Studio 2022 / Build Tools | 17.x, complete x64 MSVC >= 14.30 compiler, linker, headers and libraries |
| Windows SDK | >= 10.0.19041.0, with x64 libraries, headers and resource compiler |
| WebView2 Runtime | >= 110.0.1531.0, registered for this user or the machine |

These are project compatibility checks, not a guarantee for every system
configuration. They do not pin Windows or promise byte-identical builds.
Supported version boundaries live in `toolchain.versions.json`.

If a component is missing, incomplete or incompatible, setup reports what is
required and asks before changing the system (default **No**). Compatible
components are reused. For C++/SDK repair, the script updates one selected
VS 2022 instance to the current VS 2022 servicing release, then adds x64 C++
tools and Windows SDK 26100. The prompt identifies that instance; this can
affect other projects using it. Without VS 2022, it installs Build Tools 2022
alongside other major versions. Close Visual Studio first and allow several GB
of additional disk space. Administrator approval may be requested twice.

WebView2 uses Microsoft's Evergreen bootstrapper to install/update the shared
runtime and its updater. It normally installs for the current user when setup
is not elevated. System installers are verified for a valid Microsoft digital
signature before execution. Setup waits, checks exit codes, and checks the
components again. It never requests an automatic restart; if a restart is
required, restart Windows yourself and rerun setup. Declining consent or UAC,
an installer error, or failed rechecking stops setup with instructions.

For manual installation, use Microsoft's
[C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
(choose VS 2022 and the C++ desktop workload including a Windows SDK) and
[WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).
The [VS 2022 bootstrapper links](https://learn.microsoft.com/en-us/visualstudio/install/use-command-line-parameters-to-install-visual-studio?view=vs-2022)
identify the intended major version. No winget or Chocolatey installation is required.

The first setup downloads the pinned Rust toolchain, ADB, AAPT2, and application
dependencies. Allow at least 8 GB for tools and build caches. See
[Dependencies](development/dependencies.md) for exact version sources.

## Start the Desktop App

Open PowerShell in the repository root:

```powershell
.\run-install.ps1
.\run-dev.ps1
```

Optional setup modes:

```powershell
.\run-install.ps1 -CheckOnly       # System checks only; no downloads, prompts or installation
.\run-install.ps1 -NonInteractive  # Install project tools; fail if system prerequisites need attention
```

Both modes return a failure when system prerequisites are incompatible. Neither
authorizes system installation. `-CheckOnly` does not check project tool payloads
and cannot be combined with `-RefreshLocks`. Redirected input also prevents
installation consent; run interactively to answer the prompt.

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
. .\scripts\Environment.ps1
npm.cmd run dev
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

The portable app is `release/quest-manager.exe`; keep its `platform-tools`,
`aapt2`, `apk-tools` directories and licenses alongside it. NSIS output is under
`env/target/release/bundle/nsis`. Review local environment records before
distributing artifacts as described in [Commit and release](development/commit-and-release.md).

See [Troubleshooting](operations/troubleshooting.md) if setup, authorization, or
an operation fails. Treat device output and screenshots as private diagnostics.
