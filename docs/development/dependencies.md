# Dependencies and Reproducibility

The project records toolchain inputs and locks dependency resolution. It does
not promise byte-identical binaries across Windows SDKs, linkers, WebView2
versions, packaging metadata, or code-signing environments.

## Version Sources

| Input | Authoritative files |
| --- | --- |
| System Node and npm | [toolchain.versions.json](../../toolchain.versions.json), [package.json](../../package.json), [.node-version](../../.node-version) for Node |
| Rust toolchain and target | [rust-toolchain.toml](../../rust-toolchain.toml), `toolchain.versions.json` |
| Rust minimum language version and crate dependencies | [Cargo.toml](../../src-tauri/Cargo.toml) |
| rustup and Android Platform-Tools archives | Fixed URLs and SHA-256 values in `toolchain.versions.json` |
| JavaScript dependencies | Exact versions in `package.json`, resolved tree in [package-lock.json](../../package-lock.json) |
| Rust dependencies | Exact direct versions in `Cargo.toml`, resolved tree in [Cargo.lock](../../src-tauri/Cargo.lock) |
| Tauri packaging tools | Resolution and checksums supplied by the pinned Tauri CLI |

The exact current version list is also summarized in the root
[README](../../README.md). Update all corresponding declarations together.
Tauri Rust, JavaScript API, CLI, and plugins have independent version numbers;
do not force them to share a patch number.

## Project Environment

| Location | Contents |
| --- | --- |
| `env/cargo` | Cargo/rustup launchers and crate downloads/source cache (`CARGO_HOME`) |
| `env/rustup` | Rust toolchain, standard library, rustfmt, Clippy (`RUSTUP_HOME`) |
| `env/target` | Rust build/test output (`CARGO_TARGET_DIR`) |
| `env/target/.tauri` | Optional packaging tools cached by Tauri |
| `env/node_modules` | Installed npm packages |
| `env/npm-cache` | npm download cache |
| `env/platform-tools` | Official ADB executable, DLLs, and licenses |
| `env/downloads` | Downloaded tool archives verified by checksum |
| Root `node_modules` | Windows junction pointing to `env/node_modules` |

The scripts reuse system Node and npm. Visual Studio C++ tools, Windows SDK,
and WebView2 are also system prerequisites, not project-local installations.
Environment changes apply only to the current PowerShell process and children.
ADB's own authorization storage is outside this dependency layout.

Rust uses Cargo to resolve and build crates; rustup selects and installs the
compiler/toolchain. Cargo's source cache and compiled artifacts are separate.
There is no Python-style runtime activation needed for the compiled app, but
direct development commands must select the project toolchain environment.

## Normal Installation

`run-install.ps1` checks the required Node/npm and system prerequisites, verifies
downloaded rustup/ADB archives, installs the pinned Rust components, and uses
`npm ci` under `env` plus `cargo fetch --locked`. It refuses an unexpected root
`node_modules` directory instead of deleting it. Resolve that conflict locally
before retrying, preserving any user data and verifying junction targets.

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
[tests/fixtures/README.md](../../tests/fixtures/README.md); Java/Android build
tools are not required to build the application or use that checked-in fixture.

## Local Records

`env/installed-versions.json` records actual tool/platform versions and lockfile
hashes. Builds copy it to `release/build-environment.json`. These records may
contain machine paths and belong to ignored local output. Commit the version
declarations and lockfiles, not installation records. See [Privacy](../../PRIVACY.md).
