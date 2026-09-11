# Quest Manager Documentation

This documentation explains the current product, its implementation boundaries,
and the workflows for changing it. Start with the section matching the task;
source files and version manifests remain authoritative when details change.

## Start Here

- [Overview](overview.md): goals, implemented features, and current limits.
- [Getting started](getting-started.md): installation, desktop use, and preview.
- [Agent guide](../AGENTS.md): reading paths and repository rules.
- [Contributing](../CONTRIBUTING.md): issues, pull requests and validation.
- [Security reporting](../SECURITY.md): private vulnerability reports and support scope.

## By Area

| Area | Contents |
| --- | --- |
| [Architecture](architecture/index.md) | Source map, runtime layers, boundaries, IPC data, task execution |
| [Product](product/index.md) | User-visible device, application, file, and queue behavior |
| [Local development](development/local-dev.md) | Root scripts, direct commands, and edit workflow |
| [Dependencies](development/dependencies.md) | Pinned versions, project-local tools, and intentional updates |
| [Testing](development/testing.md) | Routine checks, preview, synthetic fixtures, and opt-in device tests |
| [Secure development](development/security.md) | Filesystem and subprocess rules, privacy review |
| [Commit and release](development/commit-and-release.md) | Conventional Commits, version locations, and packaging |
| [Runtime configuration](operations/configuration.md) | ADB discovery, runtime paths, and development server |
| [Troubleshooting](operations/troubleshooting.md) | Setup, connection, installation, and transfer failures |
| [Decisions](decisions/index.md) | Reasons for durable implementation choices |

## Reading Paths

For a first code change, read [Architecture](architecture/index.md),
[Core boundaries](architecture/core-boundaries.md), and
[Testing](development/testing.md).

For UI work, read [Design](../DESIGN.md),
[Product workflows](product/workflows.md), and the frontend section of
[Architecture](architecture/index.md).

For device or file operations, read [Data model](architecture/data-model.md),
[Architecture workflows](architecture/workflows.md), and
[Secure development](development/security.md).

For setup or packaging, read [Dependencies](development/dependencies.md),
[Runtime configuration](operations/configuration.md), and
[Commit and release](development/commit-and-release.md).

## Documentation Rules

Use English for these documents and the default [README](../README.md). Keep
the linked [Simplified Chinese README](../README.zh-CN.md) consistent with the
English entry point when commands, versions, or supported behavior change.
Use repository-relative links and synthetic data. Describe current behavior
explicitly; label proposals as proposals.

Product docs own user behavior, architecture docs own implementation contracts,
development docs own contributor procedures, and operations docs own runtime
guidance. ADRs explain durable decisions rather than repeating module reference.
Do not store live validation results or machine records in documentation.

Related root documents: [Contributing](../CONTRIBUTING.md),
[Design](../DESIGN.md), [Security](../SECURITY.md), [Privacy](../PRIVACY.md),
[Code of Conduct](../CODE_OF_CONDUCT.md), and [Third-party components](../THIRD_PARTY.md).
