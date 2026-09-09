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
- The visible version strings in [src/App.tsx](../../src/App.tsx).

Update the appropriate lock data through an intentional maintenance operation
and review its diff. Toolchain/dependency versions are separate from the app
version; see [Dependencies](dependencies.md).

## Local Packaging

```powershell
.\run-build.ps1
.\run-build.ps1 -Installer
```

The default build places `quest-manager.exe`, `platform-tools`, and a local
`build-environment.json` record in `release`. Release-mode code resolves ADB from
the application resources, so a lone executable is not the complete portable
package. Preserve the ADB DLLs and license files when preparing a distribution.

`-Installer` also creates an NSIS bundle under
`env/target/release/bundle/nsis`. The pinned Tauri CLI may download its packaging
tools into `env/target/.tauri`. Application code signing is not configured in the
checked-in Tauri setup; Git commit signing is a separate concern.

Before distributing output, inspect resource completeness and review or omit
machine-specific environment records from the distribution copy. Keep the
original local installation record for the development scripts. Build output
and environment records remain ignored by Git.

No automated CI, release workflow, remote repository, or publication destination
is defined by these docs. Add such infrastructure only as part of a requested
change. Public release notes should describe shipped behavior, not device
inventories, machine paths, or raw validation transcripts.
