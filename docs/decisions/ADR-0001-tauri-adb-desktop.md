# ADR-0001: Tauri Desktop with an Official ADB Backend

Status: Accepted; documents the current implementation.

## Context

The product needs a desktop interface, native computer file selection, APK
installation, and file management on an authorized Quest. These operations
require local device access and long-running subprocesses. A normal browser
preview cannot supply the same capabilities.

## Decision

Use Tauri 2 with React and TypeScript for the English UI, Rust for typed IPC,
validation, and task execution, and the official ADB executable for the Android
protocol. Target Windows x64 first and use WebView2 as the webview runtime.

Expose explicit application commands through Tauri. Keep arbitrary host or
Android command execution out of the frontend API. Reuse the default ADB server
and bundle Platform-Tools with release output.

## Consequences

The UI can use native file dialogs and show background task state while Rust
owns subprocess lifetime and validation. The project does not implement ADB's
protocol itself or expose an HTTP device-management service.

Windows development requires MSVC, Windows SDK, and WebView2. A portable
distribution must include ADB's resources and licenses. Other platforms would
need explicit bootstrap, resource, path, and packaging work.

Browser preview remains a separate mode with fictional reads and disabled
writes. Native IPC, installation, and transfers need the desktop runtime.

See [Architecture](../architecture/index.md) and
[Runtime configuration](../operations/configuration.md).
