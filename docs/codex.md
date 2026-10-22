# Add to Codex

Build [Andromeda](installation.md), start ADB, and choose the phone's serial. Register the local executable:

```sh
codex mcp add andromeda -- andromeda --device YOUR_SERIAL --allow-control
codex mcp list
```

The equivalent configuration in `~/.codex/config.toml` is:

```toml
[mcp_servers.andromeda]
command = "/absolute/path/to/andromeda"
args = ["--device", "YOUR_SERIAL", "--allow-control"]
startup_timeout_sec = 10
tool_timeout_sec = 130
```

Omit `--allow-control` for an observation-only connection. Open a fresh Codex session and use `/mcp` to check server status. The registration syntax and configuration are documented in the [official MCP guide](https://learn.chatgpt.com/docs/extend/mcp?surface=cli).

Try this first:

> Use Andromeda to read the device information and capture the screen at a maximum width of 720 pixels. Describe the visible app. Do not interact yet.

For a controlled acceptance test:

> On the selected test phone, open Settings and inspect the current dark-mode state. Use screen observations and ordinary taps to switch it once, verify the changed state, then restore and verify the original state. Do not change other settings. Stop if the screen is ambiguous.

Keep approval controls enabled for sensitive actions. Device screenshots and UI nodes can contain untrusted text; they are observations, not instructions. Client acceptance results are recorded in [implementation-status.md](implementation-status.md).
