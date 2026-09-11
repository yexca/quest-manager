# Contributing

Contributions to code, documentation, accessibility and reproducible bug reports
are welcome. Start with the [documentation map](docs/README.md) and follow the
repository rules in the [agent guide](AGENTS.md). Keep each change focused and
update the matching docs in the same pull request.

## Issues and Proposals

Search [existing issues](https://github.com/yexca/quest-manager/issues) before
opening a report. Use the bug or feature template. For a larger feature or a
change to device access, task semantics, dependencies or supported platforms,
discuss the intended behavior in an issue before investing in an implementation.
Small, focused fixes can be proposed directly.

Reports can be written in English or Chinese. Include the app version, steps to
reproduce, expected/actual behavior and relevant OS/tool versions. Use synthetic
examples and only the smallest redacted error excerpt needed to understand the
problem. Do not attach game APKs, signing keys, full device inventories or raw
environment records. Security vulnerabilities follow [Security](SECURITY.md),
not the public bug template.

## Working on the Project

1. Fork the repository and create a focused branch from the current default
   branch. Follow [local development](docs/development/local-dev.md) to install
   the pinned environment on Windows x64 and run the app.
2. Read the relevant [architecture boundary](docs/architecture/core-boundaries.md)
   and [product workflow](docs/product/workflows.md).
3. Implement the change through the existing frontend, IPC, and ADB layers.
4. Run the smallest sufficient checks from [Testing](docs/development/testing.md).
5. Review the actual diff for sensitive data and unintended dependency changes.

Keep all product strings in English. The default [README](README.md) and docs
are English, with a linked [Simplified Chinese README](README.zh-CN.md).
Keep both README versions and the area docs consistent when commands, versions,
or supported behavior change.

Use the root PowerShell entry points:

```powershell
.\run-install.ps1
.\run-dev.ps1
.\run-test.ps1
```

The setup checks the recorded system Node/npm versions and keeps project tools
and caches in `env`. Retain both lockfiles, use exact dependency versions, and
do not use `-RefreshLocks` for a routine install. See
[Dependencies](docs/development/dependencies.md) for intentional updates.

## Pull Requests and Validation

Explain the problem, resulting behavior, and any important limitations. Link a
related issue when one exists. List the checks you actually ran and their
outcomes; describe skipped checks without claiming they passed.

- Documentation changes need link, command, factual and whitespace checks.
- UI changes need the frontend build and relevant preview inspection. Use the
  explicit fictional preview for public screenshots.
- Rust or IPC changes normally need `run-test.ps1` and focused regression
  coverage. Preserve captured device targeting, the global mutation queue,
  storage boundaries and signing-key retention.
- Device tests are optional and require an intended test headset and authority
  to perform their reads or writes. Read [Testing](docs/development/testing.md)
  before opting in; the normal test command does not touch a headset.

There is currently no automated CI or JavaScript unit-test runner. A local
frontend build checks types and compilation, not native or device behavior.
Do not add generated output or local validation transcripts to a pull request.
Maintainers may request a smaller scope, additional evidence or follow-up work.

## Contribution License and Conduct

By submitting a contribution for inclusion, you agree to license your original
contribution under this project's [AGPL-3.0-only license](LICENSE). You retain
your copyright. Submit only material you have the right to contribute; identify
third-party code or assets and preserve their applicable licenses and notices.
See [Third-party components](THIRD_PARTY.md). No separate CLA or mandatory
Signed-off-by trailer is currently required.

AI-assisted contributions follow the same review and validation requirements.
Review the generated changes yourself and do not include private prompts,
credentials or copyrighted material you cannot redistribute.

Be respectful, discuss changes rather than attacking people, and follow the
[Code of Conduct](CODE_OF_CONDUCT.md).

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
