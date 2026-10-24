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

## Desktop connection setup

The desktop **Connect Claude** action registers Andromeda through Claude Code's supported **local scope**, private to your account in the selected project. It invokes the installed Claude CLI, then checks that Claude reports the server connected. The confirmation stays in Andromeda's connection dialog. It does not enable every MCP server, grant blanket workspace trust, or change other projects.

Claude Code or its VS Code extension must be installed. Andromeda finds the native CLI in its standard installation locations; advanced setups can specify `ANDROMEDA_CLAUDE_BIN`. A different existing local server named `andromeda` is not silently replaced. Already-open Claude conversations may need their MCP connection refreshed to discover the newly registered server.

A manually maintained shared `.mcp.json` still follows Claude's project-server approval rules. Merely saving that file does not mean Claude has approved or loaded it. See [Claude's local scope and approval documentation](https://code.claude.com/docs/en/mcp#local-scope).

The desktop runs the equivalent of `claude mcp add andromeda --scope local --transport stdio -- EXECUTABLE ARGS...` when registration is missing, then `claude mcp get andromeda`. An identical existing registration is verified without removing/re-adding it. The result remains visible in the desktop dialog.

Registration and a running conversation's tool catalog are different states. The add command does not hot-reload an already-open Claude conversation. In that conversation, use its MCP reconnect control or reopen it once; no manual add command or new Andromeda approval is needed. The desktop does not claim that its separate CLI health check refreshed that conversation.
