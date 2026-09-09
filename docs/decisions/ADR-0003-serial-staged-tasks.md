# ADR-0003: Serial Task Execution and Staged Transfers

Status: Accepted; documents the current implementation.

## Context

Installation, transfer, rename, and deletion can take time and change user data.
They need visible outcomes without blocking browsing. Concurrent mutations and
partially written final destinations make failures harder to understand.

## Decision

Use one global task semaphore for mutations across all devices. Capture the
selected ADB transport in each request and retain snapshots in process memory.
Send complete snapshot events and provide a query API for initial state.

Upload, download, and APK export use task-owned temporary paths adjacent to
their final destination. Check destination absence before transfer and again
before publication. Attempt cleanup of the owned temporary item on failure.
Report cleanup problems and possible residual paths.

Allow cancellation before execution for every task kind. While running, expose
cancellation for transfers/export; do not advertise interruption of Android
package changes or short filesystem mutations.

## Consequences

The implementation is simple to reason about and keeps task progress available
across page navigation. Reads can proceed independently, but a long transfer
blocks later mutations even when they target another device.

Temporary paths reduce exposure of incomplete final items. They are not a
transaction across concurrent external changes, and cleanup can fail after
disconnection or process exit. Several queued tasks are not one atomic batch.

There is no persisted queue, automatic retry, retargeting, or crash-resume
contract. Adding these would require a new decision covering device identity,
idempotency, ownership of partial files, privacy, and recovery.

See [Runtime workflows](../architecture/workflows.md),
[Data model](../architecture/data-model.md), and
[Product workflows](../product/workflows.md).
