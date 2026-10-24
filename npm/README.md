# Andromeda launcher

The npm package is a thin launcher for the Andromeda desktop interface. It does not contain a second copy of the controller; it starts the desktop application, which owns the MCP gateway and the Android connection.

The bare npm name `andromeda` belongs to an unrelated package. Do not use `npx andromeda` to install this project. The selected release name is `@jrw/andromeda`, with platform packages under `@jrw`. It is not published yet; the eventual registry command will be `npx @jrw/andromeda`.

The package is not published yet. To test the launcher from a checkout:

```sh
cargo build --locked --bin andromeda
npm run tauri build --prefix desktop -- --debug
node npm/launcher.mjs --print-path
node npm/launcher.mjs
```

`--print-path` reports the executable that would start. The launcher looks for a platform package (`@jrw/desktop-<platform>-<arch>`), a local Tauri bundle, or a local development build. Set `ANDROMEDA_DESKTOP_BIN` to an absolute executable path, or `ANDROMEDA_DESKTOP_APP` to a macOS `.app` bundle, to override discovery. An explicit override that does not exist is an error; the launcher does not silently fall back to another build.

Arguments after `--` are passed to the desktop executable. The launcher preserves the desktop process's exit status. The release process still needs to publish the platform packages and verify them against signed desktop artifacts.


Local platform-package staging and offline install verification are described in [release packaging](../docs/release-packaging.md). Native deliverables remain Android APK, macOS DMG and Windows EXE installer. The staged npm packages are private previews until release gates and authenticated `@jrw` publication access are ready.
