# ADR-0002: Pinned Tools and Project-Local Dependencies

Status: Accepted; system Node and prerequisite handling superseded by
[ADR-0009](ADR-0009-local-node-system-setup.md).

## Context

Contributors need to reproduce toolchain and package selection without relying
on an unspecified global Rust installation. The project should keep downloaded
tools and libraries together while reusing the computer's existing Node setup.

## Decision

Record Node/npm, Rust, target, rustup, and Platform-Tools in explicit version
files. Use exact direct package versions and commit both npm and Cargo lockfiles.
Verify the fixed rustup and ADB archives with SHA-256.

Use root PowerShell scripts to initialize tools, run development and checks,
and build distributables. Place Rust tools, Cargo/npm caches, npm packages, ADB,
and build output under `env`. Provide the root `node_modules` path through a
Windows junction. Set environment variables only for the current process.

Retain system Node/npm at the required versions. Treat Visual Studio C++ tools,
Windows SDK, and WebView2 as checked system prerequisites. Normal setup consumes
lockfiles; intentional maintenance uses `-RefreshLocks`.

## Consequences

A checkout plus version declarations and lockfiles can reproduce dependency
selection. Direct Cargo commands need the project environment, and first setup
requires downloads and substantial local cache space.

This does not freeze every system component or promise byte-identical binaries.
Local installed/build environment records help local diagnosis but can expose
machine details. They remain ignored; only generic version inputs are public.

See [Dependencies](../development/dependencies.md) and [Privacy](../../PRIVACY.md).
