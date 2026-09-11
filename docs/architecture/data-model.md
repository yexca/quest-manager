# Data Model and IPC

These are in-memory IPC structures, not database entities. Rust definitions in
[adb.rs](../../src-tauri/src/adb.rs), [metadata.rs](../../src-tauri/src/metadata.rs), and [tasks.rs](../../src-tauri/src/tasks.rs)
must agree with [src/types.ts](../../src/types.ts). Serde exposes struct fields
as camelCase; task kinds are lowercase strings.

## Read Models

| Model | Meaning and fields |
| --- | --- |
| `Device` | `id`, `model`, and `transports`; grouping of discovered connections |
| `Transport` | `serial` for command targeting, `kind` (`usb` or `wifi`), raw ADB `state` |
| `DeviceInfo` | Model, Android version, nullable battery percentage, charging state, storage byte counts |
| `AppPackage` | `packageName`, string `versionCode`, and `system` flag |
| `AppDetails` | Package/version, `apkPaths`, `apkFiles` with nullable size/modified time, nullable `apkSize`, install/update times, installer, SDK/ABI, app ID, Android user, enabled state, state flags, permissions, `assets` |
| `AppAssets` | Nullable `displayName`/`iconDataUrl`, VR declarations, signing schemes, certificate SHA-256 fingerprints and availability notes; persisted in a bounded private cache |
| `Permission` | Permission `name`, nullable `granted` boolean and `kind` (Requested, Install, Runtime) for the selected Android user |
| `LocalApk` | Package/version, byte size, source size/mtime stamp, default launcher assets, split flag and nullable verity-signature presence |
| `FileEntry` | Name, remote path, kind, byte size, and `modifiedAt` as Unix epoch milliseconds |

File kinds are `directory`, `file`, `symlink`, and `other`. Directory size is
not a recursively calculated total. Version strings can be `Unknown` when
parsing finds no value; metadata availability uses null instead of invented
values. APK modified times are Unix seconds; install/update strings retain the
headset's local time. App ID is the package app ID, not a multi-user UID.

## Commands

Registered commands live in [lib.rs](../../src-tauri/src/lib.rs), and the frontend
calls them through [api.ts](../../src/api.ts).

| Command | Frontend arguments | Result |
| --- | --- | --- |
| `list_devices` | None | `Device[]` |
| `device_info` | `device` | `DeviceInfo` |
| `list_apps` | `device`, `includeSystem` | `AppPackage[]` |
| `app_details` | `device`, `package` | `AppDetails` |
| `clear_metadata_cache` | None | Success or error; deletes cached JSON under the service gate |
| `inspect_apk` | `source` (absolute local APK path) | `LocalApk`; no device needed |
| `list_files` | `device`, `path` | `FileEntry[]` |
| `start_task` | `request` | Initial `TaskSnapshot` |
| `list_tasks` | None | Current task snapshots |
| `cancel_task` | `id` | Success or error |
| `open_project_repository` | None | Opens the fixed public GitHub repository in the default browser; success or error |

The repository command accepts no URL, path or command arguments. It opens only
`https://github.com/yexca/quest-manager` through Windows and does not use ADB.

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

Installation optionally accepts `installOptions`: `sourceStamp`, nullable
`displayName` and base64 `iconPng`, and a `compatibility` boolean. Presence is the
explicit opt-in to local preparation. Appearance edits always include compatible
signing; without appearance changes, `compatibility` must be true. Other task
kinds reject these options. A changed source stamp rejects preparation.

Local destinations are folders, not the final output filename. Upload/download
preserve the source basename. Export creates a folder named from the package
and task ID. `mkdir` and `rename` intentionally use `destination` for a basename;
do not pass a full path from a new caller.

## Task Snapshots and Events

`TaskSnapshot` is called `Task` in TypeScript. It contains `id`, captured
transport `device`, monotonic per-task `revision`, `kind`, `label`, `status`, `detail`, nullable `progress`, and
`createdAt` in epoch milliseconds. IDs combine an epoch timestamp with a
process-local counter; they are not durable identities across installations.

Statuses are `queued`, `running`, `success`, `failed`, and `cancelled`.
`progress` is a percentage when available; null means no numeric progress.
Backend updates emit the complete snapshot on `task-updated`.

The frontend registers its listener before reading `list_tasks`, then merges
events, snapshots and `start_task` responses by ID and increasing revision.
The initial queued revision is zero. Duplicate or older revisions are ignored.
`clear_completed_tasks` removes only terminal records and returns their IDs;
frontend session-only tombstones prevent delayed messages from restoring them.
These APIs do not persist history or clear device files. `list_tasks` supplements
events but is not a durable event log. See [Workflows](workflows.md).

When a native close request encounters queued/running tasks, the backend prevents
closing and emits `app-close-blocked`. The UI defaults to keeping the app open;
only explicit confirmation invokes `exit_with_active_tasks` to exit anyway.
