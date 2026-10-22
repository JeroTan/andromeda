# Desktop project setup

Andromeda's desktop app creates a managed MCP connection for a selected project. The app owns the Rust controller; keep it open while your agent uses the connection. The Android companion is a separate, optional permission and gesture bridge.

## Build the preview

Install the [Tauri desktop prerequisites](https://v2.tauri.app/start/prerequisites/), Node.js 24, Rust 1.88 or newer, and Android Platform Tools. Then:

```sh
adb start-server
cargo build --locked --bin andromeda
cd desktop
npm ci
npm run tauri dev
```

To build a packaged preview, run `npm run tauri build -- --debug`. The build script includes a matching MCP executable inside the package. CI produces preview packages for macOS, Windows and Linux. These packages are development builds; the signed release and npm launcher are tracked separately in the release checklist.

## Connect a project

1. Choose **Add a project** and select the folder where you work with your agent.
2. Select the connected Android device. Enable the tool groups the project needs and choose **Save access**. Observation tools are enabled initially; input tools are opt-in.
3. Open **Edit instructions** to create or edit the selected project instruction file. Its default path is `ANDROMEDA.md`. Existing files changed outside the editor must be reloaded before saving.
4. Choose **Connect Codex** or **Connect Claude**, review the proposed entry, and save it. This writes `.codex/config.toml` or `.mcp.json` inside the project. Other settings are preserved. Replacing a different existing Andromeda connection requires confirmation in the preview.
5. Reconnect MCP in your agent or reopen its project. Accept the client's project/connection trust prompt when appropriate. Instructions load when the MCP connection initializes.
6. Choose **Resume session** when ready for input. Review each proposed action in the desktop app and choose **Allow once** or **Decline**. Approval covers the exact tool arguments and selected device. The tool's `timeout_ms` includes time spent waiting for approval (15 seconds by default, up to 120 seconds).

**Stop & pause** cancels managed requests across projects, including requests awaiting approval, and keeps input paused until you resume it. A project's `adm_stop` cancels only that project's requests. Input already accepted by Android may finish; observe the device before retrying. Closing the app shuts down the gateway and cancels its operations. An agent cannot resume the desktop session through MCP.

**Show screen** captures one screenshot on request. It does not start background capture. Hide the preview to remove it from the interface. The desktop preview is not saved to a file.

## Scope and private state

Managed MCP startup validates the process's actual working directory against the registered canonical project path. Descendant directories are allowed; unrelated paths and escaped symlinks are rejected. Tool allowlists and device restrictions are checked by the desktop host for every request. Disabling a project revokes subsequent requests. Edit access while input is paused.

Credentials and gateway metadata live in the user's private application data directory, outside the project. Project configuration contains the registered project ID and executable path, not a credential. The local gateway authenticates both sides before accepting a request. MCP itself uses stdio.

This scope prevents accidental use from unrelated chats. It is not an OS sandbox: other processes running as the same OS user can access that user's files or run the standalone CLI. The standalone CLI's `--allow-control` mode remains a separate, explicit authorization path and relies on the agent client's approval policy.

## Troubleshooting

- **No devices:** start ADB, check the USB cable and USB debugging authorization, then choose Refresh. An offline or unauthorized device cannot be selected.
- **Desktop offline:** open Andromeda. Managed connections fail closed while its gateway is unavailable.
- **Project scope mismatch:** open the agent in the registered directory; copied configuration does not register another folder.
- **Input paused:** choose Resume session in Andromeda. Tool calls cannot clear an operator pause.
- **Approval expires:** send a new request after observing the current screen. If appropriate, use a larger `timeout_ms`; approval does not extend the request deadline.
- **Configuration changed:** reopen the preview so it incorporates external edits.
- **Moved application:** reconnect the project to update its executable path.
