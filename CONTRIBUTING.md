# Contributing

Changes should keep device control independent of the MCP adapter. Start with the [specification](docs/intial_prompt.md) and [evidence ledger](docs/implementation-status.md). Describe the user-visible behavior and its verification in each PR.

Use conventional titles such as `feat: add companion gestures`, `fix: reject rotated coordinates`, or `docs: explain device setup`. Use corresponding branch names such as `feat/companion-gestures`.

Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all-targets`. Tests belong at the public MCP interface, reusable device API, or companion protocol. A simulated ADB server can exercise framing and failure behavior. Use an authorized device for Android behavior; do not describe a fixture as a physical test.

Validate full batches before the first input. Bound device output, durations and concurrency. Treat action failures as uncertain outcomes; do not retry input automatically. Avoid logging text, UI content, screenshots, pairing secrets or device identifiers. Keep stdout reserved for MCP messages.

Performance reports must include the device, Android version, resolution, transport, build profile, workload, warm-up, sample count and raw measurements. Compare equivalent workloads. Separate controller latency from model reasoning and network time.

Contributions to original Andromeda code are licensed under MIT or Apache-2.0, at the recipient's option. Identify the source and license of any reused code or assets.
