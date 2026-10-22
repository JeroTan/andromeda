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

Branch: `feat/android-mcp`, based on `f128cac`. Repository and instruction files were inspected; no existing implementation, coding standards, issues or PRs needed reconciliation. Agent instruction files were empty. The triage skill's issue-label workflow does not apply to this supplied specification.

This first implementation contains 16 core tools. `adm_multi_touch`, the companion, persistent capture, stronger per-action approval, the benchmark suite/comparisons, remaining platform/client/device checks and the final release remain in scope and are not complete. CI configuration is present; remote execution must still pass. The full goal remains active.
