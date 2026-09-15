# USB Wireless Helper

`QuestWireless.smali` is first-party source for a small Android shell helper.
The Rust build script assembles it with the project's pinned Apktool and JRE
using `--no-apk`, then embeds `classes.dex`. No external SDK, JDK, Android Studio,
APK installation or checked-in compiled helper is required.

The authorized USB setup flow stages read-only DEX bytes in a random owned
directory under `/data/local/tmp`. It passes the current BSSID, a fresh service
name and secret through stdin. Binary transfer uses ADB `exec-in` with SHA-256
verification before execution. The helper calls named `IAdbManager` methods to
enable TLS debugging for the current network without permanently trusting the
network, then returns the dynamic port. It waits while the parent tests existing
ADB trust; only a `pair` request starts a new pairing listener. It acknowledges
that request and waits for stdin closure before stopping its listener.

The desktop wrapper bounds execution, verifies physical identity over Wi-Fi,
and attempts cleanup. No arbitrary helper command/path is exposed through IPC.
Platform method or permission availability can vary; failures preserve explicit
errors instead of falling back to root, guessed Binder transaction numbers,
port scanning or legacy `tcpip 5555` setup.

See [ADR-0008](../../docs/decisions/ADR-0008-explicit-wireless-setup.md),
[runtime workflows](../../docs/architecture/workflows.md), and
[privacy](../../PRIVACY.md) for lifecycle and retention contracts.
