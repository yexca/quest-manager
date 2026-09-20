# Dependencies and Reproducibility

The project records toolchain inputs and locks dependency resolution. It does
not promise byte-identical binaries across Windows SDKs, linkers, WebView2
versions, packaging metadata, or code-signing environments.

## Version Sources

| Input | Authoritative files |
| --- | --- |
| Project Node and npm | [toolchain.versions.json](../../toolchain.versions.json) includes the official Windows ZIP URL/SHA-256; [package.json](../../package.json), [.node-version](../../.node-version) also declare versions |
| System compatibility policy | `systemPrerequisites` in `toolchain.versions.json`; detection and consent in [SystemPrerequisites.ps1](../../scripts/SystemPrerequisites.ps1) |
| Rust toolchain and target | [rust-toolchain.toml](../../rust-toolchain.toml), `toolchain.versions.json` |
| Rust minimum language version and crate dependencies | [Cargo.toml](../../src-tauri/Cargo.toml) |
| rustup, Android Platform-Tools, AAPT2 and APK preparation archives | Fixed URLs and SHA-256 values in `toolchain.versions.json` |
| JavaScript dependencies | Exact versions in `package.json`, resolved tree in [package-lock.json](../../package-lock.json) |
| Rust dependencies | Exact direct versions in `Cargo.toml`, resolved tree in [Cargo.lock](../../src-tauri/Cargo.lock) |
| Tauri packaging tools | Resolution and checksums supplied by the pinned Tauri CLI |
| Offline portable WebView2 | [packaging/webview2.json](../../packaging/webview2.json), official fixed CAB URL/SHA-256 plus Microsoft signature validation |

The exact current version list is also summarized in the root
[README](../../README.md). Update all corresponding declarations together.
Tauri Rust, JavaScript API, CLI, and plugins have independent version numbers;
do not force them to share a patch number.

## Project Environment

| Location | Contents |
| --- | --- |
| `env/node/node-v<version>-win-x64` | Official Node executable and bundled npm; also selected for child processes through the process-local PATH |
| `env/cargo` | Cargo/rustup launchers and crate downloads/source cache (`CARGO_HOME`) |
| `env/rustup` | Rust toolchain, standard library, rustfmt, Clippy (`RUSTUP_HOME`) |
| `env/target` | Rust build/test output (`CARGO_TARGET_DIR`) |
| `env/target/.tauri` | Optional packaging tools cached by Tauri |
| `env/node_modules` | Installed npm packages |
| `env/npm-cache` | npm download cache |
| `env/platform-tools` | Official ADB executable, DLLs, and licenses |
| `env/apk-tools` | Pinned Apktool, official apksigner/zipalign, notices and private Temurin JRE |
| `env/local-data/apk-install` | Private development APK staging and persistent local signing keys; never distribute |
| `env/aapt2` | AAPT2 9.4.0-15978811 executable (2.20-15978811) and notices, extracted from the official Maven Windows JAR; no Java runtime needed |
| `env/cache/app-metadata` | Private development artwork/metadata cache, not dependency input |
| `env/downloads` | Downloaded tool archives verified by checksum |
| `env/webview2` | Extracted fixed WebView2 for the optional offline portable package |
| Root `node_modules` | Windows junction pointing to `env/node_modules` |

The scripts invoke project-local Node/npm explicitly and prepend their directory
to the current process PATH. A missing local Node/npm fails with setup guidance;
system Node is not a fallback and is never replaced. Source
`scripts/Environment.ps1` before direct Node/npm commands in a new shell.
Visual Studio C++ tools, Windows SDK and WebView2 remain system installations.
Environment changes apply only to the current PowerShell process and children.
ADB's own authorization storage is outside this dependency layout.

Rust uses Cargo to resolve and build crates; rustup selects and installs the
compiler/toolchain. Cargo's source cache and compiled artifacts are separate.
There is no Python-style runtime activation needed for the compiled app, but
direct development commands must select the project toolchain environment.

## Normal Installation

`run-install.ps1` checks system prerequisites, verifies the downloaded Node ZIP
and its bundled npm version, verifies rustup, ADB, AAPT2, Apktool, JRE and Build Tools archives, installs the
pinned Rust components, and uses
`npm ci` under `env` plus `cargo fetch --locked`. It refuses an unexpected root
`node_modules` directory instead of deleting it. Resolve that conflict locally
before retrying, preserving any user data and verifying junction targets.

Missing/incompatible system prerequisites are offered for installation or repair
only after explicit consent, defaulting to No. `-CheckOnly` performs only the
read-only system check; `-NonInteractive` never prompts or changes system
components, and fails if they need attention. Both return a failure on missing
prerequisites. See [Getting started](../getting-started.md) for minimum versions,
repair scope, elevation and restart behavior.

System repairs use Microsoft's current VS 2022 and WebView2 installer endpoints,
with valid Microsoft Authenticode signatures required. Unlike pinned project
archives, these system installers are not immutable or SHA-256 pinned. Their
payloads and update services live in system-managed locations. This distinction
is intentional; recorded local versions do not make system updates reproducible.
Installer launchers downloaded by the bootstrap stay in
`env/downloads/system-installers`. Node is not included in app release output.

The default path does not regenerate lockfiles. Build and development commands
also use Cargo's locked mode. Avoid routine root `npm install` or global Cargo
updates as substitutes for the bootstrap.

## Intentional Updates

1. Change the relevant exact version declarations together. For tool archives,
   obtain and verify the corresponding official URL and checksum.
2. Run `run-install.ps1 -RefreshLocks` only as part of the intended update.
   It regenerates both lockfiles and reinstalls the project environment.
3. Review the lockfile diff, install sources, and any dependency script changes.
4. Run the checks appropriate to the update, normally `run-test.ps1`; build the
   relevant distributable when toolchain or packaging behavior changed.
5. Update the public version summary and document material compatibility changes.

The fixture APK's generation tools are described separately in
[tests/fixtures/README.md](../../tests/fixtures/README.md). External Android
SDK and Java tools are not required to build the application or use that
checked-in fixture. The project-managed AAPT2 executable is a runtime/resource-parser test dependency.
APK preparation additionally bundles Apktool 3.0.3, Build Tools 37.0.0 signing and
alignment tools, and Temurin JRE 21.0.12.1+1, fixed by URL/SHA-256. Archives expand
under `env`; the bootstrap copies runtime contents into `env/apk-tools`. This
includes Java license/notices and Android NOTICE.txt; Apktool retains its JAR
notices. No external Android SDK or global Java is searched. Rust XML/PNG/random
crates are pinned to 0.42.0/0.18.1/0.3.4 respectively.
Local QR encoding uses exact `qrcode` 0.14.1 with default features disabled.
The app builds SVG geometry from its matrix; no remote QR service or image
dependency is used.

The first-party USB wireless helper is authored as Smali in
[device-tools/wireless](../../device-tools/wireless/README.md). `src-tauri/build.rs`
assembles it with the existing pinned Apktool and project JRE, then embeds the
DEX bytes in the Rust executable. Build output stays under Cargo's `OUT_DIR`
inside `env/target`. No extra compiler, Android Studio, SDK or global Java is required.

## Local Records

GitHub Actions uses the same local toolchain and lockfiles on `windows-2022`.
It checks the runner's VS 2022 and SDK rather than installing different compiler
versions. Its explicit disposable-runner setup may install a compatible signed
WebView2 Evergreen runtime; local noninteractive setup still refuses system
changes. Actions are pinned to full commit SHAs. See [CI and release](ci-release.md).

`env/installed-versions.json` records actual tool/platform versions and lockfile
hashes. Builds copy it to `release/build-environment.json`. These records may
contain machine paths and belong to ignored local output. Commit the version
declarations and lockfiles, not installation records. See [Privacy](../../PRIVACY.md).


## Optional Download Client

Runtime GitHub downloads use exact reqwest 0.13.5 with rustls and platform certificate
verification. Cargo.lock pins its dependencies. This client is separate from the
ADB client and does not alter the webview network capabilities.
