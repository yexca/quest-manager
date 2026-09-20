# ADR-0009: Project-Local Node and Consented System Setup

Status: Accepted. Supersedes the system Node and prerequisite handling portions
of [ADR-0002](ADR-0002-project-local-toolchains.md).

## Context

A fresh checkout should not depend on a contributor's Node installation or
require manual discovery of missing native prerequisites. Windows build tools
and the shared WebView2 runtime still need system installation and can affect
other applications; storing project tools in `env` does not isolate Windows.

## Decision

Download the pinned official Node Windows x64 ZIP, verify its committed SHA-256,
and extract Node and bundled npm into a versioned directory under `env/node`.
Verify both executable versions. Root scripts invoke those paths and prepend
the directory only to the current process PATH for child tools. Never replace
system Node, fall back to it when local files are absent, or change permanent
environment variables. Keep npm and Cargo lockfiles unchanged during normal setup.

Check VS 2022/MSVC, SDK files and registered WebView2 versions against the policy
in `toolchain.versions.json`. Reuse compatible components. For missing or
incompatible components, explain the installation/update scope and require an
affirmative interactive answer (default No). Use official Microsoft installers,
verify Microsoft Authenticode signatures, wait for completion and recheck.
C++/SDK repair can update one named VS 2022 installation; other major versions
are retained. Request elevation for C++/SDK installers only, not the entire
bootstrap. Do not force applications closed or request an automatic reboot.

Provide read-only `-CheckOnly` and fail-closed `-NonInteractive` modes.
Neither mode authorizes system installation. Keep downloads and local records
under ignored `env`; Microsoft's system payloads/caches remain system-managed.

## Consequences

Developers no longer need system Node/npm or a package manager such as winget.
Direct tool commands in a new shell must load `scripts/Environment.ps1` first.
Pinned project downloads reproduce tool selection. System repair endpoints
serve current VS 2022 servicing and WebView2 Evergreen releases and are signature
verified rather than checksum pinned. They do not provide byte-identical build
reproducibility or freeze future runtime updates.

Host-only bootstrap regression tests mock system installation and must never
elevate or install components. Real installer behavior needs separate validation
on a disposable Windows environment with explicit consent. See
[Dependencies](../development/dependencies.md) and [Testing](../development/testing.md).
