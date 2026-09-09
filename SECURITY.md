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
- System applications cannot be uninstalled through the task API. Root access,
  permission bypasses, and arbitrary private-data extraction are outside scope.

These checks reduce accidental and input-driven damage. They are not a
transactional filesystem sandbox: a device or another process can change paths
between validation and use. Closing the app is not a rollback guarantee.
See [Core boundaries](docs/architecture/core-boundaries.md).

## Reporting

No dedicated private security-reporting address or support SLA is configured in
this repository. Arrange a private channel with the repository owner before
sharing a sensitive reproduction. Public examples should use synthetic device
identifiers, package names, paths, and files.

A useful report describes the affected version, expected boundary, steps using
synthetic inputs, and potential impact. Exclude real device dumps, credentials,
personal APKs, and unredacted terminal or UI output. Do not exercise a suspected
issue against another person's device or data.

For implementation guidance, see [Secure development](docs/development/security.md).
For runtime data flows and diagnostics, see [Privacy](PRIVACY.md).
