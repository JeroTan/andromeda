Andromeda's first native preview provides 18 Android MCP tools, project-scoped desktop setup, synchronized Pause/Resume/Stop, and the optional Lynx phone bridge.

### Downloads

- **Andromeda-0.1.0-macos-arm64.dmg** — standalone desktop app for Apple silicon Macs. The Rust MCP server is included; Node.js/npm are not needed.
- **Andromeda-0.1.0-android.apk** — signed, non-debuggable phone companion for Android 8+, with ARM64, ARMv7, x86 and x86_64 libraries. Screenshot acceleration needs Android 11+.
- **SHA256SUMS.txt** and **release-manifest.json** — artifact hashes and build qualifications.

Android Platform Tools/ADB and USB debugging are still needed for the computer-to-phone connection. Enable accessibility and pair once if using the optional companion. Sessions default to unlimited; timers are optional in Settings.

### Preview status

The Mac app is not Developer ID signed or notarized. Windows native testing was deferred by the operator; a Windows EXE is not included in this preview. Linux packages and broader screen-reader/platform checks are also pending. npm publication is optional and is not part of this release.

The Android release APK has a dedicated signing key. An existing development/debug install uses a different certificate, so switching to this release APK requires removing that development app and pairing the new install once. Keep the current development app if you do not want to migrate yet; no automatic migration/uninstall is performed.

Collected license/NOTICE texts, publisher source associations and five MPL Rust source archives are bundled in both native packages. Source provenance qualifications are preserved in those files; this preview does not claim a fully reproduced third-party binary supply chain.

### Verification

62 Rust checks passed, with real Codex and Claude Settings acceptance, physical multi-touch/Unicode checks, a full 15-minute timer expiry check, and 48 physical gesture endpoint samples plus six recovery tasks. Physical scripted screenshot-to-tap acknowledgement measured 164 ms median, excluding model thinking. Native-PNG comparisons and their mode-specific limits are documented in the repository.

GitHub Actions remained disabled. No hosted test run was started for this release.
