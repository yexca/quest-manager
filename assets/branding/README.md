# Mint Pilot

[mint-pilot.png](mint-pilot.png) is the selected Quest Manager mascot artwork:
a mint-haired anime character with amber eyes, a cream VR headset and a cream
and forest-green jacket. It was generated for this project with OpenAI's built-in
image generation tool and selected by yexca. It is not artwork extracted from a
headset, game, or third-party character franchise. The original generated PNG is
retained here, including its generation provenance metadata.

## Regenerate Application Icons

After installing the project environment, run from the repository root:

```powershell
.\scripts\generate-icon.ps1
```

The script uses the pinned Tauri CLI and this source image. It writes temporary
platform variants under ignored `env/generated-icons`, then updates:

- `src-tauri/icons/icon.png`: shared sidebar, About and README artwork, and
  Tauri's application image.
- `src-tauri/icons/icon.ico`: Windows executable and installer icon, with
  multiple embedded resolutions.
- `public/favicon.ico`: browser preview tab icon.

The application's generic action icons and fictional app preview artwork have
separate purposes and are not generated from this mascot. The source image is
square; interface components apply rounded corners when displaying it. Review
the resulting 16/32/48-pixel icons and rebuild the desktop app after changes.

The source and generated assets are distributed with the project under its
[AGPL-3.0-only license](../../LICENSE). This project does not claim affiliation
with or endorsement by Meta.
