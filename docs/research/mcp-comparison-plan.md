# Android MCP comparison plan

Researched 2026-10-24; **no servers installed or started, no device accessed**. Compare these actively released open-source implementations, not a representative market ranking. Keep GitHub Actions disabled.

## Release pins

| Implementation | Published package | npm `gitHead` |
| --- | --- | --- |
| Mobile Next, Apache-2.0 | `@mobilenext/mobile-mcp@1.0.12`, 2026-10-18 | `6d60c458f9451824dd7c11355b92b2ad316c04c6` |
| MetaMask, MIT/Apache-2.0 | `@metamask/device-mcp@0.5.0`, 2026-10-19 | `623f808ea354c7a4d71370afd56ee0c6d7c4ec26` |

Versions, dates, source revisions and tarball integrity come from the official [Mobile Next registry](https://registry.npmjs.org/@mobilenext%2fmobile-mcp) and [MetaMask registry](https://registry.npmjs.org/@metamask%2fdevice-mcp). Archive resolved dependency versions and integrity: top-level pins alone leave transitive ranges mutable. Use Node 24 for both.

## Launch and MCP calls

Convenience launch recipes, **not executed**:

```sh
env MOBILEMCP_DISABLE_TELEMETRY=1 MOBILEMCP_LEGACY_ROBOT=1 npx --yes --ignore-scripts @mobilenext/mobile-mcp@1.0.12 --stdio
env DEVICE_PLATFORM=android DEVICE_ID="$SERIAL" DEVICE_MCP_ADB_SNAPSHOT=dump npx --yes --ignore-scripts @metamask/device-mcp@0.5.0
```

Install into isolated benchmark directories first; for measurements launch the resolved local binaries directly, excluding npm downloads/resolution. Supply the same existing ADB executable through PATH and Mobile Next's `ANDROID_HOME`. Give MetaMask an empty working directory: a `.device-session` there overrides explicit Android selection with Appium. [Backend selection](https://github.com/MetaMask/device-mcp/blob/623f808ea354c7a4d71370afd56ee0c6d7c4ec26/src/backends/index.ts), [CLI](https://github.com/MetaMask/device-mcp/blob/623f808ea354c7a4d71370afd56ee0c6d7c4ec26/src/cli/device-mcp.ts).

Both use newline-delimited stdio JSON-RPC. Send `initialize` with protocol `2025-03-26`, client information and empty capabilities; honor the negotiated version, send `notifications/initialized`, then save `tools/list`. Invoke `tools/call` with these `params`:

| Server | Native PNG capture | Harmless input fixture |
| --- | --- | --- |
| Andromeda | `{"name":"adm_get_view","arguments":{"image":{"format":"png"}}}` | `{"name":"adm_press_key","arguments":{"key":"home"}}` |
| Mobile Next, legacy mode | `{"name":"mobile_take_screenshot","arguments":{"device":"SERIAL","scale":1}}` | `{"name":"mobile_press_button","arguments":{"device":"SERIAL","button":"HOME"}}` |
| MetaMask | `{"name":"device_screenshot","arguments":{}}` | `{"name":"device_press_button","arguments":{"button":"home"}}` |

Prepare an already-open neutral launcher; repeated HOME then remains on that launcher. This measures acknowledgement, not presentation latency. Andromeda needs its existing control authorization. [Local schemas](../api.md), [Mobile Next schemas](https://github.com/mobile-next/mobile-mcp/blob/6d60c458f9451824dd7c11355b92b2ad316c04c6/src/server.ts), [MetaMask screenshot](https://github.com/MetaMask/device-mcp/blob/623f808ea354c7a4d71370afd56ee0c6d7c4ec26/src/tools/screenshot.ts), [button schema](https://github.com/MetaMask/device-mcp/blob/623f808ea354c7a4d71370afd56ee0c6d7c4ec26/src/tools/press-button.ts).

## Behavioral differences and side effects

Mobile Next's **default is JPEG quality 75, maximum dimension 1024**. `scale:1` removes resizing but cannot select PNG. Explicit legacy mode ignores those image options and returns native PNG using `adb exec-out screencap -p`, with display-selection queries and subsequent screen-size/density queries. HOME uses `adb shell input keyevent KEYCODE_HOME`. Label this comparison “Mobile Next ADB compatibility mode,” not its default performance. [Implementation](https://github.com/mobile-next/mobile-mcp/blob/6d60c458f9451824dd7c11355b92b2ad316c04c6/src/android.ts).

Even legacy mode checks `mobilecli --version`. Its published dependency chain currently resolves `mobilewright@0.0.67` → driver → `mobilecli@1.0.21` (revision `8d1203d5dd5049665c22e9bc4cfed43e9024c600`), distributed as native platform packages without install scripts. Pin that chain too. [Registry](https://registry.npmjs.org/mobilecli/1.0.21), [binary lookup](https://github.com/mobile-next/mobile-mcp/blob/6d60c458f9451824dd7c11355b92b2ad316c04c6/src/mobilecli.ts). Default mobilecli capture uses an on-device DEX server with screencap fallback; evaluate separately because deployment, fallback and JPEG differ. [Native implementation](https://github.com/mobile-next/mobilecli/blob/8d1203d5dd5049665c22e9bc4cfed43e9024c600/devices/android.go).

Mobile Next sends launch/tool PostHog events and a Scarf pixel unless `MOBILEMCP_DISABLE_TELEMETRY` is set. Leave `LOG_FILE` unset. Its selected legacy capture/input path needs local ADB only; this is source inspection, not proof against every dependency's networking. [Telemetry](https://github.com/mobile-next/mobile-mcp/blob/6d60c458f9451824dd7c11355b92b2ad316c04c6/src/server.ts).

MetaMask captures native PNG without resize: phone `screencap` to a temporary file, `adb pull`, phone cleanup, host base64 read. Host PNGs remain in a private temporary directory; delete benchmark-owned artifacts after consuming them. HOME sends keyevent `3`. These calls install no helper; hierarchy snapshots can install an instrumentation APK, hence exclude them. No telemetry was found in inspected application sources; Appium/CDP tools can network. [ADB backend](https://github.com/MetaMask/device-mcp/blob/623f808ea354c7a4d71370afd56ee0c6d7c4ec26/src/backends/adb-backend.ts), [artifact handling](https://github.com/MetaMask/device-mcp/blob/623f808ea354c7a4d71370afd56ee0c6d7c4ec26/src/utils/output-path.ts).

## Measurement contract

Same phone, USB connection, launcher, native dimensions, orientation and serial; two warmups, twenty samples, 400-ms gaps, sequential/reversed orders. Report startup/first-call separately; measure complete MCP round trips, image bytes/dimensions, p50/p95 and failures. Validate actual PNG signatures and dimensions. Retain metadata, not images. Preserve intrinsic geometry checks, disk transfers and encoding in timings; outputs match, internal work does not. Report Andromeda ADB and companion independently, never hide fallback or equate tool return with frame age. No measurements were obtained in this research.
