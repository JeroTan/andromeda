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

Image formats are `png` and `jpeg`. Size limits preserve aspect ratio and never upscale. Requested dimensions accept 1–8192 pixels; JPEG quality accepts 1–100 and defaults to 80. PNG is the default. Password nodes have empty text and descriptions; other private content may remain visible.

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

Input needs operator authorization with `--allow-control`. A sequence contains 1–100 actions. `interval_ms` is a gap after the preceding action completes, from 0–10000 ms. Swipes last 1–10000 ms. These sequences are sequential. Simultaneous multi-touch is available through `adm_multi_touch` with an active companion session.

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
