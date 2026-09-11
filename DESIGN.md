# Design

Quest Manager should make device maintenance clear and calm. The desktop user
should be able to identify the connected headset, choose an application or
file, and see the result of an operation without losing their place.

## Current Interface

- A dark green sidebar holds Overview, Applications, Files, the installation
  entry, Task queue, Help, and About.
- A light workspace contains the connection status, page heading, and content.
  Overview summarizes the device; Applications and Files use compact tables.
- Green is the primary action color, amber indicates attention, and red marks
  errors or destructive actions. Neutral surfaces and borders provide structure.
- Interface icons come from Lucide and the headset illustration is inline SVG.
  Applications display extracted raster icons, with generic tiles when artwork
  is unavailable. Keep package IDs visible beneath readable application names.
- Styling lives in [src/styles.css](src/styles.css). There is one light theme,
  system fonts, a visible keyboard focus style, and reduced-motion support.
- The Tauri window defaults to 1280 by 850 and has a 1000 by 680 minimum.
  Layout rules adapt within the supported desktop sizes. There is no mobile UI.

## Interaction Contract

About is a full page available without a connected headset. It presents project
credits, manifest-derived core dependency versions, the public source repository
and an expandable offline license. Its repository link opens the default browser;
it never navigates the management webview away from the app. Help retains the
connection, transfer and local signing-key guidance.

Keep interface text and errors in English. Use concrete names such as
"Install APK", "Upload files", and "Download". Explain the effect of an action
in user terms; toolchain details belong in developer documentation.

Keep one primary action obvious in each toolbar. Row utilities should remain
compact and have accessible names. Preserve labels, semantic tables, visible
focus, and native dialog keyboard behavior when extracting components.

Distinguish disconnected, authorization-required, loading, empty, and failed
states. A failed live query must not turn the desktop app into a preview or
display fictional values as device data. Unknown values should remain unknown.

Installation shows selected APKs and their expected default launcher artwork,
name, package/version and target headset before queuing. Keep each file's edit
and compatibility options independent and off by default. Label unavailable
previews honestly. Show signature/update consequences when preparation is enabled;
name/icon changes include compatibility signing. Local review works offline,
while installation still requires a ready device and the desktop runtime. File
deletion and application uninstallation show an explicit destructive
confirmation. A directory deletion includes its contents; uninstallation
removes app data. File picker cancellation must not create a task.

Keep the queue available across page navigation. Indeterminate progress is
appropriate when ADB has no percentage; do not invent elapsed-time progress.
Show task errors and cleanup details where the user can act on them. Only
display cancellation when the backend supports it for that task state.

## Changes to the UI

Reuse existing patterns before introducing another modal, toolbar, or status
style. If shared styling is extracted into tokens or components, migrate the
affected surfaces together; the current app does not have a token framework.
Keep page composition separate from IPC calls and backend validation.

Use the explicit browser preview for layout review. Keep the
"Preview · sample data" badge visible and data fictional. Screenshots intended
for repository documentation must use preview data. Real device identifiers,
paths, package inventories, and task details can otherwise appear in the UI.

See [Product workflows](docs/product/workflows.md) for behavior and
[Testing](docs/development/testing.md) for proportional validation.
