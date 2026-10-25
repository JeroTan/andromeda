<p align="center"><img src="docs/assets/andromeda.svg" alt="Andromeda. Give your AI agent a phone." width="960"></p>

**Connect your Android phone to your favorite AI agent.**

Use Claude Code, OpenAI Codex or OpenCode to see your screen, tap, swipe, type, and work through Android apps. Andromeda is the Rust-powered bridge that gives your agent those controls through the Model Context Protocol (MCP).

You choose the task, the device and the allowed tools. Your agent observes the screen, takes action and checks the result.

[Download the native apps](https://github.com/JeroTan/andromeda/releases/tag/v0.1.3) · [Desktop setup](docs/desktop.md) · [Android companion](docs/companion.md) · [Tool reference](docs/api.md)

## Bring your favorite agent

| Agent | Connect with Andromeda |
| --- | --- |
| **Claude Code** | [Connect from the desktop app](docs/claude-code.md). A real-device Settings task has been verified. |
| **OpenAI Codex** | [Connect from the desktop app](docs/codex.md). A real-device Settings task has been verified. |
| **OpenCode 2** | [Connect from the desktop app](docs/opencode.md). Connection verified with OpenCode 2.0.22 and 2.1.4; direct schemas by default. Included in the Mac preview/source builds; Windows v0.1.2 predates this connector. |
| **Cursor and other MCP clients** | [Configure a stdio connection](docs/clients.md). These clients need their own compatibility checks. |

Your agent brings the reasoning. Andromeda supplies the phone controls, so the same tools can work across different apps.

## Give your agent the phone chores

Gacha dailies can feel like a second job. Long dialogue piles up when you just want to play. A quick look at your feed turns into an hour of scrolling. These are the kinds of tasks that inspired Andromeda.

Try prompts like these with a connected agent. They are ideas for workflows, not claims that every app or task has been tested:

- **Gacha dailies and story catch-up:** “Check which daily activities remain. For this story section, advance the dialogue and summarize what happened.”
- **Less doomscrolling:** “Review the next ten posts in this feed and summarize the ones about topics I follow.”
- **Mobile-only work:** “Open my work app, collect the visible status updates, and tell me what needs my attention.”
- **Dating profiles:** “Help me review profiles against the interests I gave you. I'll choose whom to contact.”
- **Portfolio check-ins:** “Summarize the holdings and changes shown in my portfolio app. Leave trades for me.”
- **Game experiments:** “Try the controls in my single-player prototype and report where you get stuck.”

Tasks depend on what the app exposes and how well your agent handles it. Start with a small task, watch the result, then expand what you delegate.

## Get started

The [v0.1.3 preview](https://github.com/JeroTan/andromeda/releases/tag/v0.1.3) includes an Apple silicon Mac DMG and Android APK. The previous [Windows EXE preview is available in v0.1.2](https://github.com/JeroTan/andromeda/releases/tag/v0.1.2). The desktop apps include the Rust MCP server and run without npm. The optional phone companion adds multi-touch, Unicode input and faster screenshots on supported devices.

1. Install the desktop app and [Android Platform Tools](https://developer.android.com/tools/releases/platform-tools). Connect your Android device and authorize USB debugging.
2. In Andromeda, choose your project folder, device and allowed tools. Add your device-use instructions to `ANDROMEDA.md` through **Edit instructions**.
3. Connect your agent from the desktop app and load the connection in that client. Use **Start project** when you're ready for input.

Each project has its own Start, Pause/Resume and Stop controls. **Stop all projects** is the emergency control. Sessions stay open until you stop them by default; the phone's optional timer lives under Settings → Session.

See the [installation guide](docs/installation.md) for setup and troubleshooting. The Mac app has ad-hoc signatures and is not Developer ID signed or notarized; the earlier Windows preview is unsigned. Windows runtime testing is deferred, and broader accessibility checks remain unverified. Release notes and the [evidence ledger](docs/implementation-status.md) record the limits. The [npm launcher](npm/README.md) is an optional local launch route; it is not published to npm.

## Build the command-line server

Install [Android Platform Tools](https://developer.android.com/tools/releases/platform-tools) and Rust 1.88 or later. Enable USB debugging on a test phone, connect it, and accept its authorization prompt.

```sh
adb start-server
cargo install --path . --locked
andromeda devices
andromeda --device YOUR_SERIAL
```

The last command starts an MCP server on standard input/output. Your MCP client normally launches it. It can observe the phone by default. Add `--allow-control` when you want the session to send input.

```sh
codex mcp add andromeda -- andromeda --device YOUR_SERIAL --allow-control
claude mcp add --transport stdio andromeda -- andromeda --device YOUR_SERIAL --allow-control
```

See the [installation guide](docs/installation.md) for platform setup and troubleshooting.

## Observe, act, verify

Ask your agent to inspect the screen, perform a task, and check the result:

> Open Settings, find the display settings, and tell me whether dark mode is enabled. Ask before changing anything.

The agent receives an image or accessible UI nodes, chooses an action, then observes again. A successful tap means Android accepted the input; it does not prove the task succeeded.

| Capability | Tools |
| --- | --- |
| Observe | `adm_list_devices`, `adm_get_device_info`, `adm_get_view`, `adm_get_ui_tree`, `adm_list_apps` |
| Interact | `adm_touch`, `adm_touch_many`, `adm_swipe`, `adm_swipe_many`, `adm_press_key`, `adm_type_text` |
| Coordinate | `adm_multi_touch`, `adm_launch_app`, `adm_open_uri`, `adm_wait_for`, `adm_batch_actions`, `adm_get_long_view`, `adm_stop` |

Coordinates use native screen pixels, with `(0, 0)` at the top left of the current orientation. Capture results include dimensions and scale factors. Touch and swipe calls require observed geometry and reject a changed size or rotation.

## Performance and reliability

The device layer opens ADB services directly, without launching a host `adb` process for each tool call. Requests have deadlines and bounded output. `adm_stop` cancels active work, and failed input is never retried automatically. Screenshots support PNG, JPEG, lossy/lossless WebP and automatic compatible size selection. AVIF is an optional build feature for compatible clients/exports. The scrolling-view tool collects a bounded list scan; reobserve before using coordinates from earlier pages.

The capture, persistent-stream and input benchmark harness, plus Android 17 emulator and OnePlus physical baselines, are in [benchmarks](benchmarks/README.md). Matched native-PNG comparisons with two pinned open-source MCP implementations are also recorded, with their mode-specific limitations. There is no claim that Andromeda is the fastest Android MCP implementation.

## Develop

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

The reusable Rust device/controller library has no dependency on an AI vendor. See [CONTRIBUTING.md](CONTRIBUTING.md) and the [original specification](docs/intial_prompt.md) for scope and test expectations.

Original Andromeda code is available under MIT or Apache-2.0, at your option. Third-party dependencies retain their own licenses; see the [license review](docs/licenses.md).

## Privacy and responsible use

Andromeda runs locally over stdio and connects to a loopback ADB server. It returns screenshots to your agent without saving them; your AI client may retain or transmit those images and other tool results. Accessibility inspection briefly uses a private file on the device and removes it after reading. Protected screens may be blank, and some apps expose little accessible information.

Use your agent client's approval controls for messages, purchases, trades, money transfers and other account-changing actions. Coordinate input cannot determine the meaning of the button underneath it. Follow each app's automation and fair-play rules; automated gameplay can lead to account penalties. The game examples are for permitted personal tasks and test environments.

Andromeda is an independent project, unaffiliated with the AI providers or apps mentioned here. Read [SECURITY.md](SECURITY.md) for device access, project scope and privacy boundaries.
