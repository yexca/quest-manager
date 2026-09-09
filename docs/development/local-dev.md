# Local Development

Run commands from the repository root in PowerShell. The project currently
targets Windows x64; native system prerequisites are described in
[Getting started](../getting-started.md).

## Canonical Entry Points

| Command | Purpose |
| --- | --- |
| `.\run-install.ps1` | Check system prerequisites, install pinned project tools/packages, and write the local installation record |
| `.\run-dev.ps1` | Check the installed environment and start Tauri with Vite |
| `.\run-test.ps1` | Frontend build/typecheck, Rust format check, Clippy, and default Cargo tests |
| `.\run-build.ps1` | Build the release executable and portable ADB/AAPT2 directories |
| `.\run-build.ps1 -Installer` | Also build the NSIS installer |
| `.\run-install.ps1 -RefreshLocks` | Intentionally regenerate dependency locks before installation |

The scripts source [Environment.ps1](../../scripts/Environment.ps1), which sets
tool locations for the current process and child processes, and moves to the
repository root. The installed-environment check verifies Node/npm versions,
manifest consistency, and recorded lockfile hashes. It does not replace code
validation or prove that every dependency binary is unmodified.

## Targeted Commands

After installation, a frontend-only change can use:

```powershell
npm run typecheck
npm run build
```

For Cargo work, load the local toolchain first:

```powershell
. .\scripts\Environment.ps1
Assert-QuestInstalled
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

`cargo fmt` formats files; the normal test entry point uses `--check` and does
not rewrite them. Do not accidentally invoke a globally installed Rust version
or redirect build output outside `env` when reproducing project behavior.

## UI Preview

```powershell
npm run dev
```

Open [the explicit preview](http://127.0.0.1:1420/?preview=1) for layout and
interaction review with fictional data. A browser cannot replace Tauri's native
IPC or file dialogs. Keep the preview badge visible in documentation screenshots.

The dev server binds to loopback and port 1420 is strict. Use one instance at a
time; a standalone preview conflicts with the Vite process started by Tauri.

## Editing and Handoff

Follow the existing layers in [Architecture](../architecture/index.md). For an
IPC change, update the Rust contract, frontend types, wrapper, UI, and preview
as needed. For an interface change, follow [Design](../../DESIGN.md).

Choose checks from [Testing](testing.md), update the corresponding area docs,
and report material limitations. Respect the user's requested commit scope;
editing a file does not require creating a commit or a release.

See [Dependencies](dependencies.md) before changing manifests or lockfiles and
[Troubleshooting](../operations/troubleshooting.md) for setup failures.
