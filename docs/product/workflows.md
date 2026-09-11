# Product Workflows

## About and Help

The sidebar's About page works without a headset and displays the project
introduction, yexca/Codex development credit, GPT-6-Astra model attribution,
core dependency versions, source repository and AGPLv3 license. Its full license
is embedded for offline reading. The repository link opens the default browser
in the desktop app or a new browser tab in preview. No device task is created.
Help remains a separate dialog for connection, transfer and signing-key guidance.

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

Applications first lists packages, then progressively reads names, raster icons,
versions, total APK size and enabled state. Search matches names and package
IDs; "Show system apps" includes system packages. Unknown values remain explicit,
and an unreadable package does not stop the remaining list from loading.

Details has Overview, Permissions, APK files, and Signing & VR tabs. It shows
install/update times in headset local time, installer package, SDK levels,
primary/secondary ABI, app ID, active Android user, state flags, requested/granted
permissions, and every installed APK path/size. Null installer data does not
identify a distribution source. Size is APK bytes only, excluding data/cache/OBB.

Names prefer English resources, then the default label, then the package ID.
Base APK resources are read by bounded byte ranges; large game data is not
downloaded. Adaptive/vector, split-only, oversized or unreadable artwork uses a
generic icon. Signing shows v2/v3/v3.1 base APK certificate SHA-256 fingerprints;
v1-only certificates and signing lineage are not decoded. This does not validate
APK integrity or publisher trust. VR declarations are manifest metadata, not
verified compatibility. Details explains availability limitations.

Names, artwork and immutable APK metadata use a bounded private disk cache.
"Clear cached artwork" clears it and pauses background reads while preserving
already displayed data. "Load app details", refresh, or restarting resumes
loading; manually opening details can also cache that application. Device and
permission state are queried again instead of persisted. See [Privacy](../../PRIVACY.md).

"Install APK" opens a native file picker for one or more ordinary APK files.
Dropping only APKs anywhere in the desktop window opens the same installation
review. Local APK previews work without a headset. The review shows the default launcher
name/icon, package, version and size for each file. Unknown artwork is not proof
of missing artwork; device language and launcher behavior can differ. The user
reviews the files and target, then queues an independent installation for each
file. Each file has its own disabled-by-default Modify and re-sign APK switch.

The review checks the selected connection's complete package list, including
system apps, independently of the Applications filter. Each APK shows whether
its exact package name is installed and compares numeric version codes: newer,
same, older, or unavailable. Installed and selected version codes appear together;
version names are display labels, not comparison keys. Older versions show an
attention notice, and same-version replacements remain available. These checks
are advisory and never guarantee a compatible update or force a downgrade.

Check again retries the device query. Missing devices, unreadable APK identity,
and failed queries remain unavailable rather than reporting absence. Local
previews and attempts to install original APKs remain available after a query
failure. Changing connections or completing relevant install/uninstall tasks
refreshes the check and discards obsolete responses. Results are snapshots;
queued work or external changes can alter the installed version before execution.
Each submitted batch captures one transport. Multiple selected files with the
same package name are marked and retain their list order; users can remove
unwanted copies. Re-signing an already installed package adds an update warning.

When enabled, the user can enter one display name across languages and choose a
PNG/JPEG/WebP image (up to 8 MiB), crop/position/zoom it to a 512-pixel square, or
reset the appearance. Name/icon changes rebuild resources and include compatible
signing. Compatibility install can also be enabled without appearance changes;
it re-signs with verity disabled to address some large-APK signature overflows.
It does not guarantee successful installation or game behavior. Preparation
verifies the generated package/version, requested name/icon, payload metadata
and signature before invoking ADB. Ordinary installs remain available when
artwork preview fails; recognized split APKs are refused by this review.

New signatures usually cannot update an original installation. Subsequent modified
updates need the same stable local key. Help explains private key backup/restore;
artwork clearing never removes keys. Existing apps are not automatically
uninstalled. Prepared copies are temporary; originals remain unchanged.
Preparation remains part of the serial install task and cannot be cancelled once
running. Free local space must cover four APK sizes plus 512 MiB. Editing rejects
multiple launcher entries, shared-user packages and resources over 128 MiB. Removing an item from the review does not delete
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
Each task shows its captured transport and, while discovered, the headset model
and connection kind. Clear completed removes successful, failed and cancelled
records from this session; it does not cancel active work or delete transferred files.

All queued task kinds can be cancelled before execution. Running uploads,
downloads, and exports support cancellation; installs, uninstalls, folder
creation, rename, and deletion do not. Cancellation is cooperative and may need
to wait for a preflight query or finalization step.

Progress can be indeterminate when ADB does not report a percentage. A task is
successful only after its operation and publication complete. Errors include
actionable ADB or cleanup details, which may contain private paths.

Changing the selected device does not redirect queued tasks. There is no
automatic retry, resume after restart, or persistent activity log. Wait for
active work before exiting. Closing the window while work remains prompts with
Keep waiting selected by default. Exit anyway can interrupt work and leave
temporary files; closing the app is not an undo operation.

## Preview

The explicit browser preview shows "Preview · sample data", fictional devices,
synthetic package/file entries and sample running, queued, failed and completed
tasks. Task cancellation and clearing are disabled. It supports read-only layout inspection and
does not provide a way to install, transfer, or mutate device data. It is not a
fallback used after a desktop connection fails.

Related: [Design](../../DESIGN.md), [Runtime workflows](../architecture/workflows.md),
[Troubleshooting](../operations/troubleshooting.md), and [Privacy](../../PRIVACY.md).
