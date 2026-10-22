# Add to Claude Code

Build [Andromeda](installation.md), start ADB, and copy the serial from `andromeda devices`:

```sh
claude mcp add --transport stdio andromeda -- andromeda --device YOUR_SERIAL --allow-control
claude mcp get andromeda
```

For a project configuration, add an entry to `.mcp.json`:

```json
{
  "mcpServers": {
    "andromeda": {
      "type": "stdio",
      "command": "/absolute/path/to/andromeda",
      "args": ["--device", "YOUR_SERIAL", "--allow-control"]
    }
  }
}
```

Open a fresh session, approve the project server when prompted, and inspect `/mcp`. Omit `--allow-control` for observation-only use. See [Claude Code's MCP documentation](https://code.claude.com/docs/en/mcp) for scopes and client permission settings.

Start with:

> Use Andromeda to capture the phone at a maximum width of 720 pixels and inspect its accessible UI. Describe what is visible without taking action.

For the device acceptance test, ask Claude to inspect dark mode in Settings, switch it once using touch controls, verify it, and restore the original state. Use a test phone and keep the task confined to that setting. Client test evidence is tracked in [implementation-status.md](implementation-status.md).

An `adm_stop` call cancels pending work in that server. Android may finish a gesture that it already accepted. Observe the screen before continuing.
