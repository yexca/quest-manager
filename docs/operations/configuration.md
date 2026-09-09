# Runtime Configuration

The current app has no settings file, account, database connection, custom ADB
path picker, or persistent device preference. Device selection and UI state
belong to the current process.

## ADB and Device Selection

Debug builds locate `env/platform-tools/adb.exe` relative to the Rust manifest
directory. Release builds locate `platform-tools/adb.exe` through Tauri's
resource directory. This choice is made in
[lib.rs](../../src-tauri/src/lib.rs), not by searching the current shell's PATH.

ADB uses its default server and standard authorization storage. Existing USB
and Wi-Fi connections are discovered from that server. The app does not manage
the server's lifecycle, start wireless pairing, or reset authorization keys.
It can share the server with Android Studio and other ADB clients.

Overview lets the user select a physical device and a ready transport. USB is
preferred when available. New tasks capture that transport; changing a selector
does not migrate already queued work. See [Data model](../architecture/data-model.md).

## Development Environment

[Environment.ps1](../../scripts/Environment.ps1) sets these values for its
PowerShell process and children:

| Setting | Project value |
| --- | --- |
| `CARGO_HOME` | `env/cargo` |
| `RUSTUP_HOME` | `env/rustup` |
| `CARGO_TARGET_DIR` | `env/target` |
| `RUSTUP_TOOLCHAIN` | Pinned Rust version from `toolchain.versions.json` |
| `npm_config_cache` | `env/npm-cache` |
| `PATH` prefix | Project Cargo launchers and Platform-Tools |

The script makes no persistent system or user environment change. This setup
isolates project dependencies, not all Windows/WebView2/ADB runtime data.

## Webview and Server

The development server listens on loopback at port 1420. Vite uses `strictPort`;
another process on that port is a startup error. Both Tauri development and
standalone browser preview use this port.

The explicit `?preview=1` query enables synthetic reads only in a normal
browser. It does not enable a device API over HTTP. Production loads built
frontend assets from the application bundle and does not need Vite.

Window dimensions, CSP, and bundled resources are declared in
[tauri.conf.json](../../src-tauri/tauri.conf.json). Main-window core and native
dialog permissions are declared in
[capabilities/default.json](../../src-tauri/capabilities/default.json).

## Runtime Limits and Records

Read helpers have a 30-second timeout; streamed task subprocesses have a
one-hour timeout per subprocess. These limits are currently source constants,
not user settings. Queue state is in memory and has no startup recovery.

Installation and build environment records are local artifacts. They are not
runtime configuration files for the packaged app. See [Privacy](../../PRIVACY.md),
[Dependencies](../development/dependencies.md), and
[Troubleshooting](troubleshooting.md).
