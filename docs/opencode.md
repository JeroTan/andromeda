# Add to OpenCode

In Andromeda, select your saved project, choose **Connect OpenCode**, review the proposed connection, then choose **Connect this project**. Andromeda checks the bundled server, applies the reviewed connection to that OpenCode project and verifies its live status. A stale failed connection is replaced automatically without reloading unrelated sessions. Start the Andromeda project when you want to allow input. Keep Andromeda open while connected.

This connector targets **OpenCode 2**, including the installed 2.0.22 client. It adds a local stdio server under `mcp.servers.andromeda`, using OpenCode's command array and per-server startup, catalog and execution timeouts. New connections default to `codemode: false`, exposing Andromeda's full tool schemas directly instead of requiring JavaScript wrappers or deferred schema search. OpenCode 1 uses a different schema; migrate its configuration before using this connector. See the official [OpenCode 2 MCP guide](https://opencode.ai/v2/docs/mcp-servers/).

Andromeda updates the highest-precedence existing configuration in the selected directory: `.opencode/opencode.jsonc`, `.opencode/opencode.json`, `opencode.jsonc`, then `opencode.json`. With no existing file, it creates `opencode.jsonc`. It preserves comments, unrelated settings and other servers. Replacing a different Andromeda entry requires confirmation in the desktop preview. External changes after preview require a fresh preview.

Project configuration contains the executable path, private-state directory and project ID. Pairing credentials stay outside the project. The managed server validates the actual caller directory, allowed tools and selected device. Saving a connection does not start a stopped project or grant unrestricted input.

To inspect the connection from the selected project:

```sh
opencode mcp list
```

If setup fails, the desktop reports the failure instead of treating a saved file as a connected server. Higher-precedence system settings, nested project configs or explicit runtime overrides may change what OpenCode loads. See [configuration precedence](https://opencode.ai/v2/docs/config/).

For a standalone server instead of desktop-managed access, use OpenCode 2's CLI:

```sh
opencode mcp add andromeda -- /absolute/path/to/andromeda --device YOUR_SERIAL --allow-control
```

The standalone command explicitly enables ordinary input and follows the client approval policy for sensitive actions. Omit `--allow-control` for observation-only use.

## Tool-call troubleshooting

The MCP connection can be healthy while an agent supplies invalid arguments. Read the tool's current schema before calling it. `adm_swipe` requires a nested `swipe` object and fresh native `screen` geometry; a top-level duration alone is not a swipe request.

`adm_wait_for` watches an actual text/resource/pixel condition. A made-up text condition used as a sleep will expire. For a deliberate delay, use `adm_batch_actions` with an explicit `pause` action. JavaScript timers such as `setTimeout` are not guaranteed in a client's Code Mode sandbox. Keep screenshot dimensions consistent and use each returned image's native-coordinate scale when choosing touch points. A model's reasoning time is separate from device/tool latency.

Direct schemas use more context than deferred Code Mode discovery, but make required fields and tool-choice guidance available up front. Reconnect an existing connection through the desktop preview to apply this default. This changes how the tools are presented, not the project tool allowlist or client permission policy. Server instructions and executable schema examples explain taps versus swipes/holds, real conditions versus batch pauses, consistent screenshot scaling, and observing outcomes before retrying. They reduce ambiguity without guaranteeing model accuracy.
