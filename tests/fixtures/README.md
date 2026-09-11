# Android verification fixture

`verification.apk` is an inert package named `dev.questmanager.verification`. Its complete manifest is `AndroidManifest.xml`. It contains no executable code, permissions, activity, service, receiver, or provider. It is used only by the opt-in `run-test.ps1 -DeviceWrite` integration test, which installs it, updates it with the same APK, exports it, compares bytes, and uninstalls it. The test refuses to start if this package is already installed.

SHA-256: `6cb4ff7210c12ad42f21478b3b5c755250fba960f31138ee412057e46f470844`

The checked-in binary is the reproducible test input and does not need to be regenerated. APK preparation tests use the project-bundled Java and Android tools to edit and re-sign private copies with disposable keys. No global Java or Android SDK installation is required. The original fixture was generated using Android Build Tools 37.0.0 (`aapt2 2.20-15087165`), the Android 36 platform `android.jar`, and OpenJDK 21.0.10, then signed with a disposable local test key using `apksigner`. The private test key is not needed to use the fixture and is not distributed. Rebuilding with another key produces a different fixture and requires an intentional checksum update.
