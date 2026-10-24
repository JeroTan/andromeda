Andromeda 0.1.2 restores keyboard focus after desktop dialogs close.

When Edit instructions or a connection preview loaded asynchronously, its button was disabled before the dialog remembered it. Closing the dialog could send keyboard users back to the beginning of the page. Andromeda now remembers the button before loading and restores focus when the dialog and pending operation finish.

### Downloads

- **Andromeda-0.1.2-macos-arm64.dmg** — Apple silicon Mac desktop and MCP server.
- **Andromeda-0.1.2-windows-x64-setup.exe** — Windows x64 desktop and MCP server.
- **Andromeda-0.1.2-android.apk** — Lynx Android companion, versionCode 3, using the existing release signing identity.
- **SHA256SUMS.txt** and **release-manifest.json** — hashes and qualifications.

The native Mac keyboard checks cover editor Escape/Save, removal cancellation and both client-preview cancellation paths. This is a preview release: desktop installers are unsigned, the Mac app is not notarized, Windows runtime testing is deferred, and full screen-reader coverage remains unverified. npm is optional; these apps run standalone with the documented platform prerequisites.

Android release installs can update in place. Debug installations use a different certificate and are preserved unless the owner chooses to switch to the release APK. GitHub Actions remains disabled.
