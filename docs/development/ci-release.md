# CI and Release Automation

## CI

[ci.yml](../../.github/workflows/ci.yml) runs on pull requests, pushes to `main`,
and manual dispatch. The **Windows checks and portable build** job:

1. Checks that all application version locations agree.
2. Checks VS 2022, MSVC and Windows SDK on `windows-2022`.
3. Explicitly installs/updates Microsoft-signed WebView2 on the disposable
   hosted runner if necessary, then runs `run-install.ps1 -NonInteractive`.
4. Runs `run-test.ps1` without device flags.
5. Runs `run-build.ps1 -Portable Online`, verifies ZIP resources/source/hash,
   and uploads the ZIP and sidecar as an Actions artifact retained for seven days.

Download the artifact from the successful workflow run's **Artifacts** section.
The artifact wrapper contains the portable ZIP and its checksum; extract the
portable ZIP completely before running `Start-QuestManager.cmd`.

CI does not launch the desktop app, install USB drivers or contact a headset.
Source checks run after setup and tests, and again after packaging. Unexpected
changes stop the workflow and report repository-relative paths for diagnosis.
An online package build does not exercise missing-WebView2 consent on an end-user
machine. No browser E2E or clean Windows 10/11 verification is implied.

## Version Tag to Draft Release

[release.yml](../../.github/workflows/release.yml) runs when a `v*` tag is pushed.
Only stable `vMAJOR.MINOR.PATCH` tags matching every manifest/lockfile are accepted.
The tag must resolve to the clean checked-out source. Both lightweight and
annotated tags work. A tag build repeats the host suite, builds online and offline
portables, and verifies both ZIPs before any release upload.

For example, after review, committing and pushing the intended source, a
maintainer can create and push its version tag:

```powershell
git tag v0.1.0
git push origin v0.1.0
```

These are publication-triggering commands, not setup commands. Do not create or
move a tag until that exact source commit is intended for the release. Update
the version locations in [Commit and release](commit-and-release.md) together
before a later release.

The workflow creates a **draft**, with these four assets for `v0.1.0`:

- `quest-manager-v0.1.0-windows-x64-portable-online.zip`
- `quest-manager-v0.1.0-windows-x64-portable-online.zip.sha256`
- `quest-manager-v0.1.0-windows-x64-portable-offline.zip`
- `quest-manager-v0.1.0-windows-x64-portable-offline.zip.sha256`

Artifacts are also retained on the workflow run for 14 days. Only ZIPs and
sidecars are uploaded: never the whole `release` or `env` tree, `comparison.json`,
local build records, PDBs, logs or keys. Each ZIP records its exact source commit.
The packages remain unsigned; no signing secret is needed or configured.

Before clicking **Publish release**, complete the third-party notices and
corresponding-source review, clean Windows startup and relevant Quest checks,
and replace the draft review notes with the intended release notes. Building
successfully does not resolve these previously outstanding release requirements.
The workflow does not auto-publish the draft.

Rerun a failed run using GitHub's rerun control, or manually dispatch **Release**
with the existing version tag selected. With GitHub CLI:

```powershell
gh workflow run release.yml --ref v0.1.0
```

Manual dispatch from a branch fails with an instruction to select a version tag.
Reruns may replace the four draft assets, while preserving edited draft notes.
They refuse to replace assets once the release is public. Use a new version for
a corrected published release. Tags moved during a build are rejected at upload.

## Runner, Permissions and Reproducibility

Enable GitHub Actions and allow the pinned `actions/checkout`,
`actions/upload-artifact`, and `actions/download-artifact` actions in repository
or organization policy. No PAT is required: builds use read-only `GITHUB_TOKEN`,
and the separate draft job requests `contents: write`. That job does not check
out or execute repository code. Policy restricting token write access must be
resolved by the repository owner. Fork PRs run through `pull_request` with no
release credentials; no `pull_request_target` workflow is used.

The hosted runner uses the repository's downloaded Node/npm, Rust and Java in
`env`, not the preinstalled versions. Downloads and locks retain their existing
checks. There are no shared dependency/build caches. Workflows have timeouts,
cancel superseded CI runs, and serialize releases for the same tag.

`Initialize-CiRunner.ps1 -InstallWebView2` is explicit authorization to set up
the shared runtime on that disposable GitHub-hosted Windows machine. It refuses
local and self-hosted environments. Missing/incompatible C++ or SDK components
fail with a runner-selection diagnostic. Local `run-install.ps1 -NonInteractive`
still never installs system prerequisites.

Hosted images and Evergreen WebView2 change over time; these builds are not a
claim of bit-for-bit reproducibility. The offline runtime remains pinned in
`packaging/webview2.json`. A compatible compiler must support the source-path
remapping used by portable builds; privacy scans reject remaining checkout or
user-profile paths. See [Portable packages](portable.md) for runtime behavior.

Repository branch protection is separate: after a successful run, the owner can
require **Windows checks and portable build**. The workflow file alone does not
configure that rule or prove that the first hosted run has passed.
