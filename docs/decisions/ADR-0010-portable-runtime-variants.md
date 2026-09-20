# ADR-0010: Online and Offline Portable Runtime Packages

Status: Accepted for local comparison builds; public release review is separate.

## Context

A portable Windows application still needs WebView2. Users need to compare a
smaller package using the shared runtime with a package carrying its own runtime.

## Decision

Keep Tauri and build one executable for both variants. Add a PowerShell launcher:
online mode checks shared WebView2 and requires consent before running Microsoft's
signed installer; offline mode explicitly selects a pinned, bundled fixed runtime.
Neither changes permanent environment variables. Preserve app data locations.
On Windows 10, apply Microsoft's required AppContainer read permissions only to
the fixed runtime. Fail if the offline payload is incomplete instead of silently
using the installed runtime or downloading another one.

Produce separate ZIPs, public manifests and checksums. Exclude local records.
Remap Rust and native compiler source paths before building, then scan staged
binary/text files for the builder's profile. No binary byte-patching is used.

## Consequences

Offline ZIPs are larger and the fixed browser needs maintenance with app updates.
Online installation may contact Microsoft and install shared components outside
the portable directory. These packages remain unsigned and require clean-Windows
tests and a complete third-party distribution review before public publication.
See [Portable packages](../development/portable.md).
