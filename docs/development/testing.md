# Testing

Choose checks for the behavior being changed. Tests should protect a user
workflow, boundary, state transition, or regression; avoid adding tests that
only mirror the implementation or prove that documentation text exists.

## Routine Checks

```powershell
.\run-test.ps1
```

The script checks the installed environment, builds/typechecks the frontend,
runs Node's built-in frontend logic tests, `cargo fmt --check`, Clippy for all targets with warnings denied, and
runs locked Cargo tests. It does not enable ignored device integration tests.

The unit tests cover remote path boundaries, shell quoting, directory parsing,
Windows filenames, device/storage parsing, unsafe task requests, and progress
parsing. Their sources are the inline test modules in
[adb.rs](../../src-tauri/src/adb.rs) and [tasks.rs](../../src-tauri/src/tasks.rs).
Metadata coverage in [metadata.rs](../../src-tauri/src/metadata.rs) and
[apk.rs](../../src-tauri/src/apk.rs) checks Android user isolation, absent fields,
cache invalidation, resource size limits, image-type filtering, and real AAPT2
label/signing parsing against the existing inert fixture. Test output remains
synthetic; the fixture is not installed for these routine tests.

Frontend logic regressions use the pinned system Node's built-in test runner,
without additional dependencies: `node --test tests/frontend/taskState.test.ts`.
They cover stream registration/disposal, overlapping events and snapshots,
revision ordering, clearing, device changes and batched refresh policy. Rust
tests cover the authoritative exit guard, terminal-only clearing and revisions.
There is no browser E2E or documentation-test runner. These logic tests do not
replace rendered React or native-window inspection; `tsc` alone is not behavior coverage.

| Change | Appropriate starting point |
| --- | --- |
| Documentation | Resolve relative links/anchors, verify commands against source, review facts, run `git diff --check` |
| Frontend types or UI | `npm run build`; inspect affected behavior in explicit preview, and use desktop checks for native integration |
| Rust, task, or IPC behavior | `run-test.ps1` and focused regression coverage |
| ADB compatibility | Relevant opt-in device test within the authorized scope |
| Bootstrap/toolchain | Install with the intended manifests, then routine checks |
| Resource or installer layout | Relevant `run-build.ps1` mode and artifact inspection |

For docs-only work, do not install dependencies, rebuild the application, or
collect device data merely to validate Markdown. `git diff` excludes untracked
files, so include new files in link and privacy review separately.

## Preview Inspection

After installation, start `npm run dev` and open
[preview](http://127.0.0.1:1420/?preview=1). Check the changed page at supported
desktop widths, keyboard focus, dialogs, long package/file names, errors, and
unknown-value states as relevant. The preview includes fixed sample task states
but does not execute tasks or simulate every error. It cannot validate ADB,
native pickers, or real installs. Check target labels, disabled preview task
actions, and queue dialog keyboard navigation. Native close interception still
needs a desktop smoke check with authorized work; do not start device tasks
solely for a preview review.

For About, check sidebar selection, disconnected access, narrow desktop layout,
manifest version values, the repository link and expandable license text.
The desktop repository action needs a Windows browser smoke check; it does not
require a headset or enable device tests.

## Local APK Preparation

Routine tests also use the bundled tools to edit/sign an inert local fixture with
synthetic compressed game data, Unicode/quoted labels and a generated PNG. They
verify payload bytes, output appearance, verity absence, stable key reuse and
source-change rejection, then remove test keys and working copies. No device is
involved. Invalid icon/options and manifest alias/escaping cases are covered.

To exercise a selected local APK without installing it:

```powershell
. .\scripts\Environment.ps1
$env:QUEST_TEST_APK = 'C:\Example\game.apk'
# Optional: include name/icon rebuilding rather than only compatibility signing.
$env:QUEST_TEST_APK_EDIT = '1'
cargo test --locked --manifest-path src-tauri/Cargo.toml local_selected_apk_preparation -- --ignored --nocapture
```

This test writes only private copies and disposable keys under `env/test-artifacts`
and cleans them up. It requires preparation disk space and does not contact ADB.
Tool errors can contain local paths; never commit transcripts.

The explicit preview's Install APK action opens fictional APK reviews. Exercise
per-file switches, name changes, icon crop/reset, removal and disabled installation
at supported sizes. Native pickers/IPC and Quest launcher appearance still need
separate desktop/device checks.

## Opt-In Device Tests

```powershell
.\run-test.ps1 -Device
.\run-test.ps1 -DeviceWrite
```

`-Device` adds `connected_device_smoke`: discovery, metadata, third-party app
listing, shared directory listing, application details, and rejection of a
private-storage path. It is read-only but its failures can expose diagnostic
device data.

It also runs `connected_device_metadata`, reading the first third-party app on
the selected Quest to exercise APK byte ranges, label/state/size extraction and
cache round trip. Its scratch cache is under ignored `env/test-artifacts` and is
cleared after success. No fixture is installed and no device files are changed.

`-DeviceWrite` adds `connected_device_task_roundtrip` from
[device_tests.rs](../../src-tauri/src/device_tests.rs). It uses a unique
`/sdcard/Download/.quest-manager-test-*` folder and local `env/test-artifacts`,
then exercises transfer round trips, unusual filenames, collision refusal,
rename/delete, and fixture APK install/update/export/uninstall. It also installs
and updates a fixture with a modified name/icon, confirms a conflicting signature
is refused, and exports the surviving app to check its prepared resources.
Disposable signing keys are removed with the test's local scratch folder. The
test refuses to start if the fixture package already exists. This flag does not separately
enable the read-only test; both flags can be supplied when both are needed.

Device tests currently choose the first discovered model containing `Quest` and
its first ready transport. They have no device-selector flag. Establish that
this selects the intended authorized test device before opting in. Do not run
all ignored tests indiscriminately or interpret USB debugging authorization as
unlimited permission to change unrelated user data.

Reuse authorization already established for the current test scope. A routine
documentation or preview task is not a reason to run a device write test.
Cleanup is attempted by the integration test, but interruption or a disconnected
device can leave its scratch data. Inspect the exact failed test's paths locally
before any cleanup; never use a broad wildcard deletion on shared storage.

## Synthetic Fixture Data

Use `DEMO-*`/`EXAMPLE-*` device and transport IDs, `com.example.*` packages, fixed
synthetic timestamps, and RFC documentation addresses such as `192.0.2.10`.
Do not copy a real device listing, application inventory, or personal filename
into a parser test or preview.

The inert `dev.questmanager.verification` APK is the deliberate package fixture.
Its source manifest, purpose, generation tools, and checksum are documented in
[the fixture README](../../tests/fixtures/README.md). `run-test.ps1 -DeviceWrite`
checks its checksum before testing. Preserve this check when replacing the
fixture; the signing private key must remain outside tracked files.

Report commands and aggregate outcomes in the task response. Do not save live
validation records to `VALIDATION.md`, docs, or a new fixture. See
[Privacy](../../PRIVACY.md) and [Secure development](security.md).
