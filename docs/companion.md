# Android companion

The optional phone app supplies synchronized multi-touch and Unicode text input. The desktop app remains the place to choose projects, connect agents, start and manage project sessions. The phone app handles Android's accessibility permission, pairing and synchronized project controls.

## Build and install a development APK

Install Android Studio's SDK (Android API 37 and build tools), Java 17 and Node.js 24. In this repository:

```sh
npm ci --prefix android-companion/ui
npm run build --prefix android-companion/ui
cd android-companion
./gradlew :app:assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

The ReactLynx interface bundle is packaged in the APK. The app does not fetch UI code at launch. The production manifest has no Internet or storage permission. `ProbeActivity` is included only in debug builds for gesture and Unicode checks.

Open Andromeda on the phone, tap **Enable accessibility**, and grant Andromeda in Android settings. The app explains the access before opening settings. Android requires you to grant the permission on the phone. The app detects an existing grant and saved pairing, so completed steps are skipped. If Android is still reconnecting the service, the app shows that separately and refreshes automatically; it does not ask you to grant the same permission again. Return to Andromeda, tap **Pair this computer**, then choose **Pair this phone** in the desktop app for the selected device. The pairing window lasts 60 seconds. The computer stores its credential in private application state, outside any project and outside MCP configuration.

From a terminal, the equivalent diagnostic setup is:

```sh
andromeda --device YOUR_SERIAL pair
andromeda --device YOUR_SERIAL companion-status
```

Pairing is initiated from the phone when trusting a computer. A fresh pairing replaces its previous credential; healthy existing pairing is reused. Start your project once on the desktop. The phone arms that project automatically and displays its Running, Paused or Stopped state. There is no second phone Start. Pause/Resume and Stop on the phone update the same project on desktop; other projects continue. Start stopped projects on desktop, where the directory/device/tool scope is visible. **Stop all phone projects** ends every project on that phone; desktop **Stop all projects** is the global emergency stop.

Sessions have no time limit by default. In **Settings → Session → Session timer**, choose **Off**, **15 minutes**, **30 minutes** or **1 hour**. The saved choice applies when the bridge next becomes active. A timer expiry ends the phone's projects and is synchronized to desktop. Android service interruption or restarting the phone stops its sessions. Lost contact pauses paired projects rather than silently resuming later. Heartbeats report desktop connectivity separately from saved pairing; losing a heartbeat is not a session time limit.

For direct CLI/SDK use without the desktop, **Connection controls → Start standalone session** retains the explicit phone-only authorization path. Desktop-managed automation uses the project Start button instead.

**Forget this computer** on the phone revokes the current pairing. `andromeda --device YOUR_SERIAL unpair` removes the computer's saved credential and asks the phone to stop if reachable. If the phone is offline, use Forget this computer there later. Disabling Android accessibility stops the companion service.

## Tools and limits

`adm_multi_touch` sends 1–10 concurrent finger paths. Each stroke has 1–128 native-pixel points, `start_ms` and `duration_ms`; all strokes finish within 10 seconds. Supply the latest screen width, height and rotation. The phone rejects a changed geometry. A successful dispatch reports `verified: false` at the MCP level; observe the result before claiming the intended gesture occurred.

`adm_type_text` defaults to ADB for printable ASCII. Unicode, a literal `%`, or `mode: "replace"` uses the paired companion. `backend: "companion"` selects it explicitly; `backend: "adb"` requires printable ASCII and insert mode. Companion insertion uses the focused editable field and waits for its actual text to match before reporting success. Password fields require explicit replacement mode. The phone rejects input without an active session. Failed input is not retried automatically.

The companion only accepts framed, authenticated requests forwarded through the selected device's ADB connection. It has no network listener. App content returned through screen or UI tools may be retained by the MCP client in its conversation history. Protect the project and client history accordingly.

## Verification

The repo's Rust companion seam tests use a simulated ADB server for framing, credential storage, errors, validation and tool results. The manually dispatched Android workflow can build the local UI bundle, debug/release APKs and a debug instrumentation APK, then run an emulator UI check. Hosted Actions remain disabled unless the operator explicitly requests a run. An isolated Android 17 emulator additionally verified a real two-pointer, 450 ms gesture and Unicode insertion into the debug test pad. It also confirmed light/dark and large-text rendering. Physical device gestures, TalkBack task traversal and broader rotation checks remain in the release ledger.

## Optional screenshot acceleration

On Android 11 and newer, the bridge can supply fresh screenshots using its declared screenshot capability. It supports bounded PNG/JPEG/WebP output and on-device scaling, while Rust validates the payload, encoding and native geometry. This uses the existing paired ADB connection and keeps no frame cache or files. Paused managed projects can still use allowed observation tools. Android-protected content remains protected and may appear black.

The API has a platform rate limit and a bounded callback wait. Timeout, Stop and client disconnect discard the result and close buffers, including callbacks delivered later. Older phones and unavailable APIs retain the normal ADB path in automatic mode; no additional permission is granted automatically. Update/enablement behavior must be checked on each supported platform. See [API research](research/accessibility-capture.md) for lifecycle, clock and permission details.

## Keep the bridge available in other apps

Open **Settings → Background connection**. Andromeda checks its battery-optimization exemption and Android's background restriction flag, refreshes on return from system settings, and disables the exemption request once it is already granted. The home screen links to this section while a restriction may apply.

**Allow background connection** opens Android's approval flow for this app. Android requires the owner to approve; the app does not silently whitelist itself, repeatedly reopen settings, or interrupt another app when it moves to the background. If the direct request is unavailable, it falls back to Android's battery-optimization list or this app's App info. **Open app battery settings** is available for additional manufacturer controls.

On Xiaomi/HyperOS, check App battery saver → No restrictions and any Background autostart option in App info. These vendor-specific states cannot be confirmed through Android's standard APIs. The exemption improves background availability but does not guarantee survival of OEM process management, low-memory termination, reboot or service revocation. This is separate from `desktop_offline`: the Mac/PC app must remain open.

Authenticated companion status adds `battery_exempt` and `background_restricted` diagnostics. Existing protocol clients can ignore these additional fields. See the official [Android power-management guidance](https://developer.android.com/training/monitoring-device-state/doze-standby) and [Xiaomi background-app guidance](https://www.mi.com/sg/support/faq/details/KA-538010/).
