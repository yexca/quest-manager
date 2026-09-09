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
| Shared filenames, paths, sizes, and timestamps | File browsing and transfer | Current UI state and task details |
| Selected computer paths and ADB output | Task execution and error feedback | Current process and UI state |
| Uploaded, downloaded, or exported files | Explicit user operations | Files remain at their destinations |

There is no app database or persisted task history. This does not mean an
operation is undone when the app closes. Completed installs, uninstalls,
renames, deletions, and transfers remain effective. Interrupted operations can
leave temporary files. WebView2 and ADB may keep their own runtime data outside
the application's state.

## Network and Tooling

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
