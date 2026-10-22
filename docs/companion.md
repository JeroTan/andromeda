# Android companion

The optional phone app supplies synchronized multi-touch and Unicode text input. The desktop app remains the place to choose projects, connect agents, review input requests and pause automation. The phone app handles Android's accessibility permission, pairing and a 15-minute session that you start and stop on the phone.

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

Open Andromeda on the phone, tap **Enable accessibility**, and grant Andromeda in Android settings. The app explains the access before opening settings. Android requires you to grant the permission on the phone. Return to Andromeda, tap **Pair this computer**, then choose **Pair this phone** in the desktop app for the selected device. The pairing window lasts 60 seconds. The computer stores its credential in private application state, outside any project and outside MCP configuration.

From a terminal, the equivalent diagnostic setup is:

```sh
andromeda --device YOUR_SERIAL pair
andromeda --device YOUR_SERIAL companion-status
```

Pairing is initiated from the phone each time a new computer is trusted. A fresh pairing replaces the previous credential. Start a session on the phone when you want to use its accessibility tools. Sessions expire after 15 minutes. Stop from the phone at any time. The desktop **Stop & pause** cancels managed requests; it also keeps new managed input paused until you resume. Stopping one project's MCP work does not revoke the phone's pairing.

**Forget this computer** on the phone revokes the current pairing. `andromeda --device YOUR_SERIAL unpair` removes the computer's saved credential and asks the phone to stop if reachable. If the phone is offline, use Forget this computer there later. Disabling Android accessibility stops the companion service.

## Tools and limits

`adm_multi_touch` sends 1–10 concurrent finger paths. Each stroke has 1–128 native-pixel points, `start_ms` and `duration_ms`; all strokes finish within 10 seconds. Supply the latest screen width, height and rotation. The phone rejects a changed geometry. A successful dispatch reports `verified: false` at the MCP level; observe the result before claiming the intended gesture occurred.

`adm_type_text` defaults to ADB for printable ASCII. Unicode, a literal `%`, or `mode: "replace"` uses the paired companion. `backend: "companion"` selects it explicitly; `backend: "adb"` requires printable ASCII and insert mode. Companion insertion uses the focused editable field and waits for its actual text to match before reporting success. Password fields require explicit replacement mode. The phone rejects input without an active session. Failed input is not retried automatically.

The companion only accepts framed, authenticated requests forwarded through the selected device's ADB connection. It has no network listener. App content returned through screen or UI tools may be retained by the MCP client in its conversation history. Protect the project and client history accordingly.

## Verification

The repo's Rust companion seam tests use a simulated ADB server for framing, credential storage, errors, validation and tool results. Android PR workflows build the local UI bundle, debug/release APKs and a debug instrumentation APK, then run an emulator UI check. An isolated Android 17 emulator additionally verified a real two-pointer, 450 ms gesture and Unicode insertion into the debug test pad. It also confirmed light/dark and large-text rendering. Physical device gestures, TalkBack task traversal and broader rotation checks remain in the release ledger.
