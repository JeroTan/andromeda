# Seamless project sessions

User-authorized change, 2026-10-23. This supersedes the desktop's earlier per-action approval and global Resume design.

- Configure and save project directory, device and allowed tools once. Start the project once; allowed requests execute immediately, without an Andromeda approval prompt or approval timeout. Agent-client policies for sensitive actions remain independent.
- Each project has Stopped, Running and Paused states, with Start, Pause/Resume and Stop controls. Pausing/stopping cancels its work and prevents new input. Observation remains available. Other projects continue.
- Global Stop all projects is an emergency stop. Older Start/Resume or phone events cannot undo a later Stop. Input already accepted by Android may finish; never automatically retry input.
- Pair the optional phone bridge once. Saved pairing and authenticated desktop connectivity must be shown separately. Reuse healthy pairing. A computer missing its credential must report this honestly; never claim an end-to-end connection merely because the phone retained a key.
- Starting a desktop project starts its phone session without another Start on the phone. Show per-project state on both screens. Phone Pause/Resume and Stop change the same project. Starting a stopped project remains on desktop where scope is visible.
- Use bounded authenticated state messages, roughly once per second, independent of screenshot polling. Failed paired bridges pause their projects, and reconnection alone never resumes input. Phone heartbeat loss is detected within five seconds. This is connection detection, not a session timer.
- Session duration defaults to unlimited. The optional saved timer remains under Settings → Session. Its expiry propagates to desktop.
- Preserve private credentials, canonical directory/device/tool scope, bounded work, cancellation and rotation checks. Do not enable or run GitHub Actions. Verify locally; no game actions for this change.

Validation boundaries: public managed MCP calls over simulated ADB, companion protocol framing/state replication, and installed macOS/physical-phone session controls. Mock tests do not establish physical Android behavior.
