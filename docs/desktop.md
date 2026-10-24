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

Run `npm run installer` from `desktop/` to build the native installer: macOS `.dmg`, Windows NSIS `.exe`, or Linux `.deb`. Use `npm run installer -- --debug` for a development preview. The build script includes a matching MCP executable inside the package. The manual CI configuration targets these formats, but hosted Actions is disabled at the operator's request. Native Windows/Linux installer validation remains a release gate. Build/test failures fail the job; an unavailable preview upload is reported separately in its summary. These packages are development builds; published platform packages remain a release task.

After a local build, the one-command launcher is available from the repository root:

```sh
node npm/launcher.mjs
# or, after npm link
andromeda
```

The launcher resolves `ANDROMEDA_DESKTOP_BIN`, a macOS `.app` path in `ANDROMEDA_DESKTOP_APP`, a `@jrw/desktop-<platform>-<arch>` package, or a local Tauri/development build. `--print-path` reports the selected executable. It does not download a release or start the MCP server itself. See [npm/README.md](../npm/README.md).

## Connect a project

1. Choose **Add a project** and select the folder where you work with your agent.
2. Select the connected Android device. Enable the tool groups the project needs and choose **Save access**. Observation tools are enabled initially; input tools are opt-in.
3. Open **Edit instructions** to create or edit the selected project instruction file. Its default path is `ANDROMEDA.md`. Existing files changed outside the editor must be reloaded before saving.
4. Choose **Connect Codex**, **Connect Claude** or **Connect OpenCode**, review the proposed entry, and save it. Codex uses `.codex/config.toml`; Claude runs its project-local CLI registration and connection check; OpenCode 2 updates the project's `opencode.json(c)` configuration. Other settings are preserved. Replacing a different existing Andromeda connection requires confirmation in the preview. See the [OpenCode guide](opencode.md) for JSONC handling and version support.
5. Reconnect MCP in your agent or reopen its project. Accept the client's project/connection trust prompt when appropriate. Instructions load when the MCP connection initializes.
6. Choose **Start project** once. Saved, allowed tools run automatically without per-action approval prompts. **Pause project** cancels its current work and blocks new input until **Resume project**. **Stop project** ends its session; choose Start project to begin again. Observation remains available. Other projects keep their own state.
7. For the optional phone bridge, grant Android accessibility and pair once. Starting the desktop project automatically arms its phone session; there is no separate phone Start. Both screens show each project's state, and Pause/Resume/Stop works from either screen. Phone Resume is available for paused projects; starting a stopped project stays on desktop where its scope is visible.

## Remove a project

Pause that project, select it, choose **Remove project**, and confirm. This removes the project registration and stops its live preview. The project folder and its files stay on disk. Existing Codex or Claude configuration is not changed; remove the `andromeda` entry manually if you no longer want it. Re-add the folder to manage it again.

**Stop all projects** is the emergency control: it ends every project session, cancels managed work, and stops live preview. A project's `adm_stop` ends only that project's session. Input already accepted by Android may finish; observe before retrying. Closing the app cancels its work. Agents cannot start or resume projects through MCP.

Phone synchronization uses authenticated, bounded messages about project names and state; it does not capture the screen. Healthy connections synchronize about once per second. A failed paired bridge pauses its projects; reconnecting does not automatically resume them. The phone detects a missing desktop heartbeat within five seconds and pauses running projects. This heartbeat is connection detection, not a session time limit. Sessions stay active until stopped unless an optional timer is enabled.

**Show screen** captures one screenshot on request. **Start live preview** opens one persistent capture stream and keeps updating the image until you choose **Stop live preview** or **Hide**. Live preview is explicit; it does not start automatically, and changing the selected device or project stops it. The desktop preview is not saved to a file. A stream error appears beside the preview and stops the stream rather than silently retrying.

The agent panel reports managed MCP connections separately from project access. A connection that stops responding disappears within 15 seconds. Initialization failures while loading project instructions appear in the panel. Changing the selected device clears the previous device's preview.

## Scope and private state

Managed MCP startup validates the process's actual working directory against the registered canonical project path. Descendant directories are allowed; unrelated paths and escaped symlinks are rejected. Tool allowlists and device restrictions are checked by the desktop host for every request. Disabling a project revokes subsequent requests. Edit a project’s access while that project is paused or stopped. Saving scope changes ends its session and invalidates older Start/Resume requests.

Credentials and gateway metadata live in the user's private application data directory, outside the project. Project configuration contains the registered project ID and executable path, not a credential. The local gateway authenticates both sides before accepting a request. MCP itself uses stdio.

This scope prevents accidental use from unrelated chats. It is not an OS sandbox: other processes running as the same OS user can access that user's files or run the standalone CLI. The standalone CLI's `--allow-control` mode remains a separate, explicit authorization path and relies on the agent client's approval policy.

## Troubleshooting

- **No devices:** start ADB, check the USB cable and USB debugging authorization, then choose Refresh. An offline or unauthorized device cannot be selected.
- **Desktop offline:** open Andromeda. Managed connections fail closed while its gateway is unavailable.
- **Project scope mismatch:** open the agent in the registered directory; copied configuration does not register another folder.
- **Project paused/stopped:** use that project’s Resume/Start button. MCP cannot clear this state. There is no per-action approval queue.
- **Pairing differs between screens:** a phone can retain its credential while the computer has none. Open Connection controls → Pair this computer again on the phone, then Pair this phone on desktop once. Existing healthy pairing is reused.
- **Request timeout:** the deadline covers execution, not human approval. Observe the screen before retrying input.
- **Configuration changed:** reopen the preview so it incorporates external edits.
- **Live preview stopped:** check the USB connection and unlock the phone, then start the preview again. The last frame remains until you hide it.
- **Moved application:** reconnect the project to update its executable path.
