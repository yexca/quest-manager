# Data Model and IPC

These are IPC structures rather than database entities. Discovery, task, and
headset read models are in memory; named device profiles and connection
preferences are persisted by [device_profiles.rs](../../src-tauri/src/device_profiles.rs).
Rust definitions in [adb.rs](../../src-tauri/src/adb.rs), [device_profiles.rs](../../src-tauri/src/device_profiles.rs),
[metadata.rs](../../src-tauri/src/metadata.rs), and [tasks.rs](../../src-tauri/src/tasks.rs)
must agree with [src/types.ts](../../src/types.ts). Serde exposes struct fields
as camelCase; task kinds are lowercase strings.

## Read Models

| Model | Meaning and fields |
| --- | --- |
| `Device` | `id`, `model`, optional saved `displayName` and `connectionPreference`, and `transports`; grouping of discovered connections |
| `Transport` | `serial` for command targeting, `kind` (`usb` or `wifi`), raw ADB `state` |
| `DeviceProfile` | Physical `id`, model, user-facing `displayName`, and `connectionPreference` (`auto`, `usb`, or `wifi`) |
| `DevicePreferences` | Saved `profiles` plus the `autoSwitch` setting; stored in the private device-settings file |
| `DeviceInfo` | Model, Android version, nullable battery percentage, charging state, storage byte counts |
| `DevicePowerSettings` | Nullable `stayAwake` value read from the headset's charging sleep setting, plus raw setting text |
| `AppPackage` | `packageName`, string `versionCode`, `system` flag, nullable `installer` and `apkPath` |
| `AppDetails` | Package/version, `apkPaths`, `apkFiles` with nullable size/modified time, nullable `apkSize`, install/update times, installer, SDK/ABI, app ID, Android user, enabled state, state flags, permissions, `assets` |
| `AppAssets` | Nullable `displayName`/`iconDataUrl`, VR declarations, signing schemes, certificate SHA-256 fingerprints and availability notes; persisted in a bounded private cache |
| `Permission` | Permission `name`, nullable `granted` boolean and `kind` (Requested, Install, Runtime) for the selected Android user |
| `LocalApk` | Package/version, byte size, source size/mtime stamp, default launcher assets, split flag and nullable verity-signature presence |
| `LocalObb` | Absolute local `source`, `sourceStamp`, original `name` and byte `size`; no persistent cache |
| `FileEntry` | Name, remote path, kind, byte size, and `modifiedAt` as Unix epoch milliseconds |

File kinds are `directory`, `file`, `symlink`, and `other`. Directory size is
not a recursively calculated total. Version strings can be `Unknown` when
parsing finds no value; metadata availability uses null instead of invented
values. APK modified times are Unix seconds; install/update strings retain the
headset's local time. App ID is the package app ID, not a multi-user UID.

`LocalApk.versionCode` is a decimal string containing the combined
`versionCodeMajor` and `versionCode` manifest integers, read through AAPT2's
manifest tree because badging omits the high bits. A failed or invalid version
read returns an empty string and prevents version comparison. The installation
review compares complete numeric strings with `BigInt`, without converting IPC
values to JavaScript numbers. Installed status uses fresh `list_apps` data with
`includeSystem: true`; no new IPC command or persistent cache is involved.

## Commands

Registered commands live in [lib.rs](../../src-tauri/src/lib.rs), and the frontend
calls them through [api.ts](../../src/api.ts).

| Command | Frontend arguments | Result |
| --- | --- | --- |
| `list_devices` | None | `Device[]` |
| `device_preferences` | None | `DevicePreferences` |
| `save_device_profile` | `profile: DeviceProfile` | Updated `DevicePreferences` |
| `set_device_auto_switch` | `enabled` | Updated `DevicePreferences` |
| `wireless_connection` | `request: WirelessRequest` | `WirelessResult` |
| `start_wireless_qr` | None | `WirelessQrSnapshot` |
| `wireless_qr_status` | `id` | `WirelessQrSnapshot` |
| `cancel_wireless_qr` | `id` | None |
| `device_info` | `device` | `DeviceInfo` |
| `device_power_settings` | `device` | `DevicePowerSettings` |
| `set_device_stay_awake` | `device`, `enabled` | `DevicePowerSettings` |
| `list_apps` | `device`, `includeSystem` | `AppPackage[]` |
| `app_details` | `device`, `package` | `AppDetails` |
| `clear_metadata_cache` | None | Success or error; deletes cached JSON under the service gate |
| `inspect_apk` | `source` (absolute local APK path) | `LocalApk`; no device needed |
| `inspect_obb_files` | `sources` (1–128 absolute local OBB file paths) | `LocalObb[]`; no device needed |
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

`WirelessRequest` is a strict tagged union: `{method: "pair", address, code}`
or `{method: "usb", device}`. Unknown variants and fields
are rejected. Addresses are numeric IP/port pairs; USB `device` is a captured
transport serial, not a physical identity. `WirelessResult` contains nullable
`serial` and English `message`. Both variants pair/reuse trust and connect;
only a verified ready connection returns its transport serial. USB messages
distinguish reused authorization from newly paired trust. Connection setup is a bounded
IPC operation under the task queue permit, with no task snapshot or new event.

`WirelessQrSnapshot` contains a random `id`, `status`, English `message`, nullable
`qrDataUrl`, `expiresAt` in epoch milliseconds, and nullable verified transport
`serial`. Statuses are `waiting`, `pairing`, `connecting`, `connected`, `failed`,
`cancelled`, and `expired`. The deadline is enforced using monotonic time in the
worker. Start takes the global queue permit; status reads stored state only.
Cancellation and replacement require the exact session ID. QR images clear
when pairing starts or cancellation/completion occurs; raw secrets are not
separate IPC fields. Only one session snapshot is retained, in memory.

`device_power_settings` reads Android's `stay_on_while_plugged_in` global
setting. `set_device_stay_awake` invokes the named `svc power stayon` command,
then rereads the setting under the same global mutation permit. The value is
headset state, not an app preference; it is not persisted by Quest Manager.

`device_preferences` loads the private local profile file at startup. Saving a
profile trims and validates the physical identity, display name, and connection
preference, rejects case-insensitive duplicate names, writes a temporary file,
and replaces the settings file before publishing the new in-memory state.
`set_device_auto_switch` uses the same persistence path. The file contains no
pairing codes, ADB keys, transport addresses, or task history.

## Optional Setup IPC

- `lightning_releases(refresh: boolean)` returns `LightningCatalog` with Launcher
  and Navigator arrays. Each release has `tag`, `assetId`, `size`, `publishedAt`,
  and nullable `sha256`. Download URLs and package identities stay backend-owned.
- `lightning_recommendation(tag: string)` returns `launcherTag` and nullable
  `navigatorTag`; errors/missing declarations do not imply support.
- `navigator_enabled(device: string)` returns a boolean or error for the captured
  transport's active Android user. UI errors display activation as unknown.
- `open_lightning_repository()` opens only the fixed upstream public repository.

`TaskRequest.lightning` contains nullable `launcherAsset` and `navigatorAsset` IDs.
At least one is required. This field is exclusive with local source/destination,
package hint, preparation and OBB options; `kind` remains `install`. The normal
queue and `apkInstalled` partial-completion flag apply. It supplies no package hint
because the task can install two packages. Device profiles are independent of
task state and never retarget a queued task.

## Task Requests

`TaskRequest` always contains `device` and `kind`. Other fields have a
kind-specific meaning:

| Kind | `source` | `destination` | `packageName` |
| --- | --- | --- | --- |
| `install` | Absolute local APK path | Unused | Optional validated package refresh hint from preview; never used to target installation |
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

Installation can also accept `obb`: an `apkSourceStamp` and 1–128 `files`, each
containing `source` and `sourceStamp`. Other kinds reject it. OBB requests require
a valid preview `packageName`; this is checked against the staged APK manifest
before any device writes. It is never used as the destination authority. The
backend rereads local names/sizes, checks stamps and computes hashes itself.
No target path or caller-supplied checksum is accepted.

Local destinations are folders, not the final output filename. Upload/download
preserve the source basename. Export creates a folder named from the package
and task ID. `mkdir` and `rename` intentionally use `destination` for a basename;
do not pass a full path from a new caller.

## Task Snapshots and Events

`TaskSnapshot` is called `Task` in TypeScript. It contains `id`, captured
transport `device`, monotonic per-task `revision`, `kind`, nullable `packageName`
refresh hint, `label`, `status`, `detail`, nullable `progress`, and
`createdAt` in epoch milliseconds. IDs combine an epoch timestamp with a
process-local counter; they are not durable identities across installations.

`includesObb` identifies composite installs. `apkInstalled` starts false and
becomes true after Android accepts the APK, retaining that value if later OBB
work or local cleanup fails. Terminal failed snapshots with this flag invalidate
application data; composite installs also refresh Files and storage information.
The flag does not make a failed task successful or imply that the game launched.

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
