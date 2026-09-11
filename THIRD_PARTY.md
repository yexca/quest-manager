# Third-Party Components and Assets

Quest Manager's original source is licensed under [AGPL-3.0-only](LICENSE).
Third-party components retain their own licenses and copyright notices. This
document is a source and notice index, not a complete binary license bundle or
a completed dependency compliance audit.

## Application Dependencies

Exact versions and resolved dependencies are recorded in [package.json](package.json),
[package-lock.json](package-lock.json), [Cargo.toml](src-tauri/Cargo.toml) and
[Cargo.lock](src-tauri/Cargo.lock). The following summarizes direct dependency
metadata; transitive components need their own review for a binary release.

| Component | Declared license | Role |
| --- | --- | --- |
| Tauri, its CLI/API, dialog plugins and tauri-build | MIT OR Apache-2.0 | Desktop runtime and build integration |
| React and React DOM | MIT | Interface |
| Lucide React | ISC | Interface icons; preserve the included icon notices |
| TypeScript | Apache-2.0 | Build-time type checking |
| Vite, its React plugin and the direct TypeScript type packages | MIT | Frontend development/build tooling |
| Serde, serde_json, base64, sha2, getrandom and png | MIT OR Apache-2.0 | Rust serialization, hashes, encoding, randomness and images |
| Tokio, quick-xml and zip | MIT | Rust subprocesses, XML and archives |

Read the license files shipped with the exact packages as well as their metadata.
A single table entry does not describe licenses of embedded or transitive code.
Node.js/npm, the Rust toolchain, Windows C++ tools/SDK and WebView2 are separate
build or system prerequisites; the app does not ship a Node.js runtime.

## Bundled External Tools

Archive versions, upstream URLs and SHA-256 values are pinned in
[toolchain.versions.json](toolchain.versions.json). Development installs live
under ignored `env`; release copies retain upstream notices.

| Component | Upstream project/source | Notice location in the portable tool bundle |
| --- | --- | --- |
| Android Platform-Tools | [Android tools](https://android.googlesource.com/platform/tools/), [ADB](https://android.googlesource.com/platform/packages/modules/adb/) | `platform-tools/NOTICE.txt` |
| Android AAPT2 | [Android frameworks/base](https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/tools/aapt2/) | `aapt2/NOTICE` |
| Android apksigner and zipalign | [apksig](https://android.googlesource.com/platform/tools/apksig/), [zipalign](https://android.googlesource.com/platform/build/+/refs/heads/main/tools/zipalign/) | `apk-tools/NOTICE.txt` and JAR notices |
| Apktool | [iBotPeaches/Apktool](https://github.com/iBotPeaches/Apktool) | License and META-INF notices inside `apk-tools/apktool.jar` |
| Eclipse Temurin JRE | [Temurin builds](https://github.com/adoptium/temurin21-binaries), [OpenJDK sources](https://github.com/adoptium/jdk21u) | `apk-tools/jre/legal/` and runtime license files |

Android and Java distributions contain multiple components with different terms;
do not label the entire bundle with one license. Preserve all notices and legal
directories, including those covering native DLLs and JAR dependencies.
Upstream project links above identify source locations; they do not by themselves
constitute an exact corresponding-source package for a particular binary.

## Repository Assets

- Application artwork is the AI-generated Mint Pilot mascot selected for this
  project. Its [source and provenance](assets/branding/README.md) are kept in
  the repository. The [icon script](scripts/generate-icon.ps1) uses the pinned
  Tauri CLI to derive PNG/ICO files and the browser favicon from that source.
- [Preview artwork](public/preview-app.svg) is a synthetic SVG, not extracted
  from an installed game.
- The inert test APK has a source manifest, checksum and generation notes in
  [the fixture README](tests/fixtures/README.md). It contains no executable app
  code. Its private signing key is not distributed.
- Real device artwork, commercial APKs, screenshots of live inventories and
  local signing keys are not repository assets and must not be added to a release.

Quest Manager is an independent project and is not affiliated with or endorsed
by Meta. Product names identify compatibility or dependencies; their trademarks
remain with their respective owners.

## Before Distributing Binaries

1. Inventory the exact npm/Rust dependency graph and every file in the selected
   portable or installer artifact, including bundled tool dependencies.
2. Collect the applicable full license texts, copyright notices and attribution
   files. Inspect JAR and runtime contents instead of relying on this summary.
3. Review source-distribution obligations for each component and arrange the
   matching source, build instructions and any required offers before publishing.
   A generic upstream homepage is not a substitute for this review.
4. Associate the executable with its precise Quest Manager source revision and
   retain the public version manifests and lockfiles needed to build it.
5. Inspect the final distribution copy for private data; exclude local environment
   records, caches, test copies and signing keys. Keep tool DLLs and notices intact.

Follow [Commit and release](docs/development/commit-and-release.md). Source
publication and binary distribution are separate review steps.
