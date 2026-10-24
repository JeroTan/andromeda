# Install Andromeda

Download the [v0.1.2 preview](https://github.com/JeroTan/andromeda/releases/tag/v0.1.2): standalone Windows x64 EXE, Apple silicon Mac DMG, and signed Android APK. These apps do not need Node.js/npm to run. Android Platform Tools/ADB is still needed on the computer. Windows is an unsigned cross-build without native Windows testing; Linux packages remain pending. The optional `@jrw/andromeda` launcher is unpublished; npm publication is not required. See the release notes for unsigned-Mac and development-APK migration details, and [release packaging](release-packaging.md) for build commands.

1. Install the desktop app from the DMG or EXE. Install the APK on the phone if you need the optional gesture, Unicode or faster screenshot bridge.
2. Enable USB debugging and authorize the computer on the phone. Start ADB using Android Platform Tools.
3. Open Andromeda, choose your project folder and device, save access, then use **Connect Claude** or **Connect Codex**. Start the configured project when ready.

Rust is needed only when building the server from source.

## Build the command-line server from source

The current development build requires Rust 1.88+ and Android Platform Tools. No root access is needed for basic observation and input. The Android shell-v2 transport requires Android 7 or newer; device and vendor behavior still needs validation.

Download Platform Tools from [Android Developers](https://developer.android.com/tools/releases/platform-tools), extract them, and add the directory containing `adb` to your `PATH`. Android Studio includes them under its SDK directory. On macOS that is usually `~/Library/Android/sdk/platform-tools`; on Windows, `%LOCALAPPDATA%\Android\Sdk\platform-tools`.

1. Enable Developer options and USB debugging on the phone.
2. Connect by USB, unlock the phone, and accept the authorization prompt.
3. Run `adb start-server`, then `adb devices -l`.
4. Build from the repository root:

   ```sh
   cargo install --path . --locked
   andromeda devices
   ```

5. Copy the serial into your [MCP client configuration](clients.md).

See [examples](examples.md) for safe observation, settings, batch, wait and companion-gesture prompts.

`andromeda` starts the stdio server. `andromeda tools` prints tool schemas. `andromeda --device SERIAL call adm_get_device_info` runs a diagnostic call. Use an absolute executable path in client configuration when the client cannot find your Cargo bin directory.

## Troubleshooting

| Result | Action |
| --- | --- |
| `adb_unavailable` | Start the local ADB server. Check `--adb-server 127.0.0.1:5037`. |
| `unauthorized` device | Accept the USB debugging prompt on the phone. |
| `offline` device | Reconnect the cable and check `adb devices -l`. |
| `device_required` | Set `--device SERIAL` or pass `serial` to the tool. |
| `control_disabled` | The operator can add `--allow-control` when starting the server. |
| `stale_screen` | Observe again and use the new geometry. |
| `ui_unavailable` | Unlock the phone and observe the screenshot. The app may not expose UI nodes. |
| `timeout` or `transport_error` during input | Observe the phone before retrying; the action may already have happened. |

On Linux, USB device permissions may require the platform-tools udev rules supplied by your distribution. On Windows, the device vendor may require a USB driver. ADB-over-network setup remains an operator action; Andromeda does not enable it or discover network devices.
