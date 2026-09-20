# Portable Comparison Packages

Run `run-build.ps1 -Portable Both` to build both variants, or use `Online` /
`Offline` to select one. This is separate from `-Installer`. Each build uses a
new ignored `release/portable-*` folder with complete extracted folders, ZIPs,
SHA-256 sidecars and an aggregate size comparison. The local command does not
upload. [GitHub Actions](ci-release.md) uploads only the ZIPs and SHA-256 sidecars.

Launch with `Start-QuestManager.cmd`; directly running the app bypasses runtime
selection/checks. The package baseline is Windows 10 2004+ / Windows 11 x64.
Online packages reuse compatible registered WebView2 and ask before downloading
and running the Microsoft-signed Evergreen installer. Defaults and redirected
input never authorize installation. Offline packages include the SHA-256-pinned,
Microsoft-signed CAB from [webview2.json](../../packaging/webview2.json), extract
it under `webview2`, and select that runtime for the app process. On Windows 10
the launcher grants Microsoft's required AppContainer read/execute permissions
only to this runtime directory. Extract into writable local NTFS storage. Fixed
runtime updates need a new package; online app features still need internet.

Portable builds remap Rust source paths and MSVC native `__FILE__` paths before
compilation. They reject custom Rust flags and scan staged text/binary bytes for
the builder's profile and checkout paths in UTF-8/UTF-16, with either path
separator. This also covers hosted-runner workspaces outside the user profile.
Staging copies only the app, tools, launcher and
notices, omitting local environment records and user data. These checks do not
replace full privacy and third-party distribution review. MSVC mapping uses
`/experimental:deterministic` and `/pathmap`; incompatible toolsets must fail
validation rather than shipping an unsanitized executable.

The variants share one executable. Users do not need Node/Rust/SDK/global Java.
The included Java carries its VC runtime DLLs. USB access can require Meta's ADB
driver and headset authorization. App data remains in Windows user data folders.
These are unsigned comparison packages. Clean-system startup, native tools and
device access are separate validation steps.

For a read-only check, run this from the extracted directory:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File portable/Launch.ps1 -CheckOnly
```

`-NonInteractive` refuses needed system installation. Missing-runtime installation
and Windows 10 ACL handling need disposable-machine tests; do not uninstall a
user's runtime. Launcher regressions run through `run-test.ps1` or directly:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tests/bootstrap/portable.test.ps1
```

Test extracted ZIPs in a path with spaces, verify WebView2 processes use the
intended runtime, and run bundled native tools. A host launch is not a clean
Windows installation test. Follow [Commit and release](commit-and-release.md)
and [third-party distribution review](../../THIRD_PARTY.md) before publication.
