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
the server's lifecycle or reset authorization keys. Wireless connection/pairing
and USB-assisted TCP/IP enabling occur only through explicit setup actions.
It can share the server with Android Studio and other ADB clients.

The **Devices** page is the connection and headset settings surface. It lists
USB and Wi-Fi transports, keeps devices seen during the current session as
offline entries, and opens wireless setup through **Add connection**. When both
transports are ready for one physical device, USB is selected first; Wi-Fi is
the fallback. New tasks capture the selected transport, and changing the
selected device does not migrate already queued work. The page also reads the
headset's stay-awake setting and can install Lightning Launcher. The stay-awake
toggle changes the headset's global charging setting but is not stored by the
app. See [Data model](../architecture/data-model.md).

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

AAPT2 is resolved from `env/aapt2` in development and the `aapt2` resource
directory in release. Metadata uses a 64 MiB read budget, 256 KiB range pages,
8 MiB central-directory limit, 32 MiB resource-table limit and 2 MiB icon limit.
ZIP64 metadata is not supported. Range reading checks a 90-second deadline
between reads (an active ADB read can take another 30 seconds); each AAPT2 call
times out after 15 seconds. Fallbacks preserve the rest of the application list.
The cache is bounded to 512 JSON entries / 64 MiB; locations and clearing are
documented in [Privacy](../../PRIVACY.md).

APK preparation resolves its Java, Apktool, apksigner and zipalign tools from
`env/apk-tools` in development and the `apk-tools` resource directory in release.
It does not search for system Java or Android SDK installations. Preparation
requires approximately four APK sizes plus 512 MiB of free local space; resource
editing accepts at most 128 MiB of uncompressed resources. Working copies and
persistent signing keys use the separate locations described in
[Privacy](../../PRIVACY.md#apk-preparation-storage).

Read helpers have a 30-second timeout; streamed task subprocesses have a
one-hour timeout per subprocess. These limits are currently source constants,
not user settings. Queue state is in memory and has no startup recovery.

Installation and build environment records are local artifacts. They are not
runtime configuration files for the packaged app. See [Privacy](../../PRIVACY.md),
[Dependencies](../development/dependencies.md), and
[Troubleshooting](troubleshooting.md).
