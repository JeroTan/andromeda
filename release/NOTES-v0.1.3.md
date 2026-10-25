Andromeda 0.1.3 adds Android background-connection setup and clearer agent tool guidance, with the underlying Mac MCP startup/update corrections.

### Downloads

- **Andromeda-0.1.3-android.apk** — signed Android companion, versionCode 4, using the same release certificate. Update the existing release installation without uninstalling it.
- **Andromeda-0.1.3-macos-arm64.dmg** — Apple silicon desktop/MCP bundle with valid ad-hoc code signatures. It is not Developer ID signed or notarized.
- **release-manifest.json** and **SHA256SUMS.txt** — artifact details and hashes.

The previous [Windows x64 preview remains available in v0.1.2](https://github.com/JeroTan/andromeda/releases/tag/v0.1.2); this release does not provide an updated Windows installer or claim Windows runtime verification. That older installer predates the OpenCode connector and current core fixes.

### Changes

- Android Settings → Background connection checks battery exemption/background restriction state and requests Android's owner-approved exemption only when needed. Xiaomi-specific controls are explained; they are not changed silently. Existing permissions and pairing are detected.
- OpenCode connections default to direct native tool schemas. Descriptions, initialization instructions and executable examples explain swipe arguments, coordinate scaling, real condition waits, fixed batch pauses, and outcome verification.
- Mac builds stage executable replacements at fresh inodes and sign complete bundles. The local installer verifies the staged and installed server and rolls back on failure. Connect OpenCode applies the reviewed project entry and verifies live status instead of only saving a file.
- Mac DMGs are created without Finder automation or layout-timing dependencies.

Battery flows were checked with optimized/exempt states on an isolated Android emulator. A five-second switch to Android Settings retained the paired session and bridge; this is not long-term Doze/Xiaomi reliability evidence. Android must still obtain your approval for an exemption, and the computer app must stay open. Direct schemas do not guarantee model accuracy. Broader human accessibility and native-platform qualifications remain documented.
