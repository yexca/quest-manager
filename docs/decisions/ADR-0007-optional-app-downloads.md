# ADR-0007: Optional Public APK Downloads

Status: Accepted

## Context

Launcher setup needs public release discovery and downloads in an otherwise local
ADB application. The companion Navigator service is independently versioned.

## Decision

Add a narrowly scoped backend HTTPS client for the fixed LightningLauncher GitHub
repository, using exact reqwest 0.13.5 and rustls. Retain the webview CSP and explicit
business IPC; do not expose arbitrary HTTP, filesystem or shell operations. Read
upstream version declarations as data without evaluating code. Label unknown
compatibility honestly and allow users to select an unverified historical service.

Pin selected asset metadata at queue creation. One existing install task owns all
components through bounded download, package/version and original signature
verification, serial installation, partial completion and cleanup. No automatic
permission changes, retries, downgrade or uninstall are introduced.

A single localStorage boolean remembers suggestion visibility across restarts,
independently of any headset. Catalog and live device state stay in memory.

## Consequences

GitHub availability/rate limits can prevent optional setup without affecting local
APK installation. Upstream source layout changes can remove automatic version
recommendations. The network/data flow is documented in Privacy. Third-party APKs
are downloaded on demand and not included in Quest Manager's release bundle.
