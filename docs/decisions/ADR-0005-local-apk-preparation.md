# ADR-0005: Local APK Preparation and Signing

Status: Accepted

## Context

Users need to preview APKs, customize launcher names/icons, and try an alternative
signature when Android's verity verification overflows on a large APK. Appearance
changes invalidate the original signature; a new signing identity usually cannot
update an original installation.

## Decision

Ordinary installation remains the default. Each selected APK has an independent
opt-in request with its preview size/mtime stamp, optional name/PNG icon and
compatibility choice. Preview works without a headset and does not queue writes.

Preparation runs in the existing global mutation queue, bound to its captured
transport. Compatibility-only installation aligns and signs a private copy.
Appearance editing uses pinned Apktool/AAPT2 to rebuild the manifest/resources;
original code, libraries and game assets are copied as compressed ZIP entries.
Resource paths, duplicate/case collisions and size limits are checked before
decoding. Resource-free APKs receive a resource table when needed.

Prepared APKs use v1/v2 with verity disabled. They are aligned before signing,
verified with official apksigner, and checked for the expected package/version,
name/icon, absence of verity and original payload metadata before installation.
This is a workaround, not a signature bypass or a guarantee of game compatibility.

Each package has a stable, locally generated key. Its key/password files reside
together in an ACL-restricted local folder, separate from artwork and staging.
Users privately back up/restore the entire folder. Keys are never bundled,
committed, uploaded or deleted by artwork clearing. The application never
automatically uninstalls a conflicting package.

Pinned Java, Apktool, apksigner and zipalign are bundled under `apk-tools`; no
global Java/SDK is used. Subprocesses use explicit arguments, bounded output,
hidden windows and timeouts. Java temporary files belong to the task directory.

## Consequences

The distribution grows. Disk preflight conservatively reserves four APK sizes
plus 512 MiB. Editing accepts at most 128 MiB of uncompressed resources and one
launcher entry; unsupported packages fail explicitly. Split installation remains
outside scope. Unknown/adaptive icons are unavailable previews, not proof that an
APK has no icon. Quest launcher results need opt-in device validation.

Completion/failure removes task-owned copies; cleanup failures report the path.
Crashes can leave private staging files. Signing keys persist until user removal.
There is still no task persistence, automatic retry, uninstall fallback or resume.
