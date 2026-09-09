# Data Model and IPC

These are in-memory IPC structures, not database entities. Rust definitions in
[adb.rs](../../src-tauri/src/adb.rs) and [tasks.rs](../../src-tauri/src/tasks.rs)
must agree with [src/types.ts](../../src/types.ts). Serde exposes struct fields
as camelCase; task kinds are lowercase strings.

## Read Models

| Model | Meaning and fields |
| --- | --- |
| `Device` | `id`, `model`, and `transports`; grouping of discovered connections |
| `Transport` | `serial` for command targeting, `kind` (`usb` or `wifi`), raw ADB `state` |
| `DeviceInfo` | Model, Android version, nullable battery percentage, charging state, storage byte counts |
| `AppPackage` | `packageName`, string `versionCode`, and `system` flag |
| `AppDetails` | Package name, version name/code, and all installed `apkPaths` |
| `FileEntry` | Name, remote path, kind, byte size, and `modifiedAt` as Unix epoch milliseconds |

File kinds are `directory`, `file`, `symlink`, and `other`. Directory size is
not a recursively calculated total. App names and icons are not part of the
current package model. Version strings can be `Unknown` when parsing finds no
value; battery level can be null.

## Commands

Registered commands live in [lib.rs](../../src-tauri/src/lib.rs), and the frontend
calls them through [api.ts](../../src/api.ts).

| Command | Frontend arguments | Result |
| --- | --- | --- |
| `list_devices` | None | `Device[]` |
| `device_info` | `device` | `DeviceInfo` |
| `list_apps` | `device`, `includeSystem` | `AppPackage[]` |
| `app_details` | `device`, `package` | `AppDetails` |
| `list_files` | `device`, `path` | `FileEntry[]` |
| `start_task` | `request` | Initial `TaskSnapshot` |
| `list_tasks` | None | Current task snapshots |
| `cancel_task` | `id` | Success or error |

Every `device` argument is an ADB transport serial, including
`TaskRequest.device`. It is not the grouped `Device.id`. Errors are strings;
they can contain diagnostic details and are not automatically redacted.

## Task Requests

`TaskRequest` always contains `device` and `kind`. Other fields have a
kind-specific meaning:

| Kind | `source` | `destination` | `packageName` |
| --- | --- | --- | --- |
| `install` | Absolute local APK path | Unused | Unused |
| `uninstall` | Unused | Unused | Installed third-party package |
| `upload` | Absolute local file/folder | Existing remote parent directory | Unused |
| `download` | Remote file/folder | Existing local parent directory | Unused |
| `export` | Unused | Existing local parent directory | Installed package |
| `mkdir` | Existing remote parent directory | New basename, not a path | Unused |
| `rename` | Existing remote item | New basename, not a path | Unused |
| `delete` | Existing remote item | Unused | Unused |

Local destinations are folders, not the final output filename. Upload/download
preserve the source basename. Export creates a folder named from the package
and task ID. `mkdir` and `rename` intentionally use `destination` for a basename;
do not pass a full path from a new caller.

## Task Snapshots and Events

`TaskSnapshot` is called `Task` in TypeScript. It contains `id`, captured
transport `device`, `kind`, `label`, `status`, `detail`, nullable `progress`, and
`createdAt` in epoch milliseconds. IDs combine an epoch timestamp with a
process-local counter; they are not durable identities across installations.

Statuses are `queued`, `running`, `success`, `failed`, and `cancelled`.
`progress` is a percentage when available; null means no numeric progress.
Backend updates emit the complete snapshot on `task-updated`.

The frontend merges events and queried snapshots by ID. Events can arrive
before the response to `start_task`; do not overwrite a newer event with that
initial queued response. `list_tasks` supplements events but is not a durable
event log. See [Workflows](workflows.md).
