# ADR-0004: Local APK Metadata and Artwork Cache

Status: Accepted.

## Context

Package IDs and version codes are insufficient to recognize installed games.
Android package dumps expose state, SDK/ABI, permissions and install metadata,
but readable labels and artwork need resource resolution. Pulling complete VR
APKs for every list refresh would transfer large assets unrelated to this task.

## Decision

Use ADB for package-manager-derived paths, stats and current-user package dumps.
Read APK ZIP byte ranges with bounded transfer and resource sizes. Stage only
the manifest/resource table for the pinned, bundled Windows AAPT2 executable;
extract supported raster icons separately. Decode v2/v3/v3.1 certificate hashes
from the base signing block as identity metadata, without claiming verification.

Keep immutable label/icon/VR/signing metadata in a bounded private disk cache.
Keys include transport, package/version, all APK paths, sizes and modification
times. Query permission/state data again. Serialize extraction and cache clear
under a dedicated gate, independently of the existing global mutation queue.

## Consequences

The app gains one reproducibly downloaded native tool and pinned ZIP, hashing
and base64 crates. A Java runtime and external Android SDK are unnecessary.
Large game content stays on the device. Cache contents and crash-left temporary
resources are sensitive local data, documented in Privacy and excluded from Git.

Metadata is best effort: base-only resolution cannot render every adaptive,
vector or split-only icon, and ZIP64/oversized resources fall back gracefully.
Fingerprints are not APK verification, v1 parsing or signing-lineage validation.
VR declarations describe intent; they do not guarantee headset compatibility.
