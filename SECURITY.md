# Security

Use Andromeda only with devices and applications you are authorized to control. USB debugging grants broad access. Revoke debugging authorization and disable the companion when you finish.

The MCP server uses stdio, accepts an explicit device serial, and connects only to a loopback ADB server. It does not start network discovery or enable wireless debugging. Observation is enabled by default; ordinary input requires the operator's `--allow-control` startup flag.

That flag authorizes a session to send input. It does not identify sensitive operations. A tap on a purchase button looks like any other coordinate tap. Configure human approval in the client and require it for financial transactions, purchases, messages, deletions, and other account changes. Stronger per-action approval is part of the remaining release work in the evidence ledger.

Screenshots and accessibility results may contain private information. The server keeps screenshots in memory and returns them to the client. UI dumping uses a private temporary file on Android with shell cleanup traps. The client can retain results; review its settings. Android secure windows may produce blank captures. Andromeda does not bypass secure-window restrictions.

`adm_stop`, request cancellation and deadlines close in-flight ADB streams and stop further actions. Input already accepted by Android may finish. A disconnect does not prove an action failed; observe before retrying. Screen geometry checks reduce rotation mistakes but cannot make a remote observation and tap atomic.

Report a vulnerability through GitHub's private vulnerability reporting for this repository when available. If it is unavailable, open an issue requesting a private contact without including exploit details, credentials, screenshots or other sensitive data. Maintainers should enable private reporting before public release.
