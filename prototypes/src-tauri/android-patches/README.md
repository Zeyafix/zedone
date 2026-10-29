# Tauri 2.12.5 Android lifecycle patch (BUG-001)

The released manager keeps loaded plugin instances after Activity recreation.
On the first Galaxy A32 launch, CONFIG_ASSETS_PATHS destroyed their owner while
the visible WebView belonged to a new Activity. Barcode Scanner bound CameraX
to the destroyed owner and showed no preview. Evidence: [EV-022](../../../docs/agent/verification/EV-022-qr-scanner.md).

`PluginManager.kt` is the upstream 2.12.5 source with narrow lifecycle hooks.
The upstream copyright and MIT license are retained. `PluginLifecycle.kt`
retains registration classes/configuration, releases handles and Activity/View
references at owner destruction, and recreates instances for the current owner.
It uses the same public Activity constructor as Tauri's Rust registration.
Private-field reflection is used only by the instrumentation regression test.

`gen/android/tauri-lifecycle.gradle.kts` checks the normalized upstream manager
SHA-256, then generates a patched source tree under the Android build directory.
The original Cargo dependency sources remain untouched. Configurations and
Rust capabilities remain authoritative; there are no new JS/IPC permissions.

This is an explicit, version-bound dependency patch, not a general plugin API.
Review/remove it when an upstream lifecycle fix is adopted. A changed upstream
source fails the build until reviewed. Do not bypass the check or edit Cargo's
registry. Remove the Gradle apply and this directory to revert the patch; the
white-preview regression will return with the affected upstream version.

The regression test forces ActivityScenario.recreate(), checks actual Tauri
plugin owners/WebView contexts, and verifies preserved configuration and owner
cleanup on an isolated probe. Full-uninstall/install and real CameraX STREAMING
are additionally checked on the physical phone; APK builds alone do not suffice.

`bun run android:build [aarch64]` also builds the matching instrumentation APK.
`bun run check:android` installs/runs it before the AOSP UI suite. For a manual
device run, use `am instrument -w -r -e waitForActivitiesToComplete false -e class
app.zedone.prototype.PluginLifecycleTest
app.zedone.prototype.test/androidx.test.runner.AndroidJUnitRunner`.
The AndroidX runner option lets it publish the JUnit result before process
teardown: closing Tauri's final window first otherwise exits the native process.
The test deliberately leaves its final Activity for that teardown. The runner
still reports assertion failures and exceptions; the script requires `OK (1 test)`.
