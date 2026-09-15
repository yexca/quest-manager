# Security Model and Reporting

Quest Manager runs with the local user's privileges and delegates device access
to ADB and Android's debugging permissions. It is a desktop management utility,
not a sandbox for a hostile computer, modified ADB executable, or compromised
headset.

## Boundaries

- The webview calls explicit Tauri business commands. It has no application API
  for arbitrary host or device shell execution.
- Host ADB arguments are passed separately. Values inserted into device shell
  commands must use the shared validation and quoting helpers.
- General file management is limited to Android shared storage. Installed APK
  export and metadata extraction read package-manager-reported APK files as
  dedicated operations. Metadata uses bounded ZIP resource and byte-range reads.
- Task execution protects storage roots, refuses ordinary transfer collisions,
  and stages transfers before publishing their final names.
- Explicit USB wireless setup stages a first-party shell helper in a random owned
  `/data/local/tmp/quest-manager-wireless-*` directory. It uses named system ADB
  methods with existing USB authorization, bounded execution and cleanup; it
  does not install an APK or expose arbitrary shell execution to the webview.
- System applications cannot be uninstalled through the task API. Root access,
  permission bypasses, and arbitrary private-data extraction are outside scope.

These checks reduce accidental and input-driven damage. They are not a
transactional filesystem sandbox: a device or another process can change paths
between validation and use. Closing the app is not a rollback guarantee.
See [Core boundaries](docs/architecture/core-boundaries.md).

## Supported Versions

Security fixes target the latest source on the default branch and the latest
published release, when one exists. Older releases and third-party builds do
not have a separate backport commitment. Include the app version and, for a
source build, the commit ID in a report. There is no guaranteed response or fix
deadline, paid support agreement, or bug bounty program.

## Report a Vulnerability Privately

Do not post exploit details in a public issue, pull request, or discussion.
Open the repository's [Security advisories](https://github.com/yexca/quest-manager/security/advisories)
page and use **Report a vulnerability** if that button is available. GitHub's
private reporting feature must be enabled by the repository owner; adding this
document does not enable it.

If the button is unavailable, open a minimal issue asking the maintainer to
enable private vulnerability reporting or provide a private contact channel.
Include no vulnerability details, attachments, logs, or personal information in
that request. Wait for a private channel before sending a reproduction. No
dedicated security email address is currently advertised by this project.

In the private report, include:

- Affected version or commit, Windows version, and relevant tool versions.
- The expected security boundary and the potential impact of crossing it.
- Minimal reproduction steps with synthetic inputs, and whether device access
  or user interaction is required.
- A proposed fix or mitigation, if known; this is optional.

Exclude real device dumps, credentials, private signing keys, personal APKs,
and unredacted terminal or UI output. APK parsing reports should use a small
synthetic sample that you have permission to share. Do not exercise a suspected
issue against another person's device or data. Coordinate disclosure with the
maintainer through the private report.

Ordinary bugs and feature requests belong in
[Issues](https://github.com/yexca/quest-manager/issues), following
[Contributing](CONTRIBUTING.md).

## APK and Dependency Risks

APK metadata and optional rebuilding process untrusted ZIP, XML, image and
resource data using application code and bundled tools. Bounds, validation and
timeouts reduce risk; these subprocesses still run with the local user's
privileges. Re-signing does not establish that an APK is safe or authentic.
Android continues to enforce its installation checks.

Private per-package keys persist so locally signed apps can receive updates.
They must never enter a bug report or source/release archive. See
[Privacy](PRIVACY.md) for their storage, retention and backup handling.

Report suspected dependency vulnerabilities that affect Quest Manager through
the same private channel. Fixed versions are introduced through the pinned
dependency workflow; installation does not automatically update bundled tools.

For implementation guidance, see [Secure development](docs/development/security.md).
For runtime data flows and diagnostics, see [Privacy](PRIVACY.md).
