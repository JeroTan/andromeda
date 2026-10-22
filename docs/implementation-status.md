# Implementation and release evidence

The original requirements are in [intial_prompt.md](intial_prompt.md). This ledger tracks the full scope; unchecked gates are unfinished. A passing mock test does not prove Android behavior or client compatibility.

## Phase 1: Core Android MCP server

- [x] Rust library and executable; Tokio and rmcp stdio transport
- [x] Device discovery, explicit selection, device information
- [ ] Screenshot capture, touch, swipe, keys, text
- [ ] Real Android setting changed and verified through Claude Code
- [x] Real Android setting changed and verified through Codex

## Phase 2: Advanced interaction

- [ ] Timed tap and swipe sequences
- [ ] True concurrent pointers through optional companion
- [ ] Batches, stop, deadlines, coordinate/timing validation
- [ ] Gesture integration tests on supported physical devices

## Phase 3: Perception and reliability

- [ ] Accessibility trees and UI/visual waiting
- [ ] Explicit persistent screen streaming and screenshot resizing/compression
- [ ] Rotation rejection and disconnect/reconnect behavior
- [ ] Cancellation, action logs, permissions and human approval controls
- [ ] Companion APK build, installation, permission and UI validation

## Phase 4: Performance

- [ ] Reproducible benchmark harness and environment metadata
- [ ] Screenshot/input p50/p95, turnaround, frame rate, CPU/memory, gesture accuracy, completion/recovery and tool overhead
- [ ] Measure persistent capture and optimize measured bottlenecks
- [ ] Representative open-source alternatives compared under matching conditions
- [ ] Publish raw measurements and limitations without unsupported speed claims

## Phase 5: Open-source release

- [ ] Windows, macOS and Linux build/test evidence
- [ ] Installation, Claude Code, Codex, Cursor and API guides
- [ ] Example prompts, demonstrations, contribution and security policies
- [ ] License and dependency/license review
- [ ] GitHub CI and release packaging
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

The core was merged through [PR #1](https://github.com/JeroTan/andromeda/pull/1) as `1ca97be`, after CI passed at `ee1a830` on Windows, macOS, Linux and Rust 1.88. The `feat/android-mcp` branch was deleted locally and remotely. Current work is on `feat/desktop-project-control`.

Repository and instruction files were inspected; no prior implementation, coding standards, issues or PRs needed reconciliation. Agent instruction files were empty. The triage skill's issue-label workflow does not apply to the supplied specification.

The merged implementation contains 16 core tools. `adm_multi_touch`, the companion integration, persistent capture, stronger per-action approval, the benchmark suite/comparisons, remaining client/device checks and the final release remain in scope and are not complete. Desktop additions have separate platform/UX gates. The full goal remains active.

## User additions and workflow preferences

On 2026-10-22 the user clarified that Andromeda's main interface should be a desktop app on Mac/PC. It should create MCP configurations for chats, scope access to selected project directories, read a project instruction file, and offer a simple npm launch command or downloadable executable. The Android app is an optional permission/gesture bridge, not the main product interface.

- [ ] Rust desktop setup/control app with device selection, preview and stop controls
- [ ] Project registration, allowed-tool policy, instruction-file loading and startup scope checks
- [ ] Reviewable project-local Codex/Claude configuration installation preserving existing settings
- [ ] Simple npm launcher and downloadable desktop packaging
- [ ] Desktop UI/accessibility and project isolation verification
- [ ] Shared visual identity based on the supplied editorial reference: bold typography, rounded panels, thin borders, pastel accents and original celestial graphics
- [ ] Polished phone permission/pairing/session UI with accessibility, text scaling, landscape and light/dark verification
- [ ] Lynx/TypeScript UI evaluation documented; Rust remains the control and authorization core

The user asked to move full automated suites to GitHub PR workflows. Run only focused checks/debugging and hardware-dependent tests locally. Android Studio and its emulator are authorized for testing.

The companion prototype is now in `android-companion/`, with its temporary build under ignored `.scratch/companion-build`. It builds with AGP 9.4.1, Gradle 9.7.1 and Android API 37. Its development APK is installed on the physical phone. Android currently reports its accessibility permission as disabled; the user's initial readiness response was followed by clarification of the desktop interface. No pairing token was saved, and no companion control test has run yet. The prototype UI will be replaced with ReactLynx.

The UI runtime spike selected Tauri 2 for desktop and ReactLynx for Android; see [runtime research](research/ui-runtime.md) and [Android integration research](research/android-lynx.md). The Lynx desktop spike rendered the requested visual direction but did not meet the observed desktop accessibility/Linux-window gates. Its app process was closed after inspection.

Project registry/configuration work now has focused tests for directory scope, disabled projects, instruction symlink rejection, preservation of existing Claude/Codex settings, stale-preview rejection and configuration path escapes. A shared controller can isolate project stop operations while retaining an operator-wide stop. The authenticated managed gateway and Tauri interface now integrate these capabilities. Focused tests also cover per-request approval, cancellation of pending approvals, operator pause, and concurrent instruction-file edits. The packaged macOS preview launches with local assets and exposes its application controls in the native accessibility tree. Project selection, starter-instruction creation, both themes and Codex/Claude configuration saving have been exercised in an isolated test workspace. The generated bundled MCP command initializes, loads the selected instructions, discovers devices, rejects an unrelated working directory and fails after app closure. See [desktop acceptance evidence](evidence/desktop-acceptance-2026-10-22.json). Cross-platform packaging, remaining UI checks and PR review are pending.
