Andromeda 0.1.1 adds the standalone Windows x64 installer and includes the latest desktop connection and Android icon fixes.

### Downloads

- **Andromeda-0.1.1-windows-x64-setup.exe** — standalone Windows x64 NSIS installer, including the MCP server and WebView2 loader. Unsigned and cross-compiled; native Windows testing was deferred by the operator.
- **Andromeda-0.1.1-macos-arm64.dmg** — standalone Apple silicon Mac app, including the MCP server. Not Developer ID signed or notarized.
- **Andromeda-0.1.1-android.apk** — signed, non-debuggable Android companion with the Andromeda adaptive icon. Uses the same release certificate as 0.1.0 and versionCode 2.
- **SHA256SUMS.txt** and **release-manifest.json** — hashes and artifact qualifications.

Node.js/npm are not needed to run these apps. Android Platform Tools/ADB and USB debugging are still needed for the phone connection. The Windows desktop uses Microsoft WebView2.

### Changes

- Unsaved access edits now show Save access beside project controls instead of silently disabling Start.
- Connect Claude runs Claude's project-local add command and verifies registration. Results remain visible, including the distinction between a configured server and an already-open chat's cached tool list. Existing Claude chats may need an MCP reconnect/reopen once; running the add command does not hot-reload those conversations.
- Android now uses Andromeda's orbital adaptive icon, with Android 13 themed-icon support.
- Native packages include collected notices and source associations. Windows also includes the matching WebView2 loader notices and MinGW/Rust notices.

### Preview qualifications

Windows is an unsigned GNU cross-build, not a Windows-runtime-tested release. Broader platform and screen-reader verification remains deferred or incomplete. Source/notice provenance qualifications are retained in the packages. GitHub Actions stayed disabled.

Android 0.1.0 release installs can update using the same release signing identity. Development/debug installs use a different certificate and require a clean install before switching to the release APK; no automatic uninstall or migration is performed.
