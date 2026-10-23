# Examples and demonstrations

These prompts are starting points for a connected Android device. Start with observation only. Add `--allow-control` (or enable input in the desktop app) only when the client can show an approval prompt for each input request.

Andromeda never treats a dispatched action as proof of success. Every example ends by observing the phone again and reporting what changed.

## 1. Observe without touching

```text
Use Andromeda to list the connected Android devices and select <serial>.
Describe the current screen without sending input. Report the native screen
geometry, the visible app, and any fields that were redacted.
```

Expected sequence: `adm_list_devices` → `adm_get_device_info` → `adm_get_view` or `adm_get_ui_tree`.

## 2. Change a safe setting and restore it

```text
Using Andromeda, open Android Settings, find the Dark mode setting, and record
its current state. Toggle it once, verify that the UI changed, then restore the
original state. Use the screen geometry returned by Andromeda. Ask me before
touching anything outside Settings.
```

Expected sequence: observe → launch Settings → wait for text → tap → observe → tap → observe. A successful tool call only means input was dispatched; the final observation is the acceptance check.

## 3. Batch with an explicit pause

```json
{
  "actions": [
    { "action": "launch_app", "package": "com.android.settings" },
    { "action": "pause", "duration_ms": 300 },
    { "action": "press_key", "key": "back" }
  ]
}
```

The whole batch is validated before the first action. A failure stops the batch, and completed actions cannot be rolled back.

## 4. Wait for a condition

```json
{
  "condition": { "kind": "text", "value": "Display", "exact": false },
  "poll_ms": 250
}
```

Use a client deadline longer than the requested Andromeda `timeout_ms`. `adm_stop` cancels the wait and any other active operation.

## 5. Pixel check after a visual change

```json
{
  "condition": {
    "kind": "pixel",
    "point": { "x": 20, "y": 20 },
    "rgb": [255, 255, 255],
    "tolerance": 8,
    "screen": { "width": 1080, "height": 2400, "rotation": 0 }
  },
  "poll_ms": 250
}
```

Pixel coordinates are native display pixels. Re-read the geometry if the phone rotates or the display size changes.

## 6. Companion gesture and Unicode text

```json
{
  "screen": { "width": 1080, "height": 2400, "rotation": 0 },
  "strokes": [
    {
      "points": [
        { "x": 300, "y": 700 },
        { "x": 380, "y": 1050 }
      ],
      "start_ms": 0,
      "duration_ms": 450
    }
  ]
}
```

This requires a paired phone and an active companion session. For Unicode text, use `adm_type_text` with `backend: "companion"`; ADB text is limited to printable ASCII.

## Demonstration checklist

- Record the serial, Android version, screen geometry and USB/network transport.
- Observe before and after every meaningful action.
- Keep approval review enabled for sensitive actions.
- Stop with `adm_stop` or **Stop & pause** when the screen is unexpected.
- Do not claim task completion from `success: true`; verify the intended UI state.
- Do not put passwords, pairing secrets or private screenshots in a public demo.

## Hold one finger while another swipes

`adm_multi_touch` can schedule both motions locally in one request. For example, on a deliberately chosen test surface with this observed geometry:

```json
{
  "screen": {"width": 1080, "height": 2400, "rotation": 0},
  "strokes": [
    {"points": [{"x": 300, "y": 900}], "start_ms": 0, "duration_ms": 3000},
    {"points": [{"x": 700, "y": 1700}, {"x": 700, "y": 1000}], "start_ms": 800, "duration_ms": 500}
  ]
}
```

The first finger stays down while the second moves. A paired phone and active companion session are required. Timing is predetermined and runs on the phone; it does not wait for another model response. Changing the path in response to new observations while a finger remains held is a separate planned capability, requiring a bounded owner/lease and release handling. Observation or dispatch success alone does not verify the intended app outcome.
