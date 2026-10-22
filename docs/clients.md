# MCP clients

Andromeda uses stdio and the official Rust MCP SDK. The client starts the process and exchanges JSON-RPC messages over stdin/stdout. Logs go to stderr. There is no HTTP listener.

Use the dedicated [Claude Code](claude-code.md) and [Codex](codex.md) guides. For Cursor or another client that accepts an `mcpServers` configuration, use:

```json
{
  "mcpServers": {
    "andromeda": {
      "command": "/absolute/path/to/andromeda",
      "args": ["--device", "YOUR_SERIAL", "--allow-control"]
    }
  }
}
```

Place the entry in the client's documented local MCP configuration. Confirm discovery with `adm_list_devices`, then call `adm_get_device_info` and `adm_get_view`. Compatibility is proven by a recorded client session, not by a configuration example.

Set client tool deadlines above the requested Andromeda deadline. The server accepts 1–120000 ms and defaults to 15000 ms. Use a 130-second client deadline when running longer waits. Cancellation notifications and `adm_stop` are supported.
