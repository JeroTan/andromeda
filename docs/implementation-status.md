# Implementation and release evidence

The original requirements are in [intial_prompt.md](intial_prompt.md). This ledger records implementation and dated acceptance results. See the [current release audit](release-audit-0.1.1.md) for shipped artifacts, evidence and remaining validation limits. Historical pending items below may be resolved by later entries; unchecked verification is not a claim of a pass. A passing mock test does not prove Android behavior or client compatibility.

## v0.1.3 publication and cleanup — 2026-10-25

Published the [Mac/Android v0.1.3 preview](https://github.com/JeroTan/andromeda/releases/tag/v0.1.3) at `a62f067`. All four uploaded asset hashes match the local files; see [publication evidence](evidence/release-publication-0.1.3.json). Windows v0.1.2 remains the previous preview. The Mac DMG contains the current signed bundle and five executable tool examples; the Android APK uses versionCode 4 and the retained release key.

Temporary emulator, build caches, interface dependencies, test state and screenshots were removed. Final downloads and signing material were preserved; see [cleanup evidence](evidence/release-cleanup-0.1.3.json). The user chose to install the final Mac update after the current OpenCode run finishes. At publication time the core-fixed Mac app stays open, and ZZZ already uses direct schemas. Android installation and its battery approval remain owner actions on the Xiaomi device. No full OEM/Doze or model-accuracy certification is claimed.

## Background connection and tool guidance — 2026-10-25

Added Android Settings → Background connection, automatic exemption/restriction checks, an owner-approved exemption request with fallbacks, and a home link while attention may be needed. Already-exempt devices do not reopen the request. Xiaomi-specific controls are explained without claiming automatic detection or changes. Optional companion status fields expose battery state. Emulator optimized/exempt settings flows and a five-second background switch passed; long Doze/OEM survival and physical Xiaomi behavior remain unverified.

MCP descriptions and initialization instructions now distinguish taps/swipes/holds, real conditions versus batch pauses, coordinate scaling and error recovery. Five executable schema examples pass the public call boundary. OpenCode defaults to direct schemas (`codemode: false`), and the existing ZZZ connection was updated without changing its command or scope. No model-accuracy improvement is claimed. See [evidence](evidence/background-and-tool-guidance-2026-10-25.json).

## Core MCP startup/update recovery — 2026-10-25

Fixed the underlying macOS executable/bundle update path: fresh-inode staging, Tauri ad-hoc signatures, verified whole-bundle installation and rollback. A bundled-server startup/version preflight now runs before client settings are written. OpenCode connection setup applies the reviewed runtime entry in the selected project and verifies the actual connection, while preserving disabled/auth-required states and unrelated sessions.

Native regression checks cover the old inode-overwrite failure, repeated installs, invalid candidates, post-swap rollback and executable-ancestor symlinks. An actual OpenCode 2.1.4 fixture went from process-exited failure to connected through the desktop flow. The updated Mac app passes full code-signature verification; Claude/OpenCode are connected for ZZZ and its Running state was restored. See [core recovery evidence](evidence/mcp-core-recovery-2026-10-25.json). Published 0.1.2 downloads remain unchanged; these corrections are in the local/source build.

## OpenCode connector — 2026-10-24

The desktop now includes **Connect OpenCode** for OpenCode 2. It previews and saves project-local JSON/JSONC configuration, retaining comments, other settings and server entries. Replacement and concurrent-edit guards apply, and the managed connection keeps the existing directory/device/tool policy. OpenCode 1's legacy MCP map requires migration rather than silent rewriting.

The native Mac button/preview/save/focus flow passed in an isolated project. The installed OpenCode 2.0.22 client reported Andromeda connected using that saved configuration, with no model calls or phone actions. Public configuration tests, the Rust workspace suite, strict Clippy and frontend/build checks passed. See [connector evidence](evidence/opencode-connector-2026-10-24.json) and the [OpenCode guide](opencode.md). The local Mac app was updated and its Connect OpenCode button verified, preserving both registered projects in Stopped state. Published 0.1.2 release assets predate this addition.

## Preview acceptance closed — 2026-10-24

The operator explicitly deferred full human TalkBack navigation and broader desktop screen-reader coverage for this preview. Windows-native testing was already deferred. With that scope decision, the implementation, native APK/DMG/EXE delivery, publication verification and cleanup requirements for the preview are satisfied. Deferred checks remain unverified future validation; they are not relabeled as passes.

The release is [v0.1.2](https://github.com/JeroTan/andromeda/releases/tag/v0.1.2). Its [artifact evidence](evidence/native-release-0.1.2.json), [publication receipt](evidence/release-publication-0.1.2.json), [cleanup receipt](evidence/release-cleanup-0.1.2.json) and [requirement audit](release-audit-0.1.1.md) retain the verification and limits. Hosted Actions stays disabled. The repository remains private for the owner to publish.

## Published release and cleanup — 2026-10-24

[v0.1.1](https://github.com/JeroTan/andromeda/releases/tag/v0.1.1) is published with Android APK, Apple silicon DMG and Windows x64 EXE installers. All five uploaded assets, including the manifest and checksums, were matched against local SHA-256 hashes after publication. See [publication evidence](evidence/release-publication-0.1.1.json). The repository remains private until the owner changes its visibility.

Temporary compiler/toolchain outputs, dependency installations, old local release copies and private comparison imagery were removed. Installed apps, pairing, private release signing material and final v0.1.1 downloads were preserved. The task-installed MinGW, NSIS and 7-Zip packages were uninstalled without autoremove. See [cleanup evidence](evidence/release-cleanup-2026-10-24.json). GitHub Actions is still disabled with no active runs.

The adaptive-held-gesture investigation is [recorded separately](research/adaptive-held-gestures.md). Continuation is feasible at the Android API level; implementation remains planned as specified. No model-response timing guarantee is made.

## Desktop keyboard correction after 0.1.1 — 2026-10-24

Native inspection found that closing the asynchronously loaded instruction editor lost its triggering button: the next Tab returned to Add a project. Dialog entry points now remember their button before loading disables it, then restore focus after closing and completion. The native release build passed editor Escape/Save, removal Cancel and both client-preview Cancel paths in an isolated project. Scoped light/dark checks also ran at the minimum 780×620 window size. See [keyboard evidence](evidence/desktop-keyboard-2026-10-24.json).

The user's running project and installed app were left intact. Published 0.1.1 binaries do not contain this later fix; it belongs in the next build. These checks reduce the Mac keyboard-validation gap but do not establish full screen-reader coverage.

## 0.1.2 native packages — 2026-10-24

The keyboard-focus correction is packaged in updated Mac DMG and Windows EXE previews, alongside Android APK versionCode 3. The Mac DMG checksum, version and 18-tool catalog passed; the Windows x64 payload and unchanged verified WebView2 loader were inspected; the APK is non-debuggable and verifies with the same release key. [Artifact evidence](evidence/native-release-0.1.2.json) records hashes and limits. External dependencies are unchanged; the packages reuse the collected original publisher notices with the reuse basis and new lock hashes recorded.

The installed user apps were preserved. Full human screen-reader/wider-platform checks remain unverified, and Windows execution remains deferred. The packages do not convert those gaps into passes.

## 0.1.2 publication and final cleanup — 2026-10-24

[v0.1.2](https://github.com/JeroTan/andromeda/releases/tag/v0.1.2) is published at `948eaac`. All five uploaded asset digests match the local files; see the [publication receipt](evidence/release-publication-0.1.2.json). PR #20 merged the packages and standalone-install guidance. Earlier feature/release branches are removed, and build outputs, temporary compiler installations and old local downloads were cleaned up again. Final packages, private signing material and installed user apps are preserved; see [cleanup receipt](evidence/release-cleanup-0.1.2.json).

The release delivery is verified. The operator subsequently deferred the remaining human accessibility checks for this preview, as recorded in the acceptance decision above. No human screen-reader pass is inferred from native accessibility-tree checks or preview publication.

## Screenshot acceleration — 2026-10-24

Fresh on-demand screenshots can use the paired Android accessibility API, with bounded on-device scaling/encoding and validated codec passthrough. Older or unavailable bridges retain ADB fallback; protected-content and invalid-data errors do not trigger another backend. Live preview retains its existing ADB capture loop.

Local verification passed 62 Rust workspace tests, strict all-feature Clippy, Android debug/release builds, and the macOS release app build. Standards and Spec reviews have no unresolved findings. The isolated emulator retained accessibility through the capability update, protected a secure test window, passed late-callback/disconnect recovery, and supplied valid lossless WebP. See [acceptance evidence](evidence/accessibility-capture-emulator-2026-10-24.json).

The [matched MCP benchmarks](../benchmarks/README.md#compare-capture-backends-through-mcp) recorded 240 successful emulator requests in forward/reverse order. Emulator JPEG width 720 improved from 169–171 ms median through ADB to 51–52 ms through the companion. [PR #6](https://github.com/JeroTan/andromeda/pull/6) merged this implementation as `0cdea93`.

After the operator unlocked the devices, the physical phone upgrade retained accessibility and pairing without regrant or re-pairing. The macOS release app was installed and the existing project restored to Running; both interfaces showed the same project state and separate Pause and Stop controls. Its installed managed MCP connection returned a companion image, which was visually inspected, and desktop preview displayed 94 ms in one acceptance observation. Physical forward/reverse benchmarks recorded another 240 successful requests: JPEG width 720 improved from 458–461 ms median through ADB to 73–80 ms through the companion. See [physical evidence](evidence/accessibility-capture-physical-2026-10-24.json). No game actions were performed. These checks close this device's capture-upgrade acceptance, not the remaining release gates. Hosted Actions remains disabled with no active runs.

## Current session flow — 2026-10-23 correction

The user superseded the earlier per-action approval/global Resume design with [seamless project sessions](specs/seamless-project-sessions.md). Start each configured project once; its saved allowed tools execute without further Andromeda approval prompts. Each project has independent Pause/Resume and Stop, and Stop all projects is the emergency control. Phone and desktop replicate project state over authenticated bounded heartbeat messages; the phone needs no second Start. Saved pairing is shown separately from actual connectivity. Earlier sections below describing per-request approval are historical and superseded by this correction.

Local verification passed the 52-test Rust workspace suite, all-feature Clippy, desktop/frontend/package builds, phone UI/APK builds and two review axes after fixes. Public MCP/ADB tests cover independent project cancellation, later Stop versus stale Resume, loss/reconnect fencing, and a concurrent Resume during a failed bridge sync. Physical checks verified desktop Start → phone Running, phone Pause/Resume → desktop state, desktop Stop → phone Stopped, and rejection of paused/stopped MCP input. One harmless Home-key request executed without approval. Phone Stop also survived a deliberately older Resume acknowledgment, and heartbeat loss paused the phone after 4.249 seconds. These are scoped acceptance results, not a general latency benchmark. See [evidence](evidence/seamless-sessions-2026-10-23.json). Hosted Actions stayed disabled; this does not close the remaining release gates.

## Agent-visible project state — 2026-10-24

Managed device information now reports the requesting project's `project_state` and combines that state with its input policy and global control state for `control_enabled`. A paused project previously reported true when another project was running, even though its input was correctly rejected. The public MCP regression failed before the fix and passes after it. Text and structured results agree; the tool catalog also explains that managed Stop ends the project session. The full Rust suite, strict all-feature Clippy, release app build and both review axes pass. Installed macOS/physical-phone checks verified Stopped/Running/Paused responses, then restored Running and its on-demand preview. See [evidence](evidence/project-control-status-2026-10-24.json).

## Installer preparation and full timer check — 2026-10-24

The operator selected `@jrw/andromeda` and clarified the native deliverables: Lynx phone APK, macOS DMG and Windows EXE installer. The local installer command now selects DMG/NSIS EXE/Debian per host. The manual-only workflow targets DMG/NSIS too; Actions remains disabled. A macOS DMG built successfully, passed image checksum verification, mounted read-only with the app and Applications link, and exposed all 18 tools through its bundled CLI. The corresponding npm platform package installed offline and cold-launched the same app bytes with isolated state; the normal app/project was restored afterward. See [package evidence](evidence/release-packaging-2026-10-24.json) and [build commands](release-packaging.md). Windows EXE execution, Linux validation, signing, complete installer notices, authenticated npm publication and the final release remain open.

The notice collector now retains 304 macOS dependency records and 193 distinct full text files, checks harvested Cargo/npm source against verified archive bytes, and preserves exact-revision overrides. Tampered Cargo/npm notice files were independently rejected and restored. Ten macOS review gaps remain; Android/Lynx mapping and MPL source provision remain open. Staged packages are explicitly private previews with `ready_to_publish: false`.

The full 15-minute emulator timer check passed with the real clock and authenticated desktop heartbeats. Desktop Stop arrived after 902.16 seconds, and subsequent input returned `project_stopped`. The emulator Settings selection was restored to Off. A follow-up Off-timer rejection verified that the corrected probe removes its temporary project registration and does not write a false expiry report. This closes the elapsed-duration emulator check, not physical power-management or all timer durations. See [timer evidence](evidence/session-timer-expiry-2026-10-24.json).

## Native preview release direction — 2026-10-24

The operator explicitly deferred Windows-native testing and requested that current artifacts be finished without further testing expansion. npm publication is optional and no longer a release requirement. Current native deliverables are the Apple silicon Mac DMG and signed Android release APK; Windows EXE/Linux binaries remain pending and are not represented as shipped. macOS Developer ID signing/notarization is unavailable on this host, so the current Mac package is an unsigned preview.

Both native packages now contain collected original publisher notices, preserved license-by-reference statements, source associations and five corresponding MPL crate source archives. Exact Lynx source-JAR notices and PrimJS release notices supplement the earlier collection. Unresolved source-revision associations remain documented qualifications rather than invented prohibitions. The Android release APK is non-debuggable and has a dedicated release certificate; private signing material is stored outside the repository. Existing debug installs are not overwritten or uninstalled by this packaging work.

The [v0.1.0 native preview](https://github.com/JeroTan/andromeda/releases/tag/v0.1.0) was published at commit `966dcd3`; all four uploaded asset hashes match local files. See [publication evidence](evidence/release-publication-0.1.0.json). Artifact evidence is in [native-release-0.1.0.json](evidence/native-release-0.1.0.json). Wider platform/human accessibility verification remains unproven. This preview publication does not require an npm login.

## 0.1.1 native packages — 2026-10-24

The Windows x64 NSIS installer is now built, alongside the updated Apple silicon DMG and signed Android APK. The Windows payload was inspected without executing it: x64 desktop/MCP binaries, matching WebView2 loader and notices are present; the earlier cross-build's leftover Mac executable is excluded. Native Windows testing remains deferred as requested. The build uses a matching official Rust 1.99 compiler/stdlib with MinGW-w64 and NSIS; the Homebrew compiler could not consume the official prebuilt stdlib, so the isolated matching toolchain was used.

This patch includes the Android icon, visible Save action for unsaved access edits, and desktop-owned Claude CLI setup/result feedback. The actual Claude registration reports Connected; already-open chat tool catalogs still require their client reconnect/reopen step. Android versionCode is 2 and the release certificate matches 0.1.0. Artifact evidence is [native-release-0.1.1.json](evidence/native-release-0.1.1.json). Unsigned desktop status, native Windows testing and remaining broader validation qualifications are retained; this is a preview, not an unqualified compatibility claim.

## Phase 1: Core Android MCP server

- [x] Rust library and executable; Tokio and rmcp stdio transport
- [x] Device discovery, explicit selection, device information
- [x] Screenshot capture, touch, swipe, keys, text
- [x] Real Android setting changed and verified through Claude Code
- [x] Real Android setting changed and verified through Codex

## Phase 2: Advanced interaction

- [x] Timed tap and swipe sequences (swipe duration and explicit `pause` actions)
- [x] True concurrent pointers through optional companion on emulator and physical phone
- [x] Batches, stop, deadlines, coordinate/timing validation
- [x] Gesture integration tests on supported physical devices (two concurrent pointers and Unicode text)

## Phase 3: Perception and reliability

- [x] Accessibility trees and UI/visual waiting
- [x] Explicit persistent screen streaming (desktop live preview over one shell-v2 capture loop; MCP remains request/response)
- [x] Screenshot resizing/compression (PNG/JPEG/WebP/auto and optional AVIF, bounded dimensions and quality)
- [x] Rotation rejection and disconnect/reconnect behavior
- [x] Cancellation, action logs, permissions and human approval controls
- [x] Companion APK build and emulator permission/UI validation (physical evidence recorded below; release checks remain open)

## Phase 4: Performance

- [x] Reproducible capture, persistent-stream and Home-key input harness with environment metadata
- [x] Screenshot/input p50/p95, stream frame rate and host CPU/RSS on an emulator and a physical phone
- [x] Gesture endpoint accuracy, deterministic completion/recovery, real-client overhead and measured capture optimization (scoped evidence below)
- [x] Two representative open-source alternatives compared on matched native-PNG output (modes/limits recorded)
- [x] Publish raw measurements and limitations without unsupported speed claims

## Phase 5: Open-source release

- [x] Windows, macOS and Linux build/test evidence for the core and desktop stages
- [x] Installation, Claude Code, Codex, Cursor and API guides
- [x] Example prompts, demonstrations, contribution and security policies
- [x] License and dependency/license review
- [x] Historical GitHub CI evidence on Windows, macOS and Linux (core/desktop); Actions now disabled and manual-only
- [x] APK, DMG and EXE preview packaging, bundled notices and uploaded artifact hash verification
- [x] Reviewed feature PRs merged to main with conventional titles
- [x] v0.1.0 native preview published with operator-deferred checks and remaining platform qualifications
- [x] Completed feature/release branches, temporary build artifacts and task processes cleaned up (final receipts retained)

## Agreed test boundaries

1. Public MCP: schemas, structured results, errors, request cancellation and stop.
2. Public device API over a simulated ADB server: framing, failures, rotation and recovery.
3. Companion protocol: authentication, gestures and permission behavior.

The user approved these boundaries on 2026-10-22 and will connect a test phone. No phone was connected at initial inspection. An Android SDK and an Android 37 emulator image are available locally.

## Evidence collected on 2026-10-22

- Native macOS arm64 debug and release builds pass. Formatting and strict Clippy checks pass.
- Public-boundary tests cover fragmented ADB discovery, shell exit failures, control disabled by default, batch prevalidation, rotation rejection, password-node redaction, deadlines, stop/recovery, MCP negotiation, structured argument errors and concurrent MCP stop.
- Physical device: OnePlus CPH2653, Android 16/API 36, USB, 1080×2376 portrait. Discovery, device information, Settings launch, native PNG/scaled JPEG capture and accessible UI inspection work.
- This device does not emit hierarchy XML when `uiautomator dump` targets `/dev/tty`, `/dev/stdout` or `/proc/self/fd/1`. A private device temporary file with cleanup traps works; the server uses that path.
- Codex CLI acceptance passed through the client's approval-review flow: observe Dark mode, tap Light mode, observe selected Light mode, tap Dark mode, observe selected Dark mode. Nine Andromeda calls completed. All three theme-state images were inspected independently. No other settings were changed.
- The first Codex attempt successfully observed the phone but rejected input because that test session could not request approval. The subsequent test used the client's supported approval-review flow.
- Claude Code 2.1.299 is authenticated and was invoked with only the test MCP server. Its weekly usage limit stopped execution before tool calls. The client reported reset on 2026-10-24 at 03:00 Asia/Manila. The Claude acceptance gate remains open.

Private device screenshots and raw client logs are temporary test artifacts, excluded from Git. They are not public demonstration assets or reproducible performance measurements.

## Current work

The core was merged through [PR #1](https://github.com/JeroTan/andromeda/pull/1) as `1ca97be`, after CI passed at `ee1a830` on Windows, macOS, Linux and Rust 1.88. The `feat/android-mcp` branch was deleted locally and remotely. The desktop stage was subsequently merged through PR #2. [PR #3](https://github.com/JeroTan/andromeda/pull/3) merged the phone/capture/desktop follow-up as `d0efc43`; the feature branches were removed locally and remotely where present.

Repository and instruction files were inspected; no prior implementation, coding standards, issues or PRs needed reconciliation. Agent instruction files were empty. The triage skill's issue-label workflow does not apply to the supplied specification.

Main now contains 18 tools. PR #3 added `adm_multi_touch` and `adm_get_long_view`, Unicode input, pairing, the phone UI, persistent capture and compressed output. Both review axes found preview/launcher defects; the fixes passed their follow-up reviews. Local validation is recorded below. Matched open-source comparisons, remaining client/device checks and final release packaging remain incomplete; desktop additions retain platform/UX gates. The overall release scope is not complete.

## User additions and workflow preferences

On 2026-10-22 the user clarified that Andromeda's main interface should be a desktop app on Mac/PC. It should create MCP configurations for chats, scope access to selected project directories, read a project instruction file, and offer a simple npm launch command or downloadable executable. The Android app is an optional permission/gesture bridge, not the main product interface.

- [x] Rust desktop setup/control app with device selection, preview and stop controls
- [x] Project registration, allowed-tool policy, instruction-file loading and startup scope checks
- [x] Reviewable project-local Codex/Claude configuration installation preserving existing settings
- [x] Simple npm launcher (resolves local and platform builds; published platform packages remain release work)
- [ ] Remaining platform packages/signing beyond the current unsigned Mac preview (Windows testing deferred by operator)
- [ ] Desktop UI/accessibility and project isolation verification on Windows and Linux (macOS evidence exists)
- [x] Shared visual identity based on the supplied editorial reference: bold typography, rounded panels, thin borders, pastel accents and original celestial graphics
- [ ] Polished phone permission/pairing/session UI with accessibility, text scaling, landscape and light/dark verification
- [x] Lynx/TypeScript UI evaluation documented; Rust remains the control and authorization core

The user asked to move full automated suites to GitHub PR workflows. Run only focused checks/debugging and hardware-dependent tests locally. Android Studio and its emulator are authorized for testing.

The companion source is in `android-companion/`. Its ReactLynx UI bundle and debug/release APKs build with AGP 9.4.1, Gradle 9.7.1 and Android API 37. An isolated Android 17 emulator completed pairing, a user-started session, two-pointer gesture and Unicode text checks. See [companion acceptance evidence](evidence/companion-acceptance-2026-10-22.json). The physical OnePlus CPH2653 is connected; physical gesture/input acceptance passed on 2026-10-23. Session start/stop, the 15-minute timer, landscape/rotation and partial TalkBack checks were re-run on the final build; see [physical UI evidence](evidence/physical-ui-2026-10-23.json). Full TalkBack traversal remains open.

The UI runtime spike selected Tauri 2 for desktop and ReactLynx for Android; see [runtime research](research/ui-runtime.md) and [Android integration research](research/android-lynx.md). The Lynx desktop spike rendered the requested visual direction but did not meet the observed desktop accessibility/Linux-window gates. Its app process was closed after inspection.

Project registry/configuration work now has focused tests for directory scope, disabled projects, instruction symlink rejection, preservation of existing Claude/Codex settings, stale-preview rejection and configuration path escapes. A shared controller can isolate project stop operations while retaining an operator-wide stop. The authenticated managed gateway and Tauri interface now integrate these capabilities. Focused tests also cover per-request approval, cancellation of pending approvals, operator pause, and concurrent instruction-file edits. The packaged macOS preview launches with local assets and exposes its application controls in the native accessibility tree. Project selection, starter-instruction creation, both themes and Codex/Claude configuration saving have been exercised in an isolated test workspace. The generated bundled MCP command initializes, loads the selected instructions, discovers devices, rejects an unrelated working directory and fails after app closure. See [desktop acceptance evidence](evidence/desktop-acceptance-2026-10-22.json). Cross-platform packaging, remaining UI checks and PR review are pending.

PR #2 is open for the desktop stage. Its first workflow exposed current-Clippy and Windows line-ending issues; these were fixed. The next run built the macOS package and passed the core checks on macOS/Linux/Rust 1.88. GitHub rejected preview artifact upload because account-level Actions artifact storage is exhausted. Package builds and tests remain required; preview uploads report availability separately. Release artifact verification remains a required gate.

At code head `151cd43`, [PR CI run 39759622974](https://github.com/JeroTan/andromeda/actions/runs/39759622974) passed all seven jobs: core tests/builds on Windows, macOS, Linux and Rust 1.88, plus macOS app/Windows MSI/Linux deb desktop packages. All three artifact uploads succeeded on this run. Both review axes have no unresolved findings after fixes; the packaged macOS UI also shows an actual MCP client connecting and disconnecting. Native Windows/Linux UI, full accessibility/scaling and physical preview/input acceptance remain open.

[PR #2](https://github.com/JeroTan/andromeda/pull/2) merged as `5b3f8cd` after all seven jobs passed at `7ba7c17` in run 39762860792. Its feature branch was deleted locally and remotely, and the acceptance app process was closed. Phone bridge implementation continues on `feat/android-companion`; no release has been published.

Phone PR #3 originally opened before the follow-up integration. Its initial Android setup failed because the action requested the removed `tools` SDK package. The workflow now installs `platform-tools` explicitly. A listener lifecycle race found during standards review was fixed with coordinated socket ownership and final cleanup; five emulator disable/re-enable cycles passed through the public ADB companion protocol. The reviewer agents exhausted their account allowance before completing their final reports, so final review remains open.

Physical acceptance on 2026-10-23 passed on the connected OnePlus: paired after the owner granted accessibility, dispatched two concurrent pointers for 450 ms, inserted exact Unicode text, cleared the test text and stopped the session. Subsequent companion input returned `session_inactive`. See [physical evidence](evidence/physical-companion-2026-10-23.json). Physical session timer, landscape/rotation and partial TalkBack checks were repeated on the final build; see [physical UI evidence](evidence/physical-ui-2026-10-23.json). Both review agents resumed after reset and verified their fixes; see [companion review](evidence/companion-review.md). CI remains pending.

## Evidence collected on 2026-10-23

- The existing Android 17/API 37 AVD passed the full companion smoke flow without wiping data: TalkBack permission entry, session timer Off/15/30/60 persistence, new-process timer restoration, large text, dark mode, five accessibility service disable/re-enable cycles and the permission-vs-service connecting state. The session timer defaults to Off and applies to the next session.
- The API 37 emulator's `UiAutomation` clears `enabled_accessibility_services` while suppressing services. The instrumentation now uses a debug-only service-disconnect hook and asserts the visible “Permission set. Connecting…” state, so the check no longer becomes a permission-off test.
- The capture harness completed 20 samples per workload with zero failures on the emulator (battery 100%, 25.0 °C, emulator transport): direct ADB PNG p50 412 ms / p95 440 ms; external ADB CLI p50 416 ms / p95 420 ms; controller native PNG p50 442 ms / p95 491 ms; controller 720px JPEG p50 454 ms / p95 460 ms. Raw result: [emulator benchmark](../benchmarks/results/emulator-api37-2026-10-23.json). These are emulator measurements, not product comparisons or release performance claims.
- The original capture harness covered screenshot transport and image processing only. The schema-v2 additions below add input, persistent-stream and host CPU/RSS coverage. Gesture accuracy, completion/recovery, client/tool overhead and matched open-source comparisons remain open.
- Persistent capture is implemented over one ADB shell-v2 connection (`src/device/stream.rs`). Complete PNG frames are parsed by chunk structure with frame/buffer bounds and timeout/error handling. Desktop **Start live preview** consumes that stream, emits explicit preview frames, and stops on error, device/project changes or window close. The preview is never saved to disk. Transport framing, oversized-chunk and timeout tests pass; the desktop preview loop test passes.
- The benchmark harness now uses schema version 2 and adds `persistent_stream_png` plus opt-in `controller_press_home`, with host CPU/RSS sampling. A release emulator run completed 20 samples per workload with zero failures: direct ADB PNG p50 413 ms / p95 437 ms; external ADB CLI 423/442; controller native 462/486; controller 720px JPEG 482/515; persistent stream 526/543 ms at 1.89 fps; Home input 20.8/43.3 ms. Raw result: [emulator benchmark](../benchmarks/results/emulator-api37-2026-10-23-stream.json).
- A release physical run on the OnePlus CPH2653 (Android 16/API 36, USB, battery 95%, 37.0 °C) also completed 20 samples per workload with zero failures: direct ADB PNG p50 1812 ms / p95 2049 ms; external ADB CLI 1864/1982; controller native 1957/2112; controller 720px JPEG 1995/2034; persistent stream 1937/1959 ms at 0.52 fps; Home input 29.3/36.0 ms. Raw result: [physical benchmark](../benchmarks/results/oneplus-cph2653-2026-10-23.json). These are device-specific baselines, not product-comparison claims.
- The npm launcher resolves an explicit desktop executable, a macOS app bundle, a platform package, or a local Tauri build; it preserves the desktop exit status and has five passing tests. CI now runs `npm run test:launcher`. Published platform packages and release artifact verification remain release work.
- The desktop project list now supports **Remove project**. Removal requires the session to be paused, deletes only the private registry entry, stops live preview and leaves project files and existing agent configuration untouched. A focused registry test verifies that other projects and the project directory survive.
- Physical session timer/start-stop passed on 2026-10-23: 15 minutes selected, `Session active · 15:00 left` observed, Stop returned to `Computer paired · session stopped`, and the timer was restored to Off. The 15-minute expiry itself was not waited out on device; emulator smoke verifies timer choices and persistence, not elapsed-time expiry. Actual expiry remains unverified.
- Physical TalkBack check on 2026-10-23: TalkBack and Andromeda both bound; focus rings appeared on Settings and Stop; a tap on Start session started a session while TalkBack was enabled. Full focus order, labels, double-tap activation and permission/pairing traversal remain open; the first-run TalkBack hint overlay and synthetic key events are not a human substitute.
- Service lifecycle hardening: `ControlService` now binds/publishes the endpoint atomically, retries a transient `Address already in use`, and wakes a blocked `accept()` during teardown. After `uiautomator dump`/UiAutomation suppresses accessibility, the final debug build returns to `All set. You lead.` without restarting the app. Debug, release and debugAndroidTest APKs build.
- Physical landscape/rotation passed on 2026-10-23 using `settings put system accelerometer_rotation 0` plus `settings put system user_rotation 1`: the UI reported rotation 1 and 2376×1080 bounds. The phone was restored to `accelerometer_rotation=0`, `user_rotation=0` and portrait. The earlier `wm user-rotation lock 1` attempt was inconclusive; use the settings method.
- The earlier emulator benchmark in this section predates the schema-v2 stream/input run. The newer emulator result above supersedes it for stream and input measurements; the older result remains as the original four-workload baseline.
- The controller suite now includes an explicit timed-swipe/pause ordering test, a zero-distance long-press command test and a transport-failure/reconnect test. They pass with the existing rotation, batch-validation, deadline, stop/recovery, UI-tree and scope tests.
- Added [examples](examples.md) for observation-only, Settings, batch/pause, wait/pixel and companion gesture prompts, and [license review](licenses.md) covering the Rust workspace and both npm lockfiles. The release gate still requires a generated third-party notices bundle.

## Remaining gates in detail

These are the concrete items behind the unchecked boxes. Items marked **blocked** need a human, a device, credentials or a release decision; they are not missing controller code.

### 1. Claude Code acceptance — passed on 2026-10-24

- Claude Code 2.1.299 completed the actual Settings observe/toggle/restore flow over standalone MCP with explicit ordinary input authorization. Independent Android reads verified Dark (2) → Light (1) → Dark (2); harness restoration was unnecessary. The harness returned to Andromeda afterward. See [client evidence](evidence/claude-acceptance-2026-10-24.json).
- The stream recorded eight attempted tool calls: four screenshots, two successful theme taps and two client-denied optional pause/wait calls. The agent recovered using screenshots. The denials came from the intentionally narrow Claude allowlist, not Andromeda approval; they remain in the evidence. No human confirmation was requested during this run.
- Client elapsed time was 19.85 seconds; harness wall time was 21.59 seconds. Usage and per-call spans are retained, including null touch timings that this parser did not capture. The client's API duration includes more than pure thinking; this one run does not provide an exact transport/model decomposition or a benchmark distribution.

### 2. Physical phone accessibility — needs a human TalkBack pass

- **TalkBack traversal:** partially verified on the OnePlus CPH2653: TalkBack and Andromeda both bound, focus rings appeared on Settings and Stop, and Start session started a session while TalkBack was enabled. A human pass is still required for full focus order, labels, double-tap activation and permission/pairing screen traversal; the first-run TalkBack hint overlay and synthetic key events do not reproduce that path.
- **Landscape:** passed on the OnePlus CPH2653 on 2026-10-23; rotation 1 produced 2376×1080 bounds and the companion UI remained usable. The phone was restored to portrait 0/0. Focus-order verification in landscape still needs the same human TalkBack pass.
- **Rotation rejection:** covered by API-level tests. Physical rotation now uses `settings put system accelerometer_rotation 0` plus `settings put system user_rotation 1`; this produced rotation 1 and 2376×1080 bounds, and both settings were restored to 0. Do not leave rotation or accessibility settings changed.

### 3. Gesture accuracy, completion and recovery — scoped measurements recorded

- The native debug pad verified 48/48 gesture endpoint samples on both the emulator and physical OnePlus, including taps, swipes, two simultaneous fingers and screenshot-to-action sequences. Maximum endpoint error was 0 px; physical ADB swipe-duration error reached 11 ms. Six deterministic dialog completion/Back-recovery tasks passed per device without input retries. Actual Android event traces and observation-retry counts are retained in [benchmarks](../benchmarks/README.md#native-gesture-endpoints-and-deterministic-recovery).
- These controlled fixtures measure endpoint arrival and deterministic recovery, not arbitrary-app success, full intermediate trajectories or other pointer counts. The separate Claude Settings toggle/restore run provides real-client evidence; model behavior remains separate from transport timings.

### 4. Model/tool overhead — one real client run recorded

- The Claude Settings run above records client token/cache usage, eight attempted tool calls, errors, per-call spans and end-to-end time. Its capture requests took 69–110 ms inside the server; overall task time remained about 22 seconds. Null touch timing and client/API boundaries prevent an exact computation-versus-transport split. More tasks/repeated runs are needed for a distribution.
- Do not compare these numbers to another product unless the model, prompt, device, transport, thermal state and task are matched.

### 5. Representative open-source comparisons — native-PNG comparison recorded

- Pinned Mobile Next 1.0.12 and MetaMask 0.5.0 were compared with Andromeda on the same physical USB phone, static screen and native PNG dimensions. All 160 capture samples and 80 Home-acknowledgement samples succeeded. Andromeda companion capture measured 175–196 ms median; its forced ADB path 372–384 ms, MetaMask default ADB 435–441 ms and Mobile Next explicitly selected ADB compatibility mode 500–505 ms. See [raw results and limitations](../benchmarks/README.md#matched-open-source-native-png-comparison).
- Mobile Next default JPEG/mobilecli performance was not measured. This is a specific output-matched comparison, not a default-product ranking. Input had one ordering, child-process resource use is excluded, and no fastest-product or general reliability claim is established.

### 6. Historical release packaging gates — superseded by the published-release entries

- Build release artifacts for the CLI, desktop app and companion APK on Windows, macOS and Linux; include the project licenses and a generated third-party notices bundle (see [licenses](licenses.md)).
- Produce checksums and verify the npm launcher against an installed platform package, not only a local Tauri build.
- macOS signing/notarization, Windows signing and any store packaging are release decisions. The installed macOS build is release-optimized, and a DMG candidate is verified, but this is not a signed/notarized public release.
- Published npm platform packages, GitHub release assets and `v0.1.0` publication remain open.

### 7. Feature integration — merged

- PRs #1–#3 are merged to main. The phone/capture/desktop integration merged at `d0efc43` after local validation and both review axes; release acceptance remains separate.
- Hosted Actions is disabled by the operator. Re-enabling or rerunning it requires a new explicit instruction. The earlier Windows-fixture and Android-emulator failures remain documented; they are not reported as remote passes.

### 8. Historical cleanup state — superseded by the cleanup receipt

- Task-owned desktop preview/gateway processes were closed, temporary game captures and the failed video harness were removed, and completed feature branches were deleted.
- The operator restart cleared both native test recorder processes; verification found zero remaining. That native recorder path is excluded from product code and further physical experiments.
- Keep the rebuilt preview package, development build caches and the small private codec reference while release validation is unfinished. The shared ADB server remains available for the user's Android tools.


## Continuation on 2026-10-23

- Preserved the other agent's uncommitted streaming, launcher, UI, tests and benchmark work, and read its handoff log before continuing.
- Added `adm_get_long_view`: explicit scrolling viewport, up to eight bounded page observations, settle delay, repeated-viewport stop, geometry checks, input authorization/approval, cancellation and Stop. Tiles can overlap; list inertia can skip content. This is a bounded scan, not verified complete coverage or a seamless stitch. Reobserve before using coordinates from historical pages.
- Existing tap/swipe batches already support a shared gap and per-step `pause` actions. Adaptive updates to a held gesture remain planned; model/subagent response time cannot provide precise gesture timing.
- Added lossy/lossless WebP, automatic compatible byte-size selection and opt-in AVIF. Public-MCP tests cover decoded dimensions/pixels, MIME, smallest compatible candidate, and unavailable optional encoder. The default remains PNG; AVIF is never chosen automatically.
- Added capture/geometry/image-processing timing fields so device time can be separated from encoding. MCP/client upload and model time still require a dedicated client trace.
- Verified the long-view tool on the physical game's skill list, and used bounded existing batches to continue the unfinished career. Detailed daily completion is recorded separately after verifying the remaining mission state.
- Corrected the handoff's expiry claim: the existing smoke checks timer selection/persistence and manual Stop, not a full elapsed-time expiry.
- These changes and the inherited changes still require full PR CI and review; no release-completion claim is made.


## Local validation and CI policy on 2026-10-23

- The operator requested that hosted Actions stop because of costs. No queued/running runs remained at cancellation time; workflow `CI` was then confirmed `disabled_manually`. The YAML now has only `workflow_dispatch`. Re-enabling or running it requires a new explicit operator instruction.
- The last hosted run, 39910602068 at `216b19d`, passed launcher, all three desktop packages, and Rust on Linux stable/MSRV and macOS. Windows failed two codec tests because oversized debug fixtures exceeded tool deadlines; those fixtures now use a small image while retaining pixel/dimension/scaling assertions. Android failed during emulator boot, before the smoke script ran. Neither failure is relabeled as a hosted pass.
- Review fixes add a deadline covering stream startup, generation-fenced preview events, event registration before startup, cancellation checks before emission, fractional FPS and correct launcher passthrough flags. Stop also invalidates a superseded native startup.
- Local `cargo test --locked --workspace --all-targets` passes after these fixes, as do the optional AVIF checks, desktop build/Clippy and six launcher checks. Native Windows and the failed hosted emulator boot have not been rerun. GitHub Actions stays disabled.
- The user completed Umamusume dailies personally and ended that automation scope. No further game actions are authorized by that completed task.
- See [capture investigation](research/low-latency-capture.md) for measured limits and the proposed video backend. Native screenrecord did not produce usable frames and required an operator restart to clear two stalled test processes; the restart was verified. No low-latency success is claimed.

- Follow-up Standards and Spec reviews of `216b19d` → `60285e9` confirmed the preview/launcher fixes with no actionable findings. They were read-only reviews and did not run CI or phone actions.

- Packaged macOS startup was verified through its native accessibility interface: the rebuilt app opened with input paused and no registered projects, then was closed to stop its local gateway. The final global Stop/preview follow-up also passed both review axes without actionable findings.

- The rebuilt macOS bundle's MCP executable advertises all 18 tools, identifies scrolling capture as input, and documents automatic image encoding. Both final review axes found no regressions in the global Stop/preview change.
