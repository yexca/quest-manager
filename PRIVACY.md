# Privacy and Data Handling

Quest Manager manages data between the local computer and a headset through
ADB. The application code has no analytics, telemetry, cloud account, or cloud
sync integration. This describes the application, not the independent behavior
of Windows, WebView2, ADB, or the headset operating system.

## Data Used at Runtime

| Data | Purpose | App-managed retention |
| --- | --- | --- |
| Device serials, transport addresses, model, Android version | Discovery, selection, and command targeting | Current process and UI state |
| Battery and shared-storage totals | Overview | Current UI state |
| Package names, versions, and installed APK paths | Application listing, details, and export | Current UI state and task details |
| Application labels, raster icons, signing fingerprints, VR declarations | Identify and inspect applications | Local metadata cache, bounded to 512 entries / 64 MiB |
| Install/update times, permissions, installer, SDK/ABI, app state | Application details | Current UI state; queried again on refresh |
| Shared filenames, paths, sizes, and timestamps | File browsing and transfer | Current UI state and task details |
| Selected computer paths and ADB output | Task execution and error feedback | Current process and UI state |
| Local APKs and selected icon images | Optional rebuilding, signing and installation | Task-owned temporary copies; cleanup attempted on completion/failure |
| Per-package signing keys and password files | Allow subsequent locally signed updates | Persistent private local storage; user-managed backup/removal |
| Uploaded, downloaded, or exported files | Explicit user operations | Files remain at their destinations |

There is no app database or persisted task history. This does not mean an
operation is undone when the app closes. Completed installs, uninstalls,
renames, deletions, and transfers remain effective. Interrupted operations can
leave temporary files. WebView2 and ADB may keep their own runtime data outside
the application's state.

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
can communicate with a device over USB or an already established Wi-Fi
connection. Quest Manager does not set up wireless debugging itself.

Dependency installation contacts the package and tool distributors referenced
by the version manifests and lockfiles. Optional installer builds also fetch
Tauri packaging tools. Those downloads are distinct from application runtime
device operations.

Project tool locations in `env` do not isolate ADB authorization keys or all
system caches. Quest Manager does not override ADB's standard key storage or
manage revocation of the computer's device authorization.

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
