# Agent Guide

Quest Manager is a local Windows desktop app for managing Meta Quest applications
and shared files through the official ADB executable. The stack is Tauri 2,
React, TypeScript, and Rust. Product UI and error messages are English; i18n is
outside the current scope.

Quest 3 is the primary development and usability target. Keep device operations
independent of model names; other headsets can use compatible ADB capabilities.
Model-specific artwork is presentation only, never a device access restriction.

## Start Here

Read these before changing the project:

1. [README](README.md) for setup and supported features.
2. [Documentation map](docs/README.md) for the relevant reading path.
3. [Core boundaries](docs/architecture/core-boundaries.md) for invariants.
4. [Privacy and data handling](PRIVACY.md) before collecting or sharing output.

Then use the smallest relevant set of documents:

| Change | Read |
| --- | --- |
| Device discovery, IPC, or types | [Architecture](docs/architecture/index.md), [data model](docs/architecture/data-model.md) |
| Installation, transfers, or task state | [Architecture workflows](docs/architecture/workflows.md), [product workflows](docs/product/workflows.md) |
| UI or interaction | [Design](DESIGN.md), [product workflows](docs/product/workflows.md) |
| Setup or dependencies | [Local development](docs/development/local-dev.md), [dependency policy](docs/development/dependencies.md) |
| Filesystem, subprocesses, or capabilities | [Secure development](docs/development/security.md), [security model](SECURITY.md) |
| Tests | [Testing](docs/development/testing.md) |
| Commit or packaging | [Commit and release](docs/development/commit-and-release.md) |

## Implementation Boundaries

- Keep physical device identity separate from ADB transport identity. IPC
  `device` arguments contain the selected transport serial. Never retarget an
  existing task when the user changes the selected device or connection.
- Use the project ADB in development and the bundled ADB in production. Reuse
  the default ADB server; do not add automatic `kill-server`, root, reboot,
  pairing, or wireless-debugging setup as connection recovery.
- Keep device queries in `adb.rs`, metadata/cache orchestration in `metadata.rs`,
  APK decoding in `apk.rs`, task orchestration in `tasks.rs`, and explicit
  IPC registration in `lib.rs`. Do not expose an arbitrary command executor to
  the webview.
- General file management stays within `/sdcard`, with
  `/storage/emulated/0` accepted as an alias. Preserve canonical path checks,
  shell quoting, storage-root protections, and Windows filename validation.
- Installed APK export uses package-manager paths. This specific read operation
  does not authorize a general browser for private app storage.
- Keep the single global mutation queue, staged transfers, collision refusal,
  cancellation rules, and cleanup reporting unless a requested feature changes
  those contracts deliberately.
- Update Rust IPC structs, `src/types.ts`, `src/api.ts`, and their consumers
  together. Keep preview data explicitly fictional and preview writes disabled.
- Session/task state is held in memory. App artwork and immutable APK metadata
  have a bounded private local cache; see `PRIVACY.md`. Do not describe task persistence,
  reconnect resume, or automatic retry as implemented features.
- Keep APK resource rebuilding in `apk_edit.rs`, local preparation/signing in
  `apk_install.rs`, and installation orchestration in `tasks.rs`. Preparation is
  opt-in. Preserve original APKs and persistent private per-package signing keys;
  never remove keys during artwork or temporary-file cleanup.

## Development Workflow

Use the root PowerShell entry points:

```powershell
.\run-install.ps1
.\run-dev.ps1
.\run-test.ps1
.\run-build.ps1
```

The dependency policy is part of the project contract. Use the pinned Node/npm
under `env/node`, and keep project tools, npm packages, and caches in `env`.
Do not change permanent environment variables or replace system Node. Load
`scripts/Environment.ps1` before direct Node/npm or Cargo commands. System
C++/SDK/WebView2 installation or updating requires the bootstrap's explicit
user consent; noninteractive setup must fail instead of changing the system.
The CI workflow explicitly authorizes WebView2 setup on disposable GitHub-hosted
Windows runners through `scripts/Initialize-CiRunner.ps1 -InstallWebView2`.
This exception does not apply to local or self-hosted machines.
Use exact dependency versions and retain both lockfiles. `-RefreshLocks` is for
an intentional dependency update, not a routine workaround for setup failures.

For direct Cargo commands, first load the project environment:

```powershell
. .\scripts\Environment.ps1
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

Keep validation proportional to the change. Documentation changes need link,
command, factual, and whitespace checks. UI changes need the frontend build and
relevant preview inspection. Rust or IPC changes normally use `run-test.ps1`.
Frontend logic tests use Node's built-in runner through `run-test.ps1`;
GitHub Actions runs these host checks and builds portable archives. There is no
browser E2E runner. Do not claim rendered UI coverage
from the logic tests or typecheck alone.

Device tests are opt-in. `-Device` reads the connected headset; `-DeviceWrite`
creates device files and temporarily installs a fixture APK. Choose these only
when relevant to the task and within the user's established authorization.
Reuse authorization already given for that scope rather than repeatedly asking.
Follow the target-selection and cleanup notes in [Testing](docs/development/testing.md).

## Data and Handoff

- Use `DEMO-*` or `EXAMPLE-*` identifiers, `com.example.*` package names, and
  documentation addresses such as `192.0.2.10` in authored examples. The inert
  `dev.questmanager.verification` fixture is a deliberate exception.
- Never commit real device identifiers, private endpoints, app inventories,
  personal paths, screenshots of live devices, keys, logs, or validation
  records. Do not recreate `VALIDATION.md` or place device observations in docs.
- Keep `env`, `release`, build outputs, and local environment records ignored.
  Ignore rules do not remove sensitive content from an already tracked file.
- Before a requested commit, review the actual staged paths and contents,
  including binary metadata. Apply the privacy rules in
  [Secure development](docs/development/security.md).
- Use Conventional Commits, for example
  `feat(apps): add application filtering`. Preserve the configured Git identity
  and signing behavior. Respect the user's current commit and publish scope.
- Update the area docs when behavior changes. Add an [ADR](docs/decisions/index.md)
  for a durable boundary change. Record aggregate validation outcomes in the
  task response rather than saving machine-specific reports in the repository.
- Keep `README.md` as the English default and link to the Chinese version in
  `README.zh-CN.md`. Keep shared commands, versions, and feature summaries in sync.

This guide uses the standard `AGENTS.md` filename. Maintain one authoritative
root guide rather than duplicating it in `agent.md`.
