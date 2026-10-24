# Tool reference

Run `andromeda tools` for the complete machine-readable input schemas. All tools return `structuredContent` with `success` and `elapsed_ms`. Executed operations also include `operation_id`; results contain `data` or an `error` with `code`, `message` and `retryable`. Failures set MCP `isError`.

`serial` selects a device. It is required unless the operator supplied `--device`. `timeout_ms` defaults to 15000 and accepts 1–120000. `adm_stop` takes no arguments. Unknown fields are rejected.

## Observation

| Tool | Additional arguments | Result |
| --- | --- | --- |
| `adm_list_devices` | None | Serial, state and ADB details for each device, including unauthorized/offline devices |
| `adm_get_device_info` | None | Model, manufacturer, Android version, screen geometry, control state |
| `adm_get_view` | `image: {max_width?, max_height?, format?, quality?}` | MCP image plus native geometry, image dimensions, byte count and scale factors |
| `adm_get_ui_tree` | None | Flat node array with parent IDs, labels, package, class, bounds and interaction flags |
| `adm_list_apps` | `include_system: bool = false` | Sorted package names |

Image formats are `png`, `jpeg`, `webp`, `webp_lossless`, `auto` and optional `avif`. Size limits preserve aspect ratio and never upscale. Requested dimensions accept 1–8192 pixels; `quality` accepts 1–100 and defaults to 80 for JPEG, lossy WebP and AVIF. PNG and lossless WebP preserve the resized pixels. PNG remains the default. `auto` encodes PNG, JPEG at the requested quality, lossy WebP at that quality and lossless WebP, then returns the smallest byte count with its actual MIME type. Quality settings are not perceptually equivalent across codecs. Auto excludes AVIF for client compatibility. AVIF requires a build with `--features avif` and an AVIF-capable client/exporter; ordinary OpenAI/Claude vision clients do not list it as supported. Password nodes have empty text and descriptions; other private content may remain visible.

`adm_get_view` reports `timings.geometry_before_ms`, `capture_ms`, `geometry_after_ms` and `image_processing_ms`, plus the chosen `mime_type`. Capture includes Android's PNG production and ADB transport; image processing includes decoding, resizing and encoding. These timings do not include client upload or model thinking. Compression reduces bytes; it does not necessarily reduce image tokens, which also depend on image dimensions.

`screen` is `{ "width": 1080, "height": 2400, "rotation": 0 }`. Rotation is a clockwise quarter-turn from the device's natural orientation. Dimensions and coordinates are native display pixels in that orientation. Multiply coordinates measured in a scaled image by the returned `scale_to_screen.x` and `.y`.

## Input

| Tool | Required arguments | Optional arguments |
| --- | --- | --- |
| `adm_touch` | `point: {x, y}`, `screen` | Common arguments |
| `adm_touch_many` | `points: [{x, y}, ...]`, `screen` | `interval_ms = 0` |
| `adm_swipe` | `swipe: {from: {x, y}, to: {x, y}, duration_ms}`, `screen` | Common arguments |
| `adm_swipe_many` | `swipes: [...]`, `screen` | `interval_ms = 0` |
| `adm_press_key` | `key` | Common arguments |
| `adm_type_text` | `text` | Common arguments |
| `adm_launch_app` | `package` | Common arguments |
| `adm_open_uri` | `uri` | Common arguments |
| `adm_batch_actions` | `actions: [...]` | `interval_ms = 0` |

Input needs operator authorization with `--allow-control`. A sequence contains 1–100 actions. `interval_ms` is a gap after the preceding action completes, from 0–10000 ms. Swipes last 1–10000 ms. A long press is an `adm_swipe` with identical `from` and `to` points and the desired duration. These sequences are sequential. Simultaneous multi-touch is available through `adm_multi_touch` with an active companion session.

Keys: `home`, `back`, `recents`, `enter`, `delete`, `tab`, `escape`, `volume_up`, `volume_down`, `power`, `wakeup`.

ADB text accepts 1–4096 printable ASCII bytes excluding `%`. It escapes shell arguments and converts spaces for Android input. Unsupported text is rejected before any batch action executes. Unicode, literal `%` and replacement mode use the paired optional companion; `backend` chooses `auto`, `adb` or `companion`.

Batch elements use an `action` discriminator:

```json
{
  "actions": [
    {"action": "launch_app", "package": "com.android.settings"},
    {"action": "pause", "duration_ms": 300},
    {"action": "press_key", "key": "back"}
  ]
}
```

Other variants are `touch`, `swipe`, `type_text` and `open_uri`, with the fields listed above. The whole batch is validated before input. Geometry is checked again before each coordinate action. Batches stop on the first execution failure. A completed action cannot be rolled back; verify the phone before retrying.

## Wait and stop

`adm_wait_for` accepts `condition` and optional `poll_ms` (100–5000, default 250). Conditions are:

```json
{"kind": "text", "value": "Display", "exact": false}
```

```json
{"kind": "resource_id", "value": "android:id/title"}
```

```json
{"kind": "pixel", "point": {"x": 20, "y": 20}, "rgb": [255, 255, 255], "tolerance": 8,
 "screen": {"width": 1080, "height": 2400, "rotation": 0}}
```

Text matches accessible text or descriptions and is case-sensitive. Pixel tolerance is the maximum absolute difference in each RGB channel. Every poll observes the device again. The operation ends on a match, error, deadline, cancellation notification, or `adm_stop`.

`adm_stop` cancels every active operation in the current server. Future calls can run normally. Already dispatched Android input may finish. One input sequence runs at a time; other input requests receive `device_busy`, while observation and stop remain responsive.

## Errors

The server distinguishes invalid arguments, disabled control, missing device selection, ADB connection/rejection/protocol failures, failed Android commands, stale geometry, image/UI failures, output limits, timeout and cancellation. `retryable` describes transport recoverability; it never authorizes blindly repeating input.

No tool treats a dispatched action as a verified task. Successful input returns `verified: false` so the agent must observe the result.

## Companion input

`adm_multi_touch` accepts `{ "screen": { "width": 1080, "height": 2400, "rotation": 0 }, "strokes": [{ "points": [{ "x": 300, "y": 700 }, { "x": 380, "y": 1050 }], "start_ms": 0, "duration_ms": 450 }] }`. Use 1–10 strokes, 1–128 points per stroke and a 10-second maximum end time. A phone-side session and pairing are required. Check the returned `companion_results` and then observe the device.

`adm_type_text` adds optional `backend` (`auto`, `adb`, `companion`) and `mode` (`insert`, `replace`). Auto chooses ADB for printable ASCII insertions and the companion for Unicode, `%`, or replacement. The companion confirms the focused field shows the requested text before it reports `verified: true` for text itself; the overall input tool still reports `verified: false` until the intended app outcome is observed.

## Scrolling capture

`adm_get_long_view` scrolls a selected list and returns one tall image containing bounded viewport observations. It requires input authorization and a running managed project with this tool enabled. Allowed actions execute without per-action approval. Example:

```json
{
  "screen": {"width": 1080, "height": 2376, "rotation": 0},
  "viewport": {"x": 20, "y": 830, "width": 1040, "height": 1150},
  "swipe": {"from": {"x": 540, "y": 1840}, "to": {"x": 540, "y": 940}, "duration_ms": 300},
  "max_pages": 4,
  "settle_ms": 300,
  "image": {"format": "jpeg", "max_width": 720},
  "timeout_ms": 30000
}
```

Both swipe endpoints must be inside the viewport. Choose the actual scrolling region, excluding animated headers and sticky controls. `max_pages` accepts 1–8 (default 4); `settle_ms` accepts 0–3000 (default 300). Output width is at most 1024 (default 720), and total height at most 8192. Per-page scaling reserves room for the requested page limit. The loop stops at the page limit or when a captured viewport repeats; a repeat can also mean a failed scroll or loading screen, so it does not prove the end of the list. Geometry is checked before scrolling and around every capture; Stop, cancellation, pause and deadlines interrupt the loop.

`pages` maps each tile to its image offset and native viewport scale. Tiles may overlap and are not a seamless stitched screenshot. The list stays at the final position. Earlier pages are historical observations: get a fresh view before tapping an item. The tool does not promise to discover all content in an unbounded list.

## Image-format compatibility and measurements

For routine chat screenshots, try `{"image":{"format":"auto","max_width":720,"quality":80}}`. Choose `webp_lossless` when exact resized pixels matter. AVIF is an explicit export/compatible-client option, not an automatic chat candidate. Supported image types are documented by [OpenAI](https://developers.openai.com/api/docs/guides/images-vision) and [Claude](https://platform.claude.com/docs/en/build-with-claude/vision). See the [codec comparison](../benchmarks/README.md) for measured bytes and processing time rather than assuming one format always wins.

## Capture acquisition

`adm_get_view` keeps the same image options and native-coordinate metadata. With an updated, paired Android 11+ bridge it prefers the accessibility screenshot API, which checks geometry around acquisition on the phone and scales/encodes before transfer. PNG and lossless WebP retain their fidelity modes; JPEG/WebP already encoded in the requested mode are validated and passed through rather than recompressed. `auto` and optional AVIF acquire lossless PNG before host-side selection/encoding.

Results identify `capture_backend` (`companion` or `adb`). ADB fallback also identifies `capture_fallback_code` when the attempted API is unavailable or rate-limited. Explicit protected-content errors and invalid response data do not trigger another backend. Full-display protected content may instead arrive as black pixels; capture success does not prove that protected UI content is visible.

`timings.capture_ms` measures acquisition and device/ADB transport at the host. Companion results add `device_capture_ms`, `device_image_processing_ms`, and `api_timestamp_age_ms`; the latter uses the device uptime clock and is not original presentation age. The two host geometry timings are zero when `geometry_source` is `companion_checked`; the Android service performed those checks inside the measured acquisition request. Host decode/validation/optional conversion remains `image_processing_ms`. Model thinking and remote upload are outside these timings.

For standalone diagnostics, `--capture-backend adb` forces the original path and `--capture-backend companion` requires the new API without fallback. The SDK equivalent is `Controller::with_capture_backend`. Managed MCP proxies use their desktop host's configuration. The accessibility API rate-limits closely spaced captures; use a supported cadence and inspect fallback/error metadata. This remains on-demand capture, not a video stream. `adm_get_long_view`, pixel waits and the current live-preview stream retain their ADB capture paths.
