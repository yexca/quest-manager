# ADR-0006: OBB Files as Part of an APK Installation

Status: Accepted

## Context

Some applications require several expansion files with standard or custom OBB
names. Queuing unrelated file uploads cannot express the dependency on a
successful APK install or accurately report partial completion.

## Decision

An optional OBB selection belongs to one APK and one captured transport. One
global-queue task covers preflight, APK installation, OBB transfers, verification
and cleanup. A package name decoded from the staged APK determines the remote
directory; an unreadable package disables the feature in both UI and backend.
The APK is staged unchanged unless preparation/re-signing was explicitly enabled.

Preserve OBB filenames and bytes. Refuse differing existing files and unsafe
destinations. Reuse identical existing files only after SHA-256 verification.
Publish new files from task-owned temporary siblings using a no-clobber move.
Keep the existing rule that a running install cannot be cancelled.

## Consequences

APK installation and multiple remote files are not an atomic transaction.
Failures after APK success explicitly report incomplete data and retain completed
work. An `apkInstalled` snapshot flag allows correct app refresh on failure;
`includesObb` also refreshes Files. Manual retries recheck existing content,
without persisted task state, automatic retry, overwriting or auto-uninstall.
Large files require a local hash pass and remote verification; staging the APK
requires additional local disk space. Unknown-package APK-only installation
remains available. No archive extraction, split installation or OBB editing is added.
