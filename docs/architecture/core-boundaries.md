# Core Boundaries

## Device Identity and Transport

`Device.id` identifies a physical device when `ro.serialno` can be read.
`Transport.serial` identifies one ADB connection and is the value passed to
`adb -s`. A Wi-Fi address and a USB serial can refer to the same physical device.

Discovery falls back to the transport serial when the physical identity cannot
be read, including unauthorized connections. Grouping is therefore best effort.
Do not assume every connection can be deduplicated or treat an unavailable
transport as ready. Only state `device` is used for commands by the UI.

Each task captures its transport when queued. A changed selection, disappeared
connection, or new USB connection does not redirect that task. Automatic
failover would be a new behavior requiring an explicit targeting design.

## General File Management

The shared namespace is `/sdcard`; `/storage/emulated/0` is an accepted alias.
`normalize_remote` validates and normalizes the lexical path, while `shared_path`
uses device-side `readlink -f` to reject a resolved path outside shared storage.
Preserve both checks; string-prefix checks alone are insufficient.

The shared-storage root itself cannot be downloaded, renamed, or deleted.
`mutation_path` also rejects the named containers `/sdcard/Android`,
`/sdcard/Android/data`, and `/sdcard/Android/obb` for rename/delete. Their contents
can be managed when Android grants access. These checks do not bypass Android
permissions or promise access to all files within those directories.

Path validation and use are separate operations. Concurrent changes on the
device can race them; the app does not provide a race-free sandbox or a
transaction across external writers. Do not document a stronger guarantee.

## APK Operations

Installation accepts a regular local `.apk` file and invokes `adb install -r`.
Several selected files create independent tasks; they do not form one split
installation session. Android enforces signature, version, ABI, and storage
requirements.

Uninstallation checks the current third-party package list in the backend.
Hiding a button for system apps is not the only protection.

APK export obtains paths through `pm path` and pulls all returned APK files.
Those paths can lie outside `/sdcard`. Application metadata also reads bounded
byte ranges from these package-manager-derived paths. Neither operation accepts
arbitrary remote paths from the webview or grants saved-game access.

Metadata decoding accepts bounded ZIP resources and raster artwork. Render
labels and manifest data as text; do not expose resource XML as web markup.
The cache is private local data. It must not hold full APKs, persisted permission
state, diagnostics, or task history. Resource parsing failures preserve package
details and use explicit unknown values/fallback artwork.

## Task and Transfer Semantics

- Mutations are serialized globally; read queries can overlap them.
- Ordinary upload/download/rename targets must be absent. File transfers do not
  silently overwrite existing final items. Compatible APK replacement is the
  intentional exception through `install -r`.
- Transfers write to task-specific `.quest-manager-*.partial` paths before
  publishing a final name. Cleanup is attempted after ordinary failures or
  cancellation, and cleanup failures are reported.
- Cancellation is cooperative. Queued tasks can be cancelled; running upload,
  download, and export tasks support cancellation. Other running task kinds do
  not. Preflight queries and final publication may delay or race cancellation.
- Tasks are not persistent or crash-recoverable. There is no automatic retry,
  rollback of completed actions, or transaction spanning several queued tasks.

## Local Paths and Trust

Local sources and destination directories must be absolute and canonicalizable.
Downloads validate Windows filename rules, case collisions, and descendant
symbolic links before pulling a tree. Do not interpret a remote filename as a
host path without these checks.

The local user and bundled application are trusted. IPC arguments and device
output still need validation. The current webview is not a permission boundary
against code already executing with the user's privileges. See
[Secure development](../development/security.md) and [Privacy](../../PRIVACY.md).
