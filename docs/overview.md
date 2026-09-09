# Overview

Quest Manager is a local Windows desktop companion for Meta Quest, with Quest 2
as the initial product target. It uses Tauri 2, React, TypeScript, Rust, and the
official Android Platform-Tools ADB executable. The UI is English.

## Goals

- Inspect a connected headset and its shared-storage usage.
- Manage installed applications and install APKs selected on the computer.
- Move and organize shared files with clear destinations and collision errors.
- Keep long operations visible while allowing continued browsing.
- Make toolchain selection and dependency resolution reproducible.

## Implemented Behavior

The app discovers ADB connections, groups connections by device identity when
available, and prefers a ready USB connection. It can also use Wi-Fi connections
already known to ADB.

Applications are identified by package name. Users can inspect versions, install
ordinary APKs, apply Android-compatible updates, export installed APK files
including splits, and uninstall third-party applications.

The file browser exposes shared storage, with file and folder upload/download,
directory creation, rename, and deletion. Mutations run through a single global
queue. Transfers use temporary destinations and refuse existing final names.

The browser preview uses explicitly fictional data and disables device writes.
The desktop app performs real ADB operations. See
[Product workflows](product/workflows.md) for details.

## Current Limits

- The bootstrap and distributable target Windows x64 with MSVC and WebView2.
  Other desktop platforms are not packaged by the current scripts.
- There is no wireless-debugging setup, root access, arbitrary command console,
  or general browser for private Android app data.
- Installation accepts ordinary APKs individually, including a queue of several
  independent APKs. XAPK/APKS/APKM archives and split-set installation are not
  supported. APK export is not a saved-game backup.
- Tasks and selections are in memory. There is no durable queue, resume,
  automatic retry, recycle bin, or rollback after a completed destructive task.
- App names and icons are not extracted from APKs; package IDs are shown.
- The repository has local build/test scripts, with no configured CI or
  automated publication pipeline.

Exact versions live in the [dependency sources](development/dependencies.md).
See [Getting started](getting-started.md), [Architecture](architecture/index.md),
and [Privacy](../PRIVACY.md) before using live device output in documentation.
