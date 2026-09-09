# Contributing

Start with the [agent guide](AGENTS.md) and [documentation map](docs/README.md).
Keep changes focused on the requested behavior and update the matching docs in
the same change.

## Working on the Project

1. Follow [local development](docs/development/local-dev.md) to install the
   pinned environment and run the app.
2. Read the relevant [architecture boundary](docs/architecture/core-boundaries.md)
   and [product workflow](docs/product/workflows.md).
3. Implement the change through the existing frontend, IPC, and ADB layers.
4. Run the smallest sufficient checks from [Testing](docs/development/testing.md).
5. Review the actual diff for sensitive data and unintended dependency changes.

Keep all product strings in English. The default [README](README.md) and docs
are English, with a linked [Simplified Chinese README](README.zh-CN.md).
Keep both README versions and the area docs consistent when commands, versions,
or supported behavior change.

## Commit Format

Use Conventional Commits when a commit is part of the requested work:

```text
<type>(scope): <description>

feat(apps): add application filtering
fix(files): reject conflicting download names
docs(architecture): explain task cancellation
```

Preserve the user's Git configuration and any explicit instruction to leave
changes uncommitted. See [Commit and release](docs/development/commit-and-release.md)
for staged review, version locations, and local build outputs.

## Documentation and Privacy

Product behavior belongs in `docs/product`, implementation boundaries in
`docs/architecture`, contributor workflows in `docs/development`, and runtime
guidance in `docs/operations`. Use [ADRs](docs/decisions/index.md) for durable
decisions. Describe implemented behavior separately from proposals.

Use synthetic examples. Keep local dependency records, real device observations,
and diagnostic artifacts out of tracked files. Follow [Privacy](PRIVACY.md) and
[Secure development](docs/development/security.md) when preparing a change.
For a vulnerability, follow [Security](SECURITY.md).
