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

## APK Installation

Signature mismatch and version downgrade are Android installation failures.
Use a compatible APK; do not silently uninstall an existing application to
work around them, because uninstall can remove app data.

Check available space and device ABI for the corresponding errors. A missing
split error means the package needs an installation flow this version does not
provide. Several selected APKs are separate installs, not a split session.

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
