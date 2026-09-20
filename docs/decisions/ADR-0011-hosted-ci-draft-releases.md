# ADR-0011: Hosted Windows CI and Draft Releases

Status: Accepted.

## Context

The project has pinned local setup/test/build entry points and two unsigned
portable variants. Builds should be repeatable from a Git checkout without
depending on a developer's installed Node, Rust, Java or private device data.
Binary-license review and clean Windows/device validation are still distinct
from host tests and successful compilation.

## Decision

Run host tests and an online portable build on disposable `windows-2022`
GitHub runners for pull requests and `main`. Version tags run the same tests
and build both portable variants. Check every application version location,
tag/commit identity, clean source, package resources and ZIP checksums.

Use the existing local bootstrap and exact dependency locks. Explicitly permit
the CI runner preparation entry point to install Microsoft's signature-verified
WebView2 runtime on GitHub-hosted Windows only. This narrowly extends
[ADR-0009](ADR-0009-local-node-system-setup.md); local noninteractive setup still
fails without changing system components. Do not install/upgrade VS or SDK in CI.

Pin Actions by full commit SHA, avoid shared caches initially, and give build
jobs read-only repository permission. A separate job with release write access
downloads only that run's ZIPs/sidecars and creates or updates a draft. It never
executes the packaged files and refuses to replace published-release assets.
Public release publication stays a maintainer action.

## Consequences

Fresh installs cost more time and disk than cached builds. Hosted Windows images
remain mutable, so this provides a checked build procedure, not guaranteed
byte-identical binaries. Image/tool download changes can correctly fail setup.
Unsigned packages, hardware tests and distribution-license review remain visible
release concerns rather than being inferred from a green build.
