# Andromeda design

Andromeda runs on the computer beside the agent client. The Rust controller handles Android operations; MCP carries tool calls. The desktop app is the main setup and authorization interface. An optional Android bridge handles accessibility and concurrent gestures after the user grants Android permission.

## Desktop project setup

The desktop app registers an explicit project directory, selected device, allowed tools and instruction file. It previews changes to the project's `.mcp.json` or `.codex/config.toml` before applying them. Existing client settings are preserved. Credentials stay in private application storage, outside the project.

Generated MCP configurations launch the local Rust executable in managed mode for a registered project. Startup canonicalizes the actual working directory and rejects a mismatch with that project. Relative instruction paths must remain within the selected directory after symlink resolution. A missing/disabled project fails closed.

Project scoping prevents accidental activation in unrelated chats. It does not sandbox other programs running as the same operating-system user, who can access that user's files or explicitly run the standalone CLI.

## Shared control

The desktop app owns a controller and an authenticated loopback gateway. Managed MCP processes authenticate as registered projects. The gateway enforces each project's tool and device policy; a managed client never falls back to unrestricted ADB when the app is closed. Desktop stop cancels all managed work and latches input paused; only the operator can resume. A project's stop cancels that project's work, including pending approvals. Every managed input request needs approval for its exact arguments, selected device and project policy. Its deadline includes approval time. Input dispatch remains coordinated across projects sharing a phone.

The gateway is an internal transport. MCP remains stdio. It accepts bounded, framed requests and cancels in-flight work when a client disconnects. Endpoint metadata and credentials use private application storage.

## Interfaces and tests

- The controller's interface covers device observation and actions, independent of MCP and the desktop UI.
- The project registry's interface covers registration, instruction loading, authorization and reviewable client-config changes. File parsing and atomic updates stay inside that module.
- The companion adapter's interface covers pairing, accessibility, text insertion/replacement and synchronized gestures. No arbitrary shell command is exposed over its protocol.
- The desktop UI issues asynchronous operations and displays results. Device I/O and image processing must not block rendering.

Behavior is verified through the public MCP, device/controller and companion protocol seams. Managed-project isolation and configuration generation extend those tests through the public setup interface. Full suites and platform builds run in PR workflows; local work uses focused checks and hardware-dependent tests.

## Distribution

The desktop shell uses Tauri with a TypeScript UI and the Rust controller. A small npm launcher downloads a matching versioned release, verifies its checksum and opens the desktop app. Downloadable desktop packages provide a direct launch option. The Android bridge is a separate optional APK with a ReactLynx presentation layer. No installer silently enables accessibility or USB debugging.

## Visual direction

The user's 2026-10-22 reference replaces the initial dark developer-tool styling. Use an off-white/pale-lavender canvas, ink-colored typography, thin panel borders, rounded corners and bold headings. Small pastel gradients and original orbital/constellation graphics carry the Andromeda identity. Desktop and phone should share typography, spacing, color roles and interaction states.

Desktop panels organize project scope, agent connections, device state and preview. The phone bridge presents permission, pairing and session states with one primary action at a time. Move the test pad out of the primary setup flow. Both interfaces need keyboard/screen-reader support, readable scaling, visible focus and independently verified dark-theme contrast.

The [runtime spike](docs/research/ui-runtime.md) selected Tauri 2 for desktop and ReactLynx for Android. Lynxtron rendered the visual direction, but its current Linux package supports headless rendering only, and the observed macOS accessibility tree omitted application controls. The Android integration is documented separately in [android-lynx.md](docs/research/android-lynx.md). Both finished interfaces still need their own accessibility, offline-launch and performance checks.

This document includes the desktop scope added by the user on 2026-10-22. The implementation ledger distinguishes completed behavior from remaining work.
