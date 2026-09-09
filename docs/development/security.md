# Secure Development

Use the [core boundaries](../architecture/core-boundaries.md) as implementation
contracts. The current safeguards are scoped to a local desktop app; new code
must not silently broaden them.

## Subprocesses and IPC

- Register explicit commands in `lib.rs`; keep the frontend API business-oriented.
  Do not expose a raw ADB, PowerShell, or Android shell executor to the webview.
- Construct the host process through `Adb::command` with separate arguments.
  Validate transport strings and package names using the shared helpers.
- Device shell commands are a second parsing boundary. Quote every interpolated
  path/value with `shell_quote`, retain `--` operand separators where supported,
  and test relevant metacharacters when changing command construction.
- Preserve child-process cleanup, query/task timeouts, and cooperative transfer
  cancellation. A UI action must not launch a hidden unbounded background job.
- Extend Tauri permissions or CSP only for a concrete feature. Current
  capabilities provide core behavior and native dialogs; there is no shell or
  general filesystem plugin exposed to JavaScript.

## Filesystem Rules

General remote paths need lexical normalization and device-side resolution
before use. Mutations must keep root/container protections. APK export uses the
separate package-manager path flow; do not reuse that exception for arbitrary
file access.

For downloads, preserve UTF-8 checks, basename validation, Windows reserved-name
rules, case-collision detection, and descendant symlink rejection. For host
paths, canonicalize the chosen source or parent; do not build a destination by
trusting device output as a full Windows path.

Keep task-owned temporary paths adjacent to the final destination, refuse
collisions, and report cleanup failures. Cleanup must be confined to the exact
temporary path owned by the task. Preserve source and final user files on a
failed transfer. Preflight checks are not a concurrency-proof sandbox; do not
promise atomicity across another process changing the same paths.

## Errors and Output

ADB output, file names, and package metadata are untrusted data. Render them as
text, not HTML or executable snippets. Keep useful local errors, but recognize
that current error strings and task messages can contain private details.
Do not assume truncation makes output safe to publish.

Task subprocess output retains a bounded tail. Ordinary read queries and
directory-tree preflight currently buffer their output. A feature that increases
query volume must consider memory, timeout, and cancellation behavior instead
of assuming every ADB output path already streams.

## Repository Privacy Review

Review changed files, newly created files, and the actual staged blobs before a
requested commit. The repository currently has no dedicated secret-scanning
command; a text search is one part of review, not proof of absence.

Check for real device serials, private network endpoints, MAC addresses,
installed-app inventories, personal paths, credentials, signing keys, diagnostic
logs, and validation records. Include screenshots, archive contents, image
metadata, and signing-certificate identity when binaries change.

Use fictional device/transport names, reserved documentation IPs, `example.com`
for hypothetical endpoints, and `com.example.*` package names. Public platform
names and official package-distribution URLs are ordinary project metadata.
The checked-in inert verification APK is a reviewed synthetic fixture; its
private key is not part of the repository.

Keep `env`, `release`, generated schemas, build output, and local installation
records ignored. `.gitignore` cannot protect an already tracked file or remove
content from history. Do not recreate `VALIDATION.md` or move observations into
another tracked report.

See [Privacy](../../PRIVACY.md), [Testing](testing.md), and
[Commit and release](commit-and-release.md).
