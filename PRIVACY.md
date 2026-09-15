# Privacy and Data Handling

Quest Manager manages data between the local computer and a headset through
ADB. The application code has no analytics, telemetry, cloud account, or cloud
sync integration. This describes the application, not the independent behavior
of Windows, WebView2, ADB, or the headset operating system.

## Data Used at Runtime

| Data | Purpose | App-managed retention |
| --- | --- | --- |
| Device serials, transport addresses, model, Android version | Discovery, selection, and command targeting | Current process and UI state |
| Entered pairing addresses and codes | Explicit wireless pairing and connection | Dialog/request memory only; codes clear on submission or method change |
| Current Wi-Fi route/BSSID and generated USB setup credentials | Enable wireless debugging on the selected USB headset and reuse trust or pair | Request/private stdin memory only; not logged or persisted by the app |
| Generated debugging QR and shared secret | Explicit QR pairing and matching ADB mDNS discovery | Local encoding and session memory only; two-minute deadline; QR clears when pairing starts or the session ends |
| Battery and shared-storage totals | Overview | Current UI state |
| Package names, versions, and installed APK paths | Application listing, details, and export | Current UI state and task details |
| Application labels, raster icons, signing fingerprints, VR declarations | Identify and inspect applications | Local metadata cache, bounded to 512 entries / 64 MiB |
| Install/update times, permissions, installer, SDK/ABI, app state | Application details | Current UI state; queried again on refresh |
| Shared filenames, paths, sizes, and timestamps | File browsing and transfer | Current UI state and task details |
| Selected computer paths and ADB output | Task execution and error feedback | Current process and UI state |
| Local APKs and selected icon images | Optional rebuilding, signing and installation | Task-owned temporary copies; cleanup attempted on completion/failure |
| Selected OBB paths, sizes, source stamps and SHA-256 hashes | Attachment review, change detection and transfer verification | Current process/UI only; completed OBB files remain on the headset |
| Per-package signing keys and password files | Allow subsequent locally signed updates | Persistent private local storage; user-managed backup/removal |
| Uploaded, downloaded, or exported files | Explicit user operations | Files remain at their destinations |

There is no app database or persisted task history. This does not mean an
operation is undone when the app closes. Completed installs, uninstalls,
renames, deletions, and transfers remain effective. Interrupted operations can
leave temporary files. WebView2 and ADB may keep their own runtime data outside
the application's state.

Applications retains the selected transport's complete package list and loaded
details in memory across filters and ordinary task refreshes. Removed entries
are pruned; changing connections discards this view cache. Installer-derived
source labels and successful-install source hints are session-only data and
are not written into the artwork cache. Opening details or explicitly refreshing
revalidates live values. Package mutation hints stay in process memory to refresh
the correct applications and are not a persistent installation history.

Metadata extraction reads selected byte ranges of package-manager-derived APK
paths. It temporarily stages only the manifest and resource table locally for
AAPT2, then removes that file. A crash can leave a `resource-*.tmp` file in the
metadata directory; it is private local output. Full game APKs are not cached.

The metadata cache is `env/cache/app-metadata` in development and
`%LOCALAPPDATA%/dev.questmanager.desktop/app-metadata` in release builds. Cache
keys hash the transport, package/version, APK paths, sizes and modification
times; hashed filenames do not anonymize the stored names or artwork. Older
entries are evicted when the entry or byte limit is reached. Application
uninstallation does not immediately remove its cached entry.

**Clear cached artwork** deletes cached JSON and pauses background enrichment;
already displayed information remains in memory. **Load app details**, refresh,
or reopening the app resumes collection. Opening an individual detail view also
reads and can cache that application. Cache content stays on this computer and
must not be copied into the repository or shared as a diagnostic report.

## APK Preparation Storage

Preparation uses `env/local-data/apk-install` in development and
`%LOCALAPPDATA%/dev.questmanager.desktop/apk-install` in release builds. Its
`preview` folder holds temporary manifest/resources, `staging` holds complete
task-owned APK copies and decoded resources, and `signing-keys` holds a stable
key/password pair per package (directory names hash the package ID).

Original APKs are not overwritten. Temporary files are removed on normal
completion/failure; cleanup failures report their paths. Crashes may leave
private staging files. Remove only known inactive task directories, not keys.

Installing with OBB files also stages a complete APK copy here to bind the
destination package to the installed bytes. With modification disabled, this
copy retains the original signature and creates no signing keys. Local OBBs are
read directly and hashed in bounded chunks, without a persistent local copy or
hash cache. Headset transfers use task-owned `.partial` files in the package's
OBB directory. Cleanup failures report remaining paths; completed files are not
rolled back if a later transfer fails.

Signing keys persist across launches, uninstall operations and artwork clearing.
They allow compatible updates signed by this installation. Back up the entire
`signing-keys` folder privately, including password files, and restore it to the
same location before using a replacement installation. Do not share or commit
these files; possession allows signing as that local identity. Windows ACLs limit
new key directories to the current user; this does not protect against code
already running as that user. Backups need equivalent protection. Losing keys can
prevent updates without a reinstall; uninstalling may lose game data. There is
no automatic key synchronization, import wizard or cloud backup.

## Network and Tooling

About is local content. Selecting its source repository link explicitly opens
the public GitHub repository in the user's browser, where browser/GitHub network
and privacy behavior apply. The link contains no device identifiers or paths.

The app starts a local ADB client and reuses the default ADB server. The server
can communicate over USB or Wi-Fi. Explicit setup uses USB, pairing codes or
locally generated debugging QR codes. USB checks existing authorization first;
when needed it enables system TLS debugging for the current network and pairs.
ADB mDNS discovers the exact generated service and paired device's connection
service. ADB can reconnect trusted wireless transports after wake; there is no subnet scanning or app recovery
that pairs devices or enables wireless debugging.

Pairing codes and QR secrets use private subprocess stdin, not process arguments,
and are not included in returned pairing diagnostics. QR images are generated
locally without external image services. The app does not save addresses or codes.
ADB and the headset manage their own persistent trust records and authorization
keys. Closing the dialog/app does not revoke pairing or disable the headset's
wireless listener. Manage trust and wireless debugging in the headset settings.
Cancelling or expiring a QR session stops the local worker/client, but a pairing
already submitted to the shared ADB server may still complete.

Dependency installation contacts the package and tool distributors referenced
by the version manifests and lockfiles. Optional installer builds also fetch
Tauri packaging tools. Those downloads are distinct from application runtime
device operations.

Project tool locations in `env` do not isolate ADB authorization keys or all
system caches. Quest Manager does not override ADB's standard key storage or
manage revocation of the computer's device authorization.

USB setup stages a first-party DEX helper, with no credentials embedded in its
bytes, under a random owned `/data/local/tmp/quest-manager-wireless-*` directory.
It runs as the already authorized shell user; no APK is installed. Cleanup stops
an operation-requested pairing listener and removes the helper on normal exit.
A started helper has an on-device deadline; abrupt exits or lost USB can still
leave temporary files. Existing trust and enabled wireless debugging are not
rolled back. The app never reads or exports the headset's pairing-key files.

## Optional App Downloads

Opening Lightning Launcher setup or refreshing its versions contacts the author's
public GitHub repository. Version matching reads public source at a release tag;
APK downloads occur only after **Download & install**. Requests contain fixed
repository paths, selected public release information and an application user
agent. They do not include device identifiers, app inventories, local paths or
credentials. GitHub/CDN providers receive ordinary network information such as
the client IP. HTTPS is handled by pinned reqwest 0.13.5 with rustls and platform
certificate verification; the webview is not given general network access.

Release data is cached in process memory for ten minutes, bounded to 500 releases;
recommendations last until refresh/process exit. Downloads use task-owned folders
under the existing APK installation storage's `downloads` directory. They are
removed after completion or failure; an interrupted process may leave files here.
These folders contain public APK bytes, not signing keys, and are not bundled.

The **Show install suggestions** boolean is persisted in WebView2 localStorage
under `quest-manager.show-lightning-suggestion`. It contains no headset identity
and survives restarts and artwork-cache clearing. Preview uses a separate key.
Navigator activation state is read from the active Android user's Accessibility
settings and retained only in the open setup dialog. No activation or permission
settings are written. The project link opens the fixed upstream GitHub page in
the default browser.

## Diagnostics and Sharing

Errors and task messages can include actual device paths, computer paths,
package names, and command output. They are not automatically redacted.
Screenshots, terminal transcripts, crash dumps, and copied support messages
inherit that sensitivity.

`env/installed-versions.json` records local tool and platform details, including
ADB version output that can contain an executable path. `run-build.ps1` copies
it to `release/build-environment.json`. Both are local records excluded from
Git; review them before any distribution. Dependency lockfiles and the version
manifest are the public inputs for reproducing the environment.

Use fictional preview data and synthetic fixtures in public files. Keep live
device validation records, inventories, private keys, and personal paths out
of the repository. Do not recreate `VALIDATION.md`. Aggregate check outcomes
can be reported in the task conversation without storing device observations.

See [Secure development](docs/development/security.md) for review rules and
[Troubleshooting](docs/operations/troubleshooting.md) for local diagnosis.
