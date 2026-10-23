# While Codex was away

## Operator

- **Model:** DeepSeek V4.1 Flash Free (OpenCode)
- **Date:** 2026-10-23
- **Scope:** Finish Codex's uncommitted Android companion/session-timer work and the capture benchmark harness without changing host environment settings or wiping the existing AVD.

## Work completed

- 2026-10-23 — Read the repository instructions, specification, implementation ledger, companion/desktop/API guides and existing evidence before changing code.
- 2026-10-23 — Preserved Codex's uncommitted work and completed the phone Settings flow: session timer choices (**Off**, **15 minutes**, **30 minutes**, **1 hour**), persisted choice, settings navigation, native back handling and the separate “permission enabled / service connecting” state.
- 2026-10-23 — Hardened `android-companion/ci/emulator-smoke.sh`:
  - grants TalkBack's notification permission before enabling it, so the Android 13+ permission dialog cannot cover the app under test;
  - resets app data and display scale/theme for deterministic runs;
  - force-stops the app between timer tests so persistence is checked across a new process;
  - starts the large-text/dark-mode pass with an explicit cold start;
  - uses `grep` instead of requiring `ripgrep`;
  - captures and restores the emulator's original font scale, night mode and accessibility settings on exit.
- 2026-10-23 — Diagnosed the permission-state instrumentation failure on the API 37 emulator. `UiAutomation` clears `enabled_accessibility_services` while suppressing services, so the old check became a permission-off test. Added a debug-only instrumentation hook and changed the assertion to the visible “Permission set. Connecting…” title. The five disable/re-enable cycles and the permission-vs-service check now pass.
- 2026-10-23 — Ran the full phone smoke flow on the existing `Medium_Phone_API_37.0` AVD without `-wipe-data`:
  - TalkBack permission button opens Android accessibility settings;
  - session timer defaults to Off, all choices and settings navigation pass;
  - timer survives a new app process;
  - app stays running with large text and dark mode;
  - five accessibility service disable/re-enable cycles pass;
  - enabled permission remains recognized while the service handle is unavailable.
- 2026-10-23 — Verified the capture benchmark harness against the running emulator. A 20-sample run (2 warm-up samples per workload) completed with zero failures for direct ADB PNG, external ADB CLI PNG, controller native PNG and controller 720px JPEG. Raw result: `benchmarks/results/emulator-api37-2026-10-23.json`.
- 2026-10-23 — Recorded the emulator result (battery 100%, 25.0 °C, emulator transport): direct ADB PNG p50 412 ms / p95 440 ms; external ADB CLI p50 416 ms / p95 420 ms; controller native PNG p50 442 ms / p95 491 ms; controller 720px JPEG p50 454 ms / p95 460 ms. These are emulator measurements only, not product-comparison claims.
- 2026-10-23 — Ran formatting, strict Clippy, Rust tests, UI formatting/build, Android debug/release/androidTest builds and the emulator smoke flow. Restored the AVD to font scale 1.0 and light mode afterwards.
- 2026-10-23 — Updated the implementation ledger and benchmark README with the verified emulator evidence, and added this log.
- 2026-10-23 — Added a persistent `ScreenStream` over one ADB shell-v2 connection. Complete PNG frames are parsed by chunk structure, with a 32 MiB frame cap, bounded buffering, timeout handling and explicit stream-ended errors. Added `Controller::stream` and transport tests for frame framing, oversized chunks and timeouts.
- 2026-10-23 — Added explicit desktop live preview. **Start live preview** consumes the persistent stream and emits `preview-frame`/`preview-error` events; **Stop live preview** or **Hide** stops it. Preview is cleaned up on project/device changes and window close, and images are never saved. The desktop preview-loop test passes.
- 2026-10-23 — Expanded the benchmark harness to schema version 2 with `persistent_stream_png`, opt-in `controller_press_home`, stream duration/interval controls and host CPU/RSS sampling. Added the emulator and physical raw results described below.
- 2026-10-23 — Ran the release benchmark on the API 37 emulator: 20 samples per workload, zero failures. Direct ADB PNG p50 413 ms / p95 437 ms; external ADB CLI 423/442; controller native 462/486; controller 720px JPEG 482/515; persistent stream 526/543 ms at 1.89 fps; Home input 20.8/43.3 ms. Raw result: `benchmarks/results/emulator-api37-2026-10-23-stream.json`.
- 2026-10-23 — Ran the release benchmark on the physical OnePlus CPH2653 (Android 16/API 36, USB, battery 95%, 37.0 °C): 20 samples per workload, zero failures. Direct ADB PNG p50 1812 ms / p95 2049 ms; external ADB CLI 1864/1982; controller native 1957/2112; controller 720px JPEG 1995/2034; persistent stream 1937/1959 ms at 0.52 fps; Home input 29.3/36.0 ms. Raw result: `benchmarks/results/oneplus-cph2653-2026-10-23.json`.
- 2026-10-23 — Implemented the npm launcher (`npm/launcher.mjs`) and root package metadata. It resolves an explicit desktop executable, macOS app bundle, platform package or local Tauri build, passes arguments through a `--` separator, preserves the desktop exit status and reports a missing build. Five launcher tests pass; CI now has a dedicated launcher job.
- 2026-10-23 — Rebuilt the debug macOS package after the final code changes. The launcher's `--print-path` resolves `target/debug/bundle/macos/Andromeda.app/Contents/MacOS/andromeda-desktop`, and the bundled MCP executable is present.
- 2026-10-23 — Added **Remove project** to the desktop app. It requires the session to be paused, removes only the private registry entry, stops live preview and leaves the project directory and existing agent configuration untouched. Added a focused registry test covering the project directory and other registrations.
- 2026-10-23 — Removed the previously registered `Automation` project from Andromeda by moving its registry entry to Trash. The project folder was not changed.
- 2026-10-23 — Stopped the test emulator, desktop app and ADB server, and removed the temporary `.scratch` artifacts and benchmark temp folder after the runs.
- 2026-10-23 — Attempted physical rotation verification; OxygenOS ignored `wm user-rotation lock 1`, so the test was inconclusive. Restored rotation lock 0 and pressed Home afterward.
- 2026-10-23 — Updated the desktop guide, implementation ledger, benchmark README and this log with the streaming, preview, launcher, removal and benchmark evidence.
- 2026-10-23 — Added focused controller tests for timed swipe/pause ordering, zero-distance long press and transport-failure recovery. The suite now proves that a swipe duration reaches Android, a pause does not emit an input command, the next action follows in order, and a failed ADB connection does not poison the next request.
- 2026-10-23 — Added `docs/examples.md` with observation-only, Settings, batch/pause, wait/pixel and companion-gesture examples, and linked it from the README and installation guide.
- 2026-10-23 — Added `docs/licenses.md` with a dated Rust/npm license inventory and release obligations. The inventory found no GPL/AGPL/SSPL or unknown licenses; two `r-efi` records offer LGPL as an alternative and resolve to MIT/Apache-2.0. MPL-2.0, CC-BY-4.0, PSF-2.0 and BlueOak notices still need a third-party notices bundle at release time.
- 2026-10-23 — Audited the stale implementation ledger against the code and tests, marked the implemented input/gesture/batch/accessibility/rotation/reconnect/approval and cross-platform CI gates complete, and added a detailed remaining-gates section with the exact human, device, model and release blockers.
- 2026-10-23 — Reconnected the OnePlus CPH2653 and completed physical landscape/rotation verification with `settings put system accelerometer_rotation 0` plus `user_rotation 1`: UI rotation 1, 2376×1080 bounds. Restored both settings to 0 and returned the phone to portrait.
- 2026-10-23 — Completed the physical session-timer flow: selected 15 minutes, started a session (`Session active · 15:00 left`), stopped it (`Computer paired · session stopped`) and restored `Session timer · Off`. Start/stop and Settings navigation work on the final build.
- 2026-10-23 — Re-enabled TalkBack on the physical phone. TalkBack and Andromeda both bound; focus rings appeared on Settings and Stop; a tap on Start session started a session. Full focus order, labels, double-tap activation and permission/pairing traversal remain for a human pass; the TalkBack first-run hint overlay and synthetic key events are not equivalent.
- 2026-10-23 — Diagnosed the intermittent “Permission set. Connecting…” state: `uiautomator dump`/UiAutomation suppresses accessibility services, and the old accept thread kept the abstract socket, so a replacement service instance hit `Address already in use`. Hardened `ControlService` to bind/publish atomically, retry transient binds, avoid clearing a healthy endpoint, and wake the blocked accept on teardown. The final debug build returns to “All set. You lead.” after suppression; debug, release and debugAndroidTest APKs build.
- 2026-10-23 — Added `docs/evidence/physical-ui-2026-10-23.json` and updated the ledger with the physical results and the remaining TalkBack limitation.
- 2026-10-23 — Removed the temporary screenshots/logs from `/tmp`, confirmed no Andromeda/desktop process remains, and stopped the ADB server. The phone was left with Andromeda-only accessibility, portrait rotation 0/0, timer Off and no active session.
- 2026-10-23 — Fixed companion UI text centering. Step-number badges are now flex-centered views sized from the reported font size, and single-line labels use Lynx's `text-single-line-vertical-align="center"`. Emulator pixel checks put all three badge numbers within 1.5 px of their circle centers and the primary and Stop labels on center at both 1.0× and 2.0× font scale; the full emulator smoke passed on the rebuilt debug APK.

## Checks run

- `cargo fmt --all --check`
- `cargo clippy --locked --all-targets -- -D warnings`
- `cargo test --locked --all-targets`
- `cargo clippy --locked -p andromeda-desktop --all-targets -- -D warnings`
- `cargo test --locked -p andromeda-desktop`
- `cargo test --locked --test project_scope`
- `npm run format:check --prefix desktop`
- `npm run build --prefix desktop`
- `npm run tauri build --prefix desktop -- --debug`
- `node npm/launcher.mjs --print-path` (resolved the rebuilt `Andromeda.app` executable)
- `npm run test:launcher`
- `npm run format:check --prefix android-companion/ui`
- `npm run build --prefix android-companion/ui`
- `./gradlew :app:assembleDebug :app:assembleRelease :app:assembleDebugAndroidTest`
- `android-companion/ci/emulator-smoke.sh` (with `ANDROID_SERIAL=emulator-5554`)

## Still open

- Claude Code end-to-end acceptance; the client's weekly usage limit stopped the session before tool calls. The reported reset is 2026-10-24 03:00 Asia/Manila.
- Physical TalkBack full traversal, labels, double-tap activation and permission/pairing screen check. Landscape/rotation and the session timer passed on the OnePlus; see `docs/evidence/physical-ui-2026-10-23.json`.
- Gesture-accuracy, completion/recovery and model/tool-overhead benchmarks.
- Representative open-source comparisons under matched conditions.
- Release packaging, third-party notices, published npm platform packages and release artifact verification.
- PR review/merge, release publishing and cleanup gates listed in `docs/implementation-status.md`.

No host environment variables or shell profiles were changed. No commit or push was made.
