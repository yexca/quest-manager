# Product Workflows

## About and Help

The sidebar's About page works without a headset and displays the project
introduction, yexca/Codex development credit, GPT-6-Astra model attribution,
core dependency versions, source repository and AGPLv3 license. Its full license
is embedded for offline reading. The repository link opens the default browser
in the desktop app or a new browser tab in preview. No device task is created.
Help remains a separate dialog for connection, transfer and signing-key guidance.

## Lightning Launcher

The **Devices** page contains the Lightning Launcher setup entry for the
selected ready transport. It can install or update the Launcher and its
optional Navigator service without depending on the Applications inventory.

The setup dialog captures its target connection when opened. It fetches releases
only on opening/refresh and reads current installed versions. It can install the
GitHub Launcher edition, the Navigator service, or both. Recognized store editions
do not prevent an explicit GitHub edition install.
The optional service checkbox starts off. Existing installations show versions;
Navigator activation is queried separately and failures remain unknown.
Release options mark the installed version, and selecting it shows an inline
already-installed notice. Markers compare the version name of the same package;
a store edition does not mark the GitHub edition as installed. The task still
checks the APK's actual version code before deciding whether to skip installation.

The version catalog includes up to 500 published releases with the exact expected
APK assets, excluding drafts and prereleases. The default Launcher is the highest
numeric stable version. The Navigator default comes from the selected Launcher
tag's explicit upstream addon declaration, or from a matched installed version
when only adding the service. Missing/changed source declarations never imply
compatibility. **Show other service versions** permits an explicit unverified
selection with an inline notice. Original source, dates and sizes are visible.
Changing the Launcher target or refreshing releases resets the service choice to
that target's recommendation; stale responses cannot retain a previous pairing.
An installed service is compared with the target recommendation even when its
install checkbox is off. A difference or unreadable version is shown as unverified,
and updating the service remains an explicit choice. An author recommendation is
not a guarantee of compatibility with every headset OS version.

**Download & install** creates one task in the global queue. Every selected APK
is downloaded and verified before the first install. Downloads check published
size and SHA-256 when available; all APKs require matching package/version,
ordinary APK structure and a valid original signature. No re-signing, automatic
downgrade, uninstall or retry occurs. Matching installed version codes are skipped.
Launcher failure prevents the service install. A later failure preserves and
reports earlier completed installations; applications refresh on partial success.
Task-owned download files are cleaned after success/failure, with cleanup errors
reported. Queued setup can be cancelled; running setup cannot.

A service-only request requires a recognized installed Launcher. Installation
never enables Accessibility settings. The dialog/result explains how to activate
it inside Lightning Launcher and check status again. Enabling requires the
Navigator system interface; release recommendations are not proof of compatibility
with every headset OS. Preview versions are fictional and downloads/writes disabled.

## Connect and Inspect

Overview shows the selected headset, connection method, Android version,
third-party app count, shared-storage capacity, battery, and recent tasks.
The **Devices** page is the connection and headset settings surface. It lists
USB and Wi-Fi transports, opens device registration through **Add connection**,
and shows which transport is active. A registered device keeps its display name,
preferred connection, and offline entry between launches. When both transports
are ready, **Automatic** uses USB first and Wi-Fi as the fallback; **USB only**
and **Wi-Fi only** restrict the selected transport.

**Add connection** offers three methods: **USB**, **QR code**, and **Pairing
code**. USB lists detected USB transports and requires the user to accept the
headset's debugging prompt before continuing. After the USB profile is saved,
the dialog asks whether to run USB-assisted wireless setup so both transports
can be available. QR and pairing-code flows connect first, then ask for the
device name and preferred connection. Names default to the model and must be
unique; an existing profile can be renamed from its device settings.

The selected device remains selected when another headset appears or when its
transports go offline. A newly detected device that is not registered produces
an **Add device** notice. For another registered device, the notice offers a
manual switch. The **Automatic device switching** setting can switch to a newly
ready registered device when no task is active; an active task defers the switch.
Manual refresh waits for a fresh device snapshot; an older pending poll cannot
overwrite that refresh.

After selecting a ready device, the Devices page can read its stay-awake setting
and toggle whether the headset stays awake while charging. This updates the
headset's global Android setting to help long Wi-Fi transfers; it is device state,
not a saved app preference. The page also contains the Lightning Launcher setup
entry.

The headset illustration follows the selected device's discovered model: Quest 3,
Quest 3S, or Quest 2. Case, spaces, underscores, hyphens, and Meta/Oculus prefixes
are normalized for artwork selection. Unknown models use a generic illustration
and retain the same device operations. Artwork does not indicate tested support
for every feature on that model.

No-device and authorization-required states provide a link to **Devices**.
Discovery refreshes automatically and can be requested manually. Developer mode
and headset debugging authorization are required. Existing Wi-Fi connections
also appear through ADB.

The app distinguishes shared capacity from private app storage. Battery or
version information can be unavailable; it must not be replaced with sample
measurements in desktop mode.

### Wireless Setup

The dialog offers three icon-and-text tabs in one row, in this order:

- **USB setup** requires a detected USB headset and an accepted debugging prompt.
  It first reuses a ready wireless connection whose physical identity matches. Otherwise
  it enables system wireless debugging for the current network and tries existing
  trust before pairing. A working authenticated connection reports that pairing
  is already available; an authentication failure starts pairing and connection.
  Network failures are not reported as proof of pairing. The temporary helper is
  removed after setup; no APK is installed. A disappeared selection never
  substitutes another USB headset.
- **QR code** generates an Android debugging QR locally, valid for two minutes.
  In Lightning Launcher, open **Android Settings → System → Developer options**,
  then **Debugging → Wireless debugging → Pair device with QR code**. The app's
  **Lightning Launcher setup** can install the launcher. Scanner availability
  depends on the headset OS; use USB setup if absent. Meta mobile QR pairing uses
  another protocol. ADB mDNS finds the exact scanner, pairs once, and connects
  the verified device. Cancelling stops the app session; pairing already submitted
  to the shared ADB server may still finish.
- **Pairing code** accepts the numeric pairing IP/port and six-digit code from
  the headset's open Wireless debugging pairing screen. Pairing and connection
  happen automatically, using the paired device identity. The pairing port
  differs from the connection port; the app discovers the latter. Not every
  Horizon OS version exposes this entry.

Keep the headset awake and both devices on the same reachable local network.
Pairing cannot start in deep sleep: put on the headset or press its power button,
and keep the display on until setup finishes. This instruction appears above all tabs.
The top note explains that deep sleep disconnects Wi-Fi and ADB reconnects on wake
on the same network while wireless debugging remains enabled. This is ADB's
existing reconnection behavior; the app does not resume failed tasks or silently
pair again. Reboots or network changes may require explicit setup again.
There is no pairing-code retention. Closing the dialog does not disable
wireless debugging or revoke ADB trust.

Setup is disabled while tasks are queued/running. The backend independently
holds the global mutation permit and rejects busy setup. Code setup has a
90-second deadline; USB has bounded preflight, setup and awaited cleanup stages;
QR sessions have a two-minute total deadline. The dialog prevents closing during the request,
with a dedicated cancel action for QR sessions. The native exit guard also
covers setup in progress.

Connection success requires a matching ready transport (and ADB's success
response for explicit connect). QR/code verify the paired GUID through a readable
device property or its exact authenticated TLS service transport when the
property is hidden. USB verifies the same physical identity over both connections.
Discovery then refreshes and makes that transport available on the Devices page.
Already queued tasks keep their original transport. USB remains the default when
both transports are ready; Wi-Fi is selected after USB is unavailable.
Preview exposes all forms but disables connection and pairing actions.

## Manage Applications

Applications first lists packages, then progressively reads names, raster icons,
versions, total APK size and enabled state. Search matches names and package
IDs; "Show system apps" includes system packages. Unknown values remain explicit,
and an unreadable package does not stop the remaining list from loading.

Install source is a separate column and filter: Meta Store (inferred), Sideloaded
(inferred), Other installer, and Unknown source. The reported installer
`com.oculus.ocms` is treated as a Meta distribution hint and `com.android.shell`
as an ADB hint. Exact installer identities are used, never the application's
name or package prefix. A successful installation with a known package through
Quest Manager also marks that package as sideloaded in the current transport's
session cache. This hint expires when the package identity changes or the
connection selection changes. Empty installer records stay unknown; some ADB
installs have no installer record. These labels do not prove store purchases,
publisher identity, or match every Horizon OS distribution channel/version.

The selected connection's full application list and loaded details stay in
memory while browsing. System/source filters are local; revealing system apps
loads only their missing details. Successful installs/uninstalls refresh the
lightweight list, prune removed packages and reload affected packages while
preserving other names and icons. Same-version installs carry a package refresh
hint; if it is unavailable, all details are revalidated without blanking artwork.
Version, installed APK path or installer changes also invalidate an entry.
Explicit refresh revalidates all entries while keeping the displayed artwork.
Opening details reads live information again. A connection change starts a fresh
session cache; the private disk artwork cache remains available. Lists and
source hints are not persisted across launches.

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

Each APK also has an optional **OBB files** area. **Add OBB files** accepts one or
more local `.obb` files, including custom names such as `audio.obb`. While review
is open, native drops attach OBB files to the selected APK, including when Files
is the underlying page; they do not start ordinary uploads or replace the APK
selection. Attachments can be removed individually. Switching APKs keeps their
attachments separate. A pending picker/read retains its original APK target.
Unreadable package names and recognized split APKs disable OBB selection/drops.

The displayed destination is `/sdcard/Android/obb/<package>/`. One task installs
the APK and its OBB files. It checks all selected files and existing destinations
before installing the APK, then transfers and SHA-256 verifies each OBB under its
original name. Existing identical files are reused; different content, unsafe
directories and non-file collisions are refused. Running installs, including
OBB work, cannot be cancelled. An OBB failure keeps the installed APK and completed
files, reports the verified count and any cleanup failure, and stops that task.
The app/files views refresh even if the overall result failed after installing.
Select the APK and files again for a manual retry. This does not resume a saved
task, overwrite different data, or automatically uninstall the app. Other queued
tasks remain independent.

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
The confirmation shows the cached icon, display name, package ID and version,
plus the target headset and connection captured when it opened. Missing metadata
uses a generic icon and package ID without rereading the application list.
Changing the selected connection cannot retarget this confirmation; its action
is disabled while the original connection is unavailable. The fictional browser
preview allows opening the confirmation but keeps uninstallation disabled.
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
