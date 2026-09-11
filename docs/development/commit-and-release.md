# Commit and Release

## Commit Scope and Format

Follow the user's current instruction about whether to commit. A request to
leave work uncommitted takes precedence over the normal handoff workflow. A
local commit does not imply authorization to push, tag, or publish artifacts.

Use Conventional Commits:

```text
<type>(scope): <description>

feat(apps): add application filtering
fix(files): reject conflicting download names
docs(agents): explain repository workflows
```

Common scopes include `app`, `apps`, `devices`, `files`, `tasks`, `deps`,
`build`, `docs`, and `agents`. Keep the description about the final behavior.

## Before a Requested Commit

1. Run validation proportional to the change, as described in [Testing](testing.md).
2. Review new and changed files for sensitive information, including binary
   metadata when relevant, following [Secure development](security.md).
3. Stage the intended files and inspect `git diff --cached --stat`,
   `git diff --cached`, and `git diff --cached --check`. Review the index rather
   than assuming it matches the working files.
4. Verify local records, dependencies, generated files, and validation reports
   are absent from the staged tree.
5. Commit using the existing Git identity and signing configuration. If a
   configured signer fails, report the concrete blocker instead of disabling
   signing or replacing the signer.
6. Verify the commit and remaining working-tree state, and report the result
   without including private identity or device data.

Do not recreate deleted validation records as part of a handoff. There is no
repository-specific signing service or release branch policy to infer from
another project's workflow.

## Application Version Locations

There is no single `VERSION` file or automated version-bump command. For an
intentional app version change, keep these locations consistent:

- [package.json](../../package.json) and its root entries in
  [package-lock.json](../../package-lock.json).
- [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml) and the application package
  entry in [Cargo.lock](../../src-tauri/Cargo.lock).
- [src-tauri/tauri.conf.json](../../src-tauri/tauri.conf.json).
- About, the sidebar and Help read the version from `package.json` through
  [src/About.tsx](../../src/About.tsx); no separate UI version edit is needed.

Update the appropriate lock data through an intentional maintenance operation
and review its diff. Toolchain/dependency versions are separate from the app
version; see [Dependencies](dependencies.md).

## Local Packaging

```powershell
.\run-build.ps1
.\run-build.ps1 -Installer
```

The default build places `quest-manager.exe`, `platform-tools`, `aapt2`,
`apk-tools`, `LICENSE`, and a local `build-environment.json` record in `release`.
Release-mode code resolves ADB from
the application resources, so a lone executable is not the complete portable
package. Preserve the ADB DLLs, AAPT2 executable and license/NOTICE files when
preparing a distribution. Tauri also includes all three tool directories in NSIS.
Keep the private Java runtime and JAR notices with the preparation tools; never
bundle local signing keys.

`-Installer` also creates an NSIS bundle under
`env/target/release/bundle/nsis`. The pinned Tauri CLI may download its packaging
tools into `env/target/.tauri`. Application code signing is not configured in the
checked-in Tauri setup; Git commit signing is a separate concern.

Before distributing output, inspect resource completeness and review or omit
machine-specific environment records from the distribution copy. Keep the
original local installation record for the development scripts. Build output
and environment records remain ignored by Git.

The source repository is [yexca/quest-manager](https://github.com/yexca/quest-manager).
Setting a repository URL in About or package metadata does not configure a Git
remote or authorize pushing. There is no automated CI or release workflow.
Public release notes should describe shipped behavior, not device inventories,
machine paths, or raw validation transcripts.

## Before the First Public Push

- Review the actual commit history as well as the working tree for personal
  paths, device details, logs and keys. Ignore rules do not remove old blobs.
- Keep source, lockfiles and the root AGPL-3.0-only `LICENSE` together. Review
  any third-party assets and fixture provenance before publication.
- Check the intended Git remote, branch and authenticated account. Configure
  and push only within the user's requested scope.
- Report outstanding device/large-APK validation honestly. Source publication
  and a tested binary release are separate milestones.
- Before distributing binaries, audit the complete dependency licenses and
  notices (including Java and Android tools), and provide the matching source
  and build instructions required by the applicable licenses. The core-tool
  list in About is not a complete third-party notices inventory.
- Review [Contributing](../../CONTRIBUTING.md), [Security](../../SECURITY.md),
  [Code of Conduct](../../CODE_OF_CONDUCT.md), the `.github` issue/PR templates
  and [third-party component index](../../THIRD_PARTY.md) with the source.

## GitHub Repository Settings

These are owner-managed settings, not features activated by checking in files:

- Enable Issues if using the included bug/feature templates. Templates become
  available after the files reach the repository's default branch.
- Enable **Private vulnerability reporting** under the repository's security
  settings and verify the **Report a vulnerability** entry. Keep the conditional
  fallback in `SECURITY.md` accurate. See
  [GitHub's configuration guide](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/configure-vulnerability-reporting/configure-for-a-repository).
- Review notifications for security reports. Do not publish an email address or
  promise a support deadline without the maintainer's agreement.
- Review available dependency alerts and secret-scanning/push-protection settings
  for the repository. A scan is useful evidence, not a guarantee that data is safe.
- Consider branch rules and Windows CI when the actual runner can satisfy the
  pinned toolchain. There is currently no workflow or required status check to
  select; do not document unconfigured checks as enforced.

Do not enable automatic dependency upgrades casually: the project requires exact
versions, coordinated manifests and review of both lockfiles. A future update
workflow must follow [Dependencies](dependencies.md).

## Dependency Security Review

After setup, an explicit npm check can inspect the locked dependency graph:

```powershell
npm audit --package-lock-only --ignore-scripts --cache env/npm-cache
```

This contacts the configured npm registry with dependency metadata. It does not
audit Rust crates or bundled Java/Android tools, and a clean result does not prove
that dependencies have no vulnerabilities. Do not use `npm audit fix` to rewrite
the pinned graph without an intentional dependency update. No Rust advisory
scanner is currently configured; review Rust and bundled-tool advisories as a
separate release step. Keep raw local reports outside tracked files.
