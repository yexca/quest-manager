# Architecture Decision Records

These ADRs explain durable choices already present in the implementation. They
are not device validation records or a claim about a published release.

- [ADR-0001: Tauri desktop with an official ADB backend](ADR-0001-tauri-adb-desktop.md)
- [ADR-0002: Pinned tools and project-local dependencies](ADR-0002-project-local-toolchains.md)
- [ADR-0003: Serial task execution and staged transfers](ADR-0003-serial-staged-tasks.md)

Add an ADR when a change alters a long-lived boundary such as process topology,
device identity, storage scope, task persistence, dependency management, or
packaging. Routine component edits and bug fixes belong in code and area docs.

Use a sequential filename and explain status, context, decision, and
consequences. A proposal must not be described as implemented. When a decision
is superseded, link to the replacement and update the active architecture docs.
Use synthetic examples and omit machine details or testing transcripts.

Related: [Architecture](../architecture/index.md),
[Core boundaries](../architecture/core-boundaries.md), and
[Contributing](../../CONTRIBUTING.md).
