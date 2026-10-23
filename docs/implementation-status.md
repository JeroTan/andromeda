# Implementation and release evidence

The original requirements are in [intial_prompt.md](intial_prompt.md). This ledger tracks the full scope; unchecked gates are unfinished. A passing mock test does not prove Android behavior or client compatibility.

## Phase 1: Core Android MCP server

- [x] Rust library and executable; Tokio and rmcp stdio transport
- [x] Device discovery, explicit selection, device information
- [x] Screenshot capture, touch, swipe, keys, text
- [ ] Real Android setting changed and verified through Claude Code
- [x] Real Android setting changed and verified through Codex

## Phase 2: Advanced interaction

- [x] Timed tap and swipe sequences (swipe duration and explicit `pause` actions)
- [x] True concurrent pointers through optional companion on emulator and physical phone
- [x] Batches, stop, deadlines, coordinate/timing validation
- [x] Gesture integration tests on supported physical devices (two concurrent pointers and Unicode text)

## Phase 3: Perception and reliability

- [x] Accessibility trees and UI/visual waiting
- [x] Explicit persistent screen streaming (desktop live preview over one shell-v2 capture loop; MCP remains request/response)
- [x] Screenshot resizing/compression (PNG/JPEG, bounded dimensions and JPEG quality)
- [x] Rotation rejection and disconnect/reconnect behavior
- [x] Cancellation, action logs, permissions and human approval controls
- [x] Companion APK build and emulator permission/UI validation (physical evidence recorded below; release checks remain open)

## Phase 4: Performance

- [x] Reproducible capture, persistent-stream and Home-key input harness with environment metadata
- [x] Screenshot/input p50/p95, stream frame rate and host CPU/RSS on an emulator and a physical phone
- [ ] Gesture accuracy, completion/recovery, model/tool overhead and further measured optimization
- [ ] Representative open-source alternatives compared under matching conditions
- [x] Publish raw measurements and limitations without unsupported speed claims

## Phase 5: Open-source release

- [x] Windows, macOS and Linux build/test evidence for the core and desktop stages
- [x] Installation, Claude Code, Codex, Cursor and API guides
- [x] Example prompts, demonstrations, contribution and security policies
- [x] License and dependency/license review
- [x] GitHub CI on Windows, macOS and Linux (core and desktop; companion PR run pending)
- [ ] Release packaging, third-party notices and artifact verification
- [ ] Reviewed PRs merged to main with conventional titles
- [ ] v0.1.0 release published after acceptance gates pass
- [ ] Temporary branches, build artifacts and processes cleaned up

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

The core was merged through [PR #1](https://github.com/JeroTan/andromeda/pull/1) as `1ca97be`, after CI passed at `ee1a830` on Windows, macOS, Linux and Rust 1.88. The `feat/android-mcp` branch was deleted locally and remotely. The desktop stage was subsequently merged through PR #2; current work is on `feat/android-companion`.

Repository and instruction files were inspected; no prior implementation, coding standards, issues or PRs needed reconciliation. Agent instruction files were empty. The triage skill's issue-label workflow does not apply to the supplied specification.

The merged core contains 16 tools. The companion branch adds `adm_multi_touch` as the seventeenth and integrates Unicode input, pairing and the phone UI. This branch is not yet reviewed or merged. Persistent capture and the schema-v2 benchmark harness are implemented; matched open-source comparisons, remaining client/device checks and the final release remain in scope and are not complete. Desktop additions have separate platform/UX gates. The full goal remains active.

## User additions and workflow preferences

On 2026-10-22 the user clarified that Andromeda's main interface should be a desktop app on Mac/PC. It should create MCP configurations for chats, scope access to selected project directories, read a project instruction file, and offer a simple npm launch command or downloadable executable. The Android app is an optional permission/gesture bridge, not the main product interface.

- [x] Rust desktop setup/control app with device selection, preview and stop controls
- [x] Project registration, allowed-tool policy, instruction-file loading and startup scope checks
- [x] Reviewable project-local Codex/Claude configuration installation preserving existing settings
- [x] Simple npm launcher (resolves local and platform builds; published platform packages remain release work)
- [ ] Downloadable, signed/notarized desktop packaging for every platform
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

Phone PR #3 is open at `b496246`. Core and desktop checks passed its first run; Android setup failed because its action requested the removed `tools` SDK package. The workflow now installs `platform-tools` explicitly. A listener lifecycle race found during standards review was fixed with coordinated socket ownership and final cleanup; five emulator disable/re-enable cycles passed through the public ADB companion protocol. The reviewer agents exhausted their account allowance before completing their final reports, so final review remains open.

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

### 1. Claude Code acceptance — blocked by client usage limit

- The only remaining Phase 1 client gate. Codex acceptance passed; Claude Code 2.1.299 is authenticated but its weekly limit stopped the session before tool calls.
- The client reported a reset at **2026-10-24 03:00 Asia/Manila**. Re-run the Settings dark-mode observe/toggle/restore flow with approval review enabled after that time.
- Acceptance requires an actual Claude Code session, not a simulated MCP client. Record the tool calls and the before/after observations in the evidence ledger.

### 2. Physical phone accessibility — needs a human TalkBack pass

- **TalkBack traversal:** partially verified on the OnePlus CPH2653: TalkBack and Andromeda both bound, focus rings appeared on Settings and Stop, and Start session started a session while TalkBack was enabled. A human pass is still required for full focus order, labels, double-tap activation and permission/pairing screen traversal; the first-run TalkBack hint overlay and synthetic key events do not reproduce that path.
- **Landscape:** passed on the OnePlus CPH2653 on 2026-10-23; rotation 1 produced 2376×1080 bounds and the companion UI remained usable. The phone was restored to portrait 0/0. Focus-order verification in landscape still needs the same human TalkBack pass.
- **Rotation rejection:** covered by API-level tests. Physical rotation now uses `settings put system accelerometer_rotation 0` plus `settings put system user_rotation 1`; this produced rotation 1 and 2376×1080 bounds, and both settings were restored to 0. Do not leave rotation or accessibility settings changed.

### 3. Gesture accuracy, completion and recovery — needs scripted scenarios

- The harness currently measures capture, persistent-stream frames and a Home-key input dispatch. It does not measure whether a tap/swipe hit the intended control, whether a multi-step task completed, or whether the agent recovered from a wrong screen.
- Add a small set of deterministic Settings tasks (open a page, toggle, restore; dismiss a dialog; recover from a wrong screen) and record success rate, retries, wall time and final-state verification. Keep model behavior separate from transport latency.

### 4. Model/tool overhead — needs a real client/model run

- Capture client-visible token counts, tool-call counts and end-to-end latency for the same task through at least one real model/client. The local harness measures host/device transport only.
- Do not compare these numbers to another product unless the model, prompt, device, transport, thermal state and task are matched.

### 5. Representative open-source comparisons — not started

- Select comparable local Android-control tools and document why each is comparable. Run matched device/transport/build/task measurements; include failures, not only successful latency.
- The current README and benchmark report deliberately make no speed claim. This gate stays open until raw comparable results exist.

### 6. Release packaging and artifact verification — release work

- Build release artifacts for the CLI, desktop app and companion APK on Windows, macOS and Linux; include the project licenses and a generated third-party notices bundle (see [licenses](licenses.md)).
- Produce checksums and verify the npm launcher against an installed platform package, not only a local Tauri build.
- macOS signing/notarization, Windows signing and any store packaging are release decisions. The current debug package is a development preview, not a signed release.
- Published npm platform packages, GitHub release assets and `v0.1.0` publication remain open.

### 7. PR review and merge — needs a push/review decision

- The current `feat/android-companion` branch is ahead of `origin/feat/android-companion` by one commit; all work in this session is uncommitted. PR #3 is open at `b496246`.
- Push/update the branch, run the full PR CI (core, desktop, launcher and Android emulator jobs), complete both review axes and merge with a conventional title. Do not merge until the physical/accessibility gates above are either passed or explicitly waived.

### 8. Cleanup — needs a keep/remove decision

- Test emulator, desktop app and ADB server were stopped; `.scratch` and the benchmark temp directory were removed.
- Remaining build artifacts and feature branches are intentionally kept for review. Deleting them requires a decision about whether the debug packages and evidence are still needed.


## Continuation on 2026-10-23

- Preserved the other agent's uncommitted streaming, launcher, UI, tests and benchmark work, and read its handoff log before continuing.
- Added `adm_get_long_view`: explicit scrolling viewport, up to eight bounded page observations, settle delay, repeated-viewport stop, geometry checks, input authorization/approval, cancellation and Stop. Tiles can overlap; list inertia can skip content. This is a bounded scan, not verified complete coverage or a seamless stitch. Reobserve before using coordinates from historical pages.
- Existing tap/swipe batches already support a shared gap and per-step `pause` actions. Adaptive updates to a held gesture remain planned; model/subagent response time cannot provide precise gesture timing.
- Added lossy/lossless WebP, automatic compatible byte-size selection and opt-in AVIF. Public-MCP tests cover decoded dimensions/pixels, MIME, smallest compatible candidate, and unavailable optional encoder. The default remains PNG; AVIF is never chosen automatically.
- Added capture/geometry/image-processing timing fields so device time can be separated from encoding. MCP/client upload and model time still require a dedicated client trace.
- Verified the long-view tool on the physical game's skill list, and used bounded existing batches to continue the unfinished career. Detailed daily completion is recorded separately after verifying the remaining mission state.
- Corrected the handoff's expiry claim: the existing smoke checks timer selection/persistence and manual Stop, not a full elapsed-time expiry.
- These changes and the inherited changes still require full PR CI and review; no release-completion claim is made.
