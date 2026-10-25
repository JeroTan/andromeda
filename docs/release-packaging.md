# Release packaging

The native deliverables are an Android **APK** containing the Lynx phone interface, a macOS **DMG**, and a Windows **EXE installer**. The desktop UI uses Tauri with the Rust controller. Linux source/package builds have historical CI evidence; current Linux native UI acceptance remains unverified. The optional npm launcher is `@jrw/andromeda`; platform packages use `@jrw/desktop-<platform>-<arch>`. The unrelated bare npm package `andromeda` must not be used to install this project.

## Native builds

From `desktop/`, run:

```sh
npm ci
npm run installer
```

The helper selects `dmg` on macOS, `nsis` on Windows, and `deb` on Linux. Run it on the corresponding OS with the documented Tauri prerequisites. Outputs are under the workspace's `target/release/bundle/` directory. `npm run installer -- --debug` produces development packages. NSIS produces an `.exe` installer, and a DMG contains the app plus the Applications drop target. See the official [Windows installer](https://v2.tauri.app/distribute/windows-installer/) and [DMG](https://v2.tauri.app/distribute/dmg/) guides.

For Android, build the offline Lynx bundle before Gradle:

```sh
npm --prefix android-companion/ui ci
npm --prefix android-companion/ui run build
cd android-companion
./gradlew :app:assembleRelease
```

The current Gradle release output is `app/build/outputs/apk/release/app-release-unsigned.apk`. It still needs the project's release signing configuration. The existing debug APK is for local acceptance, not the public release. An npm package does not replace this APK or either desktop installer.

GitHub Actions remains disabled and manual-only. Changing its configured package targets does not authorize a run. Windows NSIS is cross-built and its payload inspected; Windows execution is deferred by the operator. Current Linux installer/UI validation remains open. Historical MSI checks do not establish current EXE acceptance.

## Notice collection and local npm staging

Install the pinned collector outside the published artifact:

```sh
cargo install cargo-about --version 0.9.2 --features cli --locked --root .scratch/notices-tools
python3 scripts/collect-notices.py \
  --cargo-about .scratch/notices-tools/bin/cargo-about \
  --out .scratch/macos-notices
```

This collects macOS arm64 Rust/default-feature dependencies and desktop npm runtime candidates. It verifies cached Rust archive checksums against Cargo.lock and npm archive SRI against the lockfile, compares installed files against those verified archives, harvests original texts from the archive bytes, includes nested native notices, and validates checked-in upstream overrides against exact crate/VCS hashes. `release/notice-overrides.json` records source revisions and unresolved original-text cases. The generated inventory deliberately retains `complete: false` until source and final-bundle review is complete. Android/Lynx and other target inventories remain separate work; see [research](research/release-notices.md).

Stage an existing macOS bundle as private local packages:

```sh
node scripts/stage-macos-package.mjs \
  --app /Applications/Andromeda.app \
  --notices .scratch/macos-notices \
  --out .scratch/macos-packages
```

The command refuses an existing output directory. It checks Mach-O architecture, app/CLI version agreement, preserves and verifies every app file and executable bit through `npm pack`, includes project/dependency notices beside the app, and writes SHA256SUMS plus an artifact manifest. These are private previews with `ready_to_publish: false`. The ZIP is a verification/support archive; the macOS end-user installer remains the DMG.

Verify a local install without registry publication:

```sh
npm install --prefix .scratch/package-check --ignore-scripts --offline --no-audit --no-fund \
  .scratch/macos-packages/jrw-desktop-darwin-arm64-0.1.0.tgz \
  .scratch/macos-packages/jrw-andromeda-0.1.0.tgz
.scratch/package-check/node_modules/.bin/andromeda --print-path
.scratch/package-check/node_modules/.bin/andromeda
```

The selected npm scope still needs authenticated ownership/publishing access on the release host. Publishing is pending; neither tarball staging nor a registry 404 proves scope ownership.

## Preview qualifications

The APK is signed with the private release identity. Desktop packages are unsigned, and macOS is not notarized. Windows runtime testing is deferred; wider native-platform/accessibility checks are not represented as passes. Collected notices and corresponding MPL archives ship inside the packages, with remaining source-provenance qualifications preserved. npm publication stays optional. See the current release audit and publication receipts for the artifact checks already completed.

## Full-duration timer acceptance

Use a disposable emulator only. Set **Settings → Session → 15 minutes** through the phone UI, enable its accessibility service and pair it into a new private state directory. Then run:

```sh
cargo run --locked --example verify_session_timer -- \
  emulator-SERIAL PRIVATE_STATE NEW_REPORT.json
```

The probe uses the public controller/managed gateway, keeps authenticated heartbeats running, waits the full elapsed duration, checks phone expiry and desktop Stop, then verifies subsequent input is rejected. It does not accelerate a clock. The temporary registration is removed on normal/error return; the state directory remains the caller's responsibility. Restore Off through Settings and delete the disposable emulator/state after collecting metadata.

## v0.1.0 preview publication

The operator deferred Windows-native testing and asked to finish the existing native artifacts. npm publication is optional, not a release gate. That first preview shipped the macOS arm64 DMG and signed Android APK. The later 0.1.1 release added the Windows EXE; Linux downloads are not shipped. The Mac package is not Developer ID signed or notarized.

Run `scripts/assemble-release-notices.py --macos-notices PATH_TO_COLLECTED_MACOS_NOTICES` after the collector to append the exact Lynx source-JAR notices, pinned PrimJS notices and corresponding MPL source archives. Copy the resulting directory to `android-companion/app/src/main/assets/third-party-notices/` before building the APK. Desktop packaging copies `release/generated-notices/` (or `ANDROMEDA_NOTICES_DIR`) into the app's resources. Generated directories are ignored by Git; source links, checksums and qualifications remain in the artifacts.

Android release signing material is held privately outside the repository in the operator's Application Support directory. Preserve a secure backup of the keystore and its password file; future release APKs need the same key. Do not upload either file. The release APK is aligned and signed with Android build-tools, then verified with `apksigner verify`. It uses a different certificate from the development/debug app; migration details are in the release notes.

## Windows cross-build used for 0.1.1

The unsigned NSIS preview was built on macOS with the official Rust 1.99.0 compiler, matching host and x86_64-pc-windows-gnu standard libraries, MinGW-w64 14.0.0 and NSIS 3.13. A Homebrew rustc build cannot consume the official stdlib metadata despite matching version numbers; use a matching compiler/stdlib set.

```sh
python3 scripts/collect-notices.py --cargo-about PATH_TO_CARGO_ABOUT \
  --target x86_64-pc-windows-gnu --out NEW_WINDOWS_NOTICES
python3 scripts/build-windows-preview.py --toolchain PREPARED_RUST_SYSROOT \
  --notices WINDOWS_NOTICES_WITH_VENDOR_TEXTS
```

The sysroot must contain its host compiler/stdlib and Windows GNU stdlib. The helper does not download or accept licenses automatically. Windows notices include the exact WebView2 1.0.3650.58 publisher-package texts after matching its loader DLL to the bundled crate binary, plus MinGW/Rust notices. This cross-build is not native Windows validation.

## Local macOS updates and MCP startup checks

The macOS bundle uses Tauri's ad-hoc signing identity (`-`), providing local code/resource integrity without Developer ID signing or notarization. This follows the [Tauri signing guide](https://v2.tauri.app/distribute/sign/macos/#ad-hoc-signing). The published 0.1.2 downloads predate this correction.

Build an app bundle with `npm run tauri --prefix desktop -- build --bundles app`, close the running desktop app, then run:

```sh
npm run install:macos --prefix desktop
# Optional test destination:
# npm run install:macos --prefix desktop -- --destination /absolute/path/Andromeda.app
```

The installer stages a fresh complete bundle beside the destination, signs and verifies it, checks the bundled MCP version/tool catalog, swaps directories and repeats verification at the installed path. Failed installed-path validation restores the previous app; a failed rollback preserves its backup. It never merges new executable bytes into the installed bundle. Private project/pairing state remains outside the app. Build preparation likewise stages the MCP executable at a new inode before replacement.

Regression checks: `npm run test:packaging --prefix desktop` and `python3 -m unittest discover -s scripts -p test_install_macos.py`. The native installer checks require macOS/Clang/code-signing tools. Desktop connection setup now starts the bundled server with `--version` before writing client configuration, reporting a killed or mismatched executable directly.

## Headless macOS DMG creation

`npm run installer --prefix desktop` builds the signed app bundle, then packages it with `desktop/scripts/create-dmg.mjs`. The image contains Andromeda.app and an Applications link. The helper verifies the staged signature and final image checksum, refuses an existing output, and does not open Finder or depend on `.DS_Store` layout timing. This avoids the reproducible temporary-image unmount failures seen with the previous Finder-driven helper on this Mac.
