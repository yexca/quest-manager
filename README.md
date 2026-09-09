<p align="center">
  <img src="src-tauri/icons/icon.png" width="128" height="128" alt="Quest Manager logo">
</p>

<h1 align="center">Quest Manager</h1>

<p align="center">
  Manage apps and files on your Meta Quest through a local ADB connection.
</p>

<p align="center">
  <strong>English</strong> · <a href="README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <a href="docs/README.md">Documentation</a> ·
  <a href="docs/getting-started.md">Getting Started</a> ·
  <a href="AGENTS.md">Agent Guide</a> ·
  <a href="SECURITY.md">Security</a> ·
  <a href="PRIVACY.md">Privacy</a>
</p>

Quest Manager is a Windows desktop companion for Meta Quest, with Quest 2 as
its initial product target. It combines an English interface with native file
pickers, background tasks, and the official Android Platform-Tools ADB client.
The stack is **Tauri 2 + React + TypeScript + Rust**.

## Key Features

- **Device overview.** Inspect the model, Android version, battery, and shared
  storage. Group USB and existing Wi-Fi connections when device identity is
  available, with USB preferred by default.
- **Install from your computer.** Select or drop ordinary APK files, review the
  target headset, and queue installations or compatible updates.
- **Application management.** Search package IDs, inspect versions, export all
  installed APK files including splits, and uninstall third-party apps.
- **Shared file management.** Browse, upload, and download files or folders;
  create directories, rename items, and delete selected content.
- **Visible background work.** Follow a serial task queue with ADB progress and
  errors. Cancel queued work or supported running transfers while browsing.
- **Reproducible setup.** Use pinned tools and locked dependencies, with Rust,
  ADB, npm packages, and build caches kept inside the project environment.

## Quick Start

### 1. Prepare the computer

Use Windows x64 with **Node.js 24.19.0 / npm 11.17.0** already installed. The
bootstrap reuses system Node and checks these prerequisites:

- Visual Studio 2022 or Build Tools with **Desktop development with C++**,
  including MSVC and a Windows SDK.
- Microsoft Edge WebView2 Runtime.

Allow at least 8 GB for tools and build caches. The first setup downloads the
pinned toolchain and dependencies; the first Rust build takes longer than
subsequent builds. See [Getting Started](docs/getting-started.md) for details.

### 2. Install dependencies and start the app

Open PowerShell in the repository root:

```powershell
.\run-install.ps1
.\run-dev.ps1
```

The bootstrap does not require administrator privileges or change the permanent
PATH. System prerequisites are installed separately. If PowerShell blocks local
scripts, use a process-scoped invocation:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\run-install.ps1
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\run-dev.ps1
```

### 3. Connect the headset

Enable developer mode on the Quest, connect a USB data cable, and allow USB
debugging inside the headset. Choose the headset in **Overview**.

Use **Applications** to install or inspect APKs and **Files** to browse shared
storage. Follow operations in **Task queue** and wait for active work to finish
before closing the app. Existing ADB Wi-Fi connections appear automatically;
this version does not configure wireless debugging.

See [Troubleshooting](docs/operations/troubleshooting.md) if setup or connection
fails.

## Applications and Files

| Action | Result | Scope |
| --- | --- | --- |
| Install APK | Install an ordinary APK or apply a compatible update | Each selected APK is an independent task |
| Export application | Save all installed APK files into a new local folder | Includes splits; excludes private app data and saved games |
| Uninstall application | Remove an app and its app data after confirmation | Third-party apps only |
| Upload or download | Copy files or folders through a temporary destination | Existing final items are refused |
| Rename or delete | Organize shared files; confirm permanent deletion | Android permissions and protected containers still apply |

File management is limited to `/sdcard`, with `/storage/emulated/0` accepted as
an alias. Access to `Android/data` and `Android/obb` depends on the headset OS.
General private-app storage access and complete saved-game backups are outside
the current scope.

Installation does not support XAPK/APKS/APKM archives or split-set installation.
Compatible APK updates use Android's replace-install behavior; signature,
downgrade, ABI, or storage failures are reported rather than worked around by
automatically uninstalling an existing app.

Transfers reject Windows-incompatible download names, case-only collisions,
and descendant symbolic links. Interrupted transfers can leave a reported
`.quest-manager-*.partial` item when cleanup cannot complete. Task history lasts
for the current session, with no automatic retry, restart resume, or recycle bin.
See [Product Workflows](docs/product/workflows.md) for the complete behavior.

## Local Files

| Project path | Purpose |
| --- | --- |
| `env/` | Rust and ADB tools, npm packages, caches, build output, and local installation records |
| `node_modules/` | Windows junction pointing to `env/node_modules` |
| `dist/` | Built frontend assets |
| `release/` | Portable executable, Platform-Tools, and local build environment record |
| `src-tauri/gen/` | Generated Tauri schemas |

These generated locations are ignored by Git. Source code, version manifests,
and both dependency lockfiles are the inputs needed to recreate the project
environment. See [Dependencies](docs/development/dependencies.md) for the full
layout and [Privacy](PRIVACY.md) before sharing local records.

## Reproducible Setup

| Component | Pinned version or source |
| --- | --- |
| Node.js / npm | 24.19.0 / 11.17.0; `toolchain.versions.json`, `.node-version`, and `package.json` |
| Rust / target | 1.95.0 / `x86_64-pc-windows-msvc`; `rust-toolchain.toml` |
| rustup | 1.29.0; fixed archive URL and SHA-256 |
| Android Platform-Tools | 37.0.1; fixed archive URL and SHA-256 |
| Tauri Rust / CLI / JS API | 2.11.5 / 2.11.4 / 2.11.1 |
| React / TypeScript / Vite | 19.3.0 / 7.0.2 / 8.2.2 |
| JavaScript dependency tree | Exact direct versions and `package-lock.json` |
| Rust dependency tree | Exact direct versions and `src-tauri/Cargo.lock` |
| Optional installer tools | Downloaded and verified by the pinned Tauri CLI; cached under `env/target/.tauri` |

Normal installation uses `npm ci` and `cargo fetch --locked`. Lockfiles are
regenerated only during an intentional dependency update with `-RefreshLocks`.
Tauri components are released independently and do not need matching patch
versions.

Visual Studio, Windows SDK, and WebView2 remain system prerequisites. Local
installation and build records capture their actual environment details, but
are not committed. Reproducibility here means toolchain and dependency selection,
not byte-identical output across different system or signing environments.
Read [Dependencies and Reproducibility](docs/development/dependencies.md) before
changing versions.

## Build

```powershell
.\run-build.ps1
.\run-build.ps1 -Installer
```

The portable executable is `release/quest-manager.exe`. Keep its
`platform-tools` directory, DLLs, and license files with it. The optional NSIS
installer is written under `env/target/release/bundle/nsis`.

Review local environment records before distributing build output. See
[Commit and Release](docs/development/commit-and-release.md) for version
locations, packaging details, and artifact handling.

## Browser Preview

After installing dependencies, run `npm run dev` and open
[the preview](http://127.0.0.1:1420/?preview=1). It shows **Preview · sample data**
with fictional device and file entries; device writes are disabled. Real
operations require the desktop app.

The preview and Tauri development use the same strict port. Stop a standalone
Vite instance before starting `run-dev.ps1`.

## Documentation

| Goal | Start here |
| --- | --- |
| Install and connect a headset | [Getting Started](docs/getting-started.md) |
| Understand user-visible behavior | [Product Specs](docs/product/index.md) |
| Configure and diagnose the runtime | [Configuration](docs/operations/configuration.md) · [Troubleshooting](docs/operations/troubleshooting.md) |
| Understand data and system boundaries | [Architecture](docs/architecture/index.md) |
| Review design and security contracts | [Design](DESIGN.md) · [Security](SECURITY.md) · [Privacy](PRIVACY.md) |
| Find every public document | [Documentation Index](docs/README.md) |

## Development and Contributing

Development setup, validation, and maintenance procedures live under
`docs/development/`. Start with the relevant guide:

- [Local Development](docs/development/local-dev.md)
- [Testing](docs/development/testing.md)
- [Dependencies](docs/development/dependencies.md)
- [Contributing](CONTRIBUTING.md)
- [Agent Guide](AGENTS.md)

Use `run-test.ps1` for the routine frontend and Rust checks. Device integration
tests are opt-in; read the testing guide before using a connected headset.
The product UI remains English; the linked Chinese README is documentation.

## Security and Privacy

Quest Manager runs with the local user's privileges and delegates device access
to ADB and Android's debugging permissions. It uses explicit application
commands for supported operations and has no cloud sync or analytics integration.

Device identifiers, application inventories, paths, and ADB output can appear
in the interface and diagnostics. Review [Privacy](PRIVACY.md) before sharing
screenshots or logs, and follow [Security](SECURITY.md) for sensitive reports.
