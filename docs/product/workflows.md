# Product Workflows

## Connect and Inspect

Overview shows the selected headset, connection method, Android version,
third-party app count, shared-storage capacity, battery, and recent tasks.
Users can choose a device, and choose a connection when several ready
transports belong to that device. USB is preferred by default.

No-device and authorization-required states provide connection guidance. Device
discovery refreshes automatically and can be requested manually. The user must
enable developer mode and authorize USB debugging in the headset. Existing
Wi-Fi connections appear through ADB; there is no pairing or wireless setup UI.

The app distinguishes shared capacity from private app storage. Battery or
version information can be unavailable; it must not be replaced with sample
measurements in desktop mode.

## Manage Applications

Applications lists package names and version codes. Search filters package IDs;
"Show system apps" includes system packages. Details display version metadata
and installed APK paths. Generic artwork is used instead of extracted app icons.

"Install APK" opens a native file picker for one or more ordinary APK files.
Dropping only APKs anywhere in the desktop window opens the same installation
review. The user reviews the files and target, then queues an independent
installation for each file. Removing an item from the review does not delete
the original file.

Compatible updates use Android's replace-install behavior and normally retain
app data. Signature mismatch, downgrade, missing splits, CPU incompatibility,
and insufficient storage are reported as failures. There is no automatic
uninstall to work around those failures and no XAPK/APKS/APKM extraction or
split-set installation.

Export asks for a local parent folder and saves all installed APK files into a
new package/task-named directory. System packages can also be exported when
readable. Exports do not include private app data or saved games.

Uninstall requires confirmation and removes a third-party app and its app data.
System-app uninstall is disabled in the UI and rejected by the backend.

## Manage Files

Files starts at `/sdcard`. Shortcuts open shared storage, Downloads, Movies, or
Android OBB storage. Users can enter a shared path, navigate to a parent, open
directories, filter the current listing, and select multiple items. Filtering
is local to the current folder; it is not a recursive device search.

| Action | Behavior |
| --- | --- |
| Upload files | Pick one or more local files; each becomes a task in the current remote folder |
| Upload folder | Pick one local directory; retain its name beneath the current remote folder |
| Drag/drop other items | On Files, review an upload confirmation for the current folder; outside Files, show guidance |
| Download | Pick a local parent directory; queue the chosen remote items using their existing basenames |
| New folder | Enter a single name beneath the current remote folder |
| Rename | Enter a new basename in the item's existing parent |
| Delete | Confirm permanent deletion of selected items, including directory contents |

Dropping only APK files opens installation review even on Files. To copy an APK
as a file, use "Upload files". A cancelled file picker creates no tasks.

Existing final targets are refused rather than silently replaced. Transfers use
temporary paths, and failures can leave a reported temporary item. Downloads
can reject names that Windows cannot represent, case-only collisions, or
symbolic links in a directory tree. There is no recycle bin or undo.

Android controls access to `Android/data` and `Android/obb`; some items can be
inaccessible. The named Android container directories are protected from
rename/delete, while their accessible contents can be managed. General browsing
does not expose private app directories.

## Follow Tasks

Task queue shows work from the current session across devices. Recent activity
shows the latest tasks on Overview, and a floating shortcut appears while work
is active. Navigating or closing the queue dialog leaves tasks running.

All queued task kinds can be cancelled before execution. Running uploads,
downloads, and exports support cancellation; installs, uninstalls, folder
creation, rename, and deletion do not. Cancellation is cooperative and may need
to wait for a preflight query or finalization step.

Progress can be indeterminate when ADB does not report a percentage. A task is
successful only after its operation and publication complete. Errors include
actionable ADB or cleanup details, which may contain private paths.

Changing the selected device does not redirect queued tasks. There is no
automatic retry, resume after restart, or persistent activity log. Wait for
active work before exiting; closing the app is not an undo operation.

## Preview

The explicit browser preview shows "Preview · sample data", fictional devices,
and synthetic package/file entries. It supports read-only layout inspection and
does not provide a way to install, transfer, or mutate device data. It is not a
fallback used after a desktop connection fails.

Related: [Design](../../DESIGN.md), [Runtime workflows](../architecture/workflows.md),
[Troubleshooting](../operations/troubleshooting.md), and [Privacy](../../PRIVACY.md).
