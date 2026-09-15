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
without additional dependencies: `node --test tests/frontend/taskState.test.ts tests/frontend/installState.test.ts tests/frontend/applicationState.test.ts`.
They cover stream registration/disposal, overlapping events and snapshots,
revision ordering, clearing, device changes and batched refresh policy. Rust
tests cover the authoritative exit guard, terminal-only clearing and revisions.
There is no browser E2E or documentation-test runner. These logic tests do not
replace rendered React or native-window inspection; `tsc` alone is not behavior coverage.

Install review tests cover lossless numeric version ordering, unknown/error
states, exact package matching, system apps, obsolete query disposal, retry and
duplicate removal. Rust tests check complete manifest version codes and the
bundled AAPT2 output. Preview APKs demonstrate newer, same, older, absent and
unknown-version system packages, plus duplicate package selection. Inspect the
inline comparison, retry, removal and re-signing warning at supported widths;
preview installation remains disabled.

OBB logic tests cover unknown/split-package disabling, repeated selections,
custom filenames, duplicate conflicts, attachment limits and stamped requests.
Partial-install task tests check app/files refresh without converting failure
into success. The normal Rust suite compiles the host-only
[mock ADB fixture](../../tests/fixtures/mock-adb.rs) with the project Rust toolchain
and runs composite installs against an isolated directory under `env/test-artifacts`.
It checks multi-file order, reuse, conflicts, changed sources, unreadable/wrong
packages, redirected directories, APK failure, interrupted pushes, checksum
mismatch, publication collisions and cleanup reporting. This executable cannot
contact a headset; its simulated checksum responses do not establish device
compatibility. Real local hashing and APK parsing run in the normal test process.
Preview attachment selection/removal and switching at both supported widths;
the unreadable APK fixture must keep OBB controls disabled. Native file picker
and drag/drop integration require separate desktop validation.

Application cache regressions check that deleting one package and toggling
system visibility do not reread unrelated metadata. They also cover same-version
replacement, APK path/installer changes, obsolete pending results, detail request
deduplication, explicit refresh, source inference and unknown source fallback.
Preview the source filter together with search/system visibility, cached icons,
detail dialogs, empty matches, and narrow layouts. Device source classification
remains best effort; fixture tests do not verify every Horizon OS installer.

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

Overview preview defaults to a fictional Quest 3. Use the device selector to
check Quest 3, Quest 3S, Quest 2, and the generic headset illustration at both
1280 by 850 and 1000 by 680. Confirm that the model name and illustration follow
selection, USB/Wi-Fi changes preserve the model, and preview writes stay disabled.

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


## Optional Launcher Setup

Routine frontend tests include `tests/frontend/lightningState.test.ts` for absent,
unknown, dismissed and installed-edition states, explicit addon matching and
partial-setup refresh, installed-version normalization, edition isolation, and
service choice invalidation when changing targets or refreshing. Rust tests cover fixed-host/asset checks, source matching,
streamed size/hash validation, file collision refusal and component ordering using
the host-only mock ADB. These tests cannot contact a headset.

An opt-in network-only smoke check exercises real release discovery, matching,
download and original signature verification; it creates temporary local files,
cleans them up, and never invokes ADB:

```powershell
. .\scripts\Environment.ps1
cargo test --locked --manifest-path src-tauri/Cargo.toml github_release_download_and_signature_smoke -- --ignored
```

Inspect the suggestion, dismissal/restoration and setup dialog at both supported
sizes. Changing Launcher versions must update the recommended service. Other
versions require an explicit choice and remain labeled unverified; preview install
stays disabled. Native IPC and real installation require separate device testing.
Use `?preview=1&lightning=installed` for fictional older installations, or
`?preview=1&lightning=current` for fictional current installations. In manual
setup, check the installed labels in both selectors, notices when selecting those
versions, and the service recommendation comparison while its checkbox is off.
The default `?preview=1` keeps the absent-installation scenario.
