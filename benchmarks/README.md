# Reproducible measurements

Run the device workloads while the selected phone displays a stable, non-sensitive screen. The harness sends no input unless `--allow-input` is set, keeps images in memory, and saves timing/byte counts rather than screenshots or device serials.

```sh
cargo run --locked --release --example benchmark_capture -- \
  --device YOUR_SERIAL --adb /path/to/adb --warmup 2 --samples 20 \
  --stream-seconds 60 --stream-interval-ms 100 --allow-input \
  --output benchmarks/results/device-unique-name.json
```

The output refuses to overwrite an existing file. Build in release mode for performance comparisons. The JSON records per-sample elapsed time, failures, bytes, p50/p95, device model/API/geometry, transport class, emulator flag, battery level/temperature, host OS/architecture, sample count, warm-up, stream settings and the input flag. Record the exact USB/Wi-Fi path, thermal state and competing host work alongside physical-device results; the harness does not control throttling.

## Workloads

| Workload | Metric | What it includes |
| --- | --- | --- |
| `direct_adb_png` | request latency | Direct ADB shell-v2 `screencap -p`; native PNG, no decode |
| `adb_cli_png` | request latency | External `adb shell screencap -p`; includes host process startup |
| `controller_native_png` | request latency | Geometry checks, bounded PNG decode and native PNG output |
| `controller_jpeg_720` | request latency | Geometry checks, resize to 720 px wide and JPEG quality 80 |
| `persistent_stream_png` | frame interval | One shell-v2 capture loop; complete PNG frames are parsed from the stream |
| `controller_press_home` | input latency | One `KEYCODE_HOME` through the normal validation/transport path; opt-in |

`persistent_stream_png` reports `fps` from the measured frame timestamps. Its `elapsed_ms` values are frame intervals, not input latency. `controller_press_home` is a safe transport check, not a gesture-accuracy or task-completion test. The harness samples host CPU and RSS with `ps`; CPU averaging differs by platform and includes image processing, so it is not a device-power measurement.

The raw direct-ADB and external-ADB cases issue the same PNG command. The controller cases additionally validate geometry and decode the image; scaled JPEG includes resizing/encoding. These are distinct workloads, not interchangeable speed claims. This original harness does not cover gesture accuracy, completion/recovery, model/tool overhead or matched open-source alternatives. Later sections add dedicated interaction and comparison measurements; real-client overhead is recorded in the release ledger.

## Committed raw runs

- [emulator-api37-2026-10-23-stream.json](results/emulator-api37-2026-10-23-stream.json): Android 17/API 37 emulator, 20 samples per workload, 2 warm-ups, zero failures, release build, battery 100%, 25.0 °C, emulator transport. Direct ADB PNG p50 413 ms / p95 437 ms; external ADB CLI 423/442; controller native PNG 462/486; controller 720px JPEG 482/515; persistent stream p50 frame interval 526 ms / p95 543 ms at 1.89 fps; Home input 20.8/43.3 ms.
- [oneplus-cph2653-2026-10-23.json](results/oneplus-cph2653-2026-10-23.json): OnePlus CPH2653, Android 16/API 36, 1080×2376, USB, 20 samples per workload, 2 warm-ups, zero failures, release build, battery 95%, 37.0 °C. Direct ADB PNG p50 1812 ms / p95 2049 ms; external ADB CLI 1864/1982; controller native PNG 1957/2112; controller 720px JPEG 1995/2034; persistent stream p50 frame interval 1937 ms / p95 1959 ms at 0.52 fps; Home input 29.3/36.0 ms.

These are device-specific baselines, not product-comparison results. The physical phone's capture path is much slower than the emulator's, so do not combine the two rows into a speed claim. Re-run on the target hardware and keep the raw JSON next to any summary.

External ADB is a transport baseline. The matched open-source comparison below adds pinned versions and raw results for a native-PNG workload. No fastest-product claim is established here.

## Image codecs

Run the offline comparison against one operator-provided PNG:

```sh
cargo run --locked --release --features avif --example benchmark_image_formats -- \
  /path/to/private-capture.png --max-width 720 --quality 80 --samples 3 \
  --output benchmarks/results/unique-codec-run.json
```

The report contains dimensions, byte counts and processing times, never image pixels or the input path. It refuses to overwrite an existing report. AVIF uses speed 10. `quality: 80` is codec-specific, not a matched perceptual-quality guarantee. The public capture encoder bounds decoding and retains coordinate scaling; automatic mode compares PNG/JPEG/lossy WebP/lossless WebP byte counts and excludes AVIF for chat compatibility.

A fixed 1080×2376 game-result capture resized to 720 px on macOS arm64, release build, three samples after one warm-up, produced the following [raw results](results/image-formats-lossy-webp-2026-10-23.json):

| Encoding | Bytes | Processing p50 |
| --- | ---: | ---: |
| PNG | 661,366 | 23.7 ms |
| JPEG, quality 80 | 135,712 | 25.6 ms |
| WebP, quality 80 | 56,360 | 56.8 ms |
| Lossless WebP | 640,032 | 24.0 ms |
| Auto | 56,360 (WebP) | 71.8 ms |
| AVIF, quality 80, speed 10 | 68,894 | 308.1 ms |

WebP won on byte size in this sample; AVIF is not universally smaller. The earlier [lossless-WebP-only run](results/image-formats-2026-10-23.json) records the result before adding lossy WebP. These are fixed-source image-processing measurements, excluding phone capture, MCP transport and model time. They do not establish equal visual quality, device power use, or performance on other images. Raw screenshots remain private and are excluded from Git.

### Compare capture backends through MCP

Build the release CLI and an updated companion, pair the chosen device, and leave a non-sensitive static screen visible. This harness only calls `adm_get_view`; it never dispatches input and never saves image pixels:

```sh
cargo build --locked --release --bin andromeda
python3 benchmarks/benchmark_mcp_capture.py \
  --server target/release/andromeda --profile release \
  --device YOUR_SERIAL --state-dir YOUR_PRIVATE_STATE_DIRECTORY \
  --samples 20 --warmup 2 --output YOUR_NEW_RESULT_FILE.json
```

Repeat with `--reverse` and a different output path to inspect ordering/thermal effects. Each source is explicit, so fallback cannot silently enter a comparison. Native PNG, 720-pixel JPEG and 720-pixel WebP share requested dimensions and nominal quality across sources; Android and Rust encoders/resizers are not perceptually calibrated. The report separates the full stdio/JSON MCP round trip from controller execution and records warm-ups, failures, timing components, bytes, CPU/RSS samples, battery/temperature, host/device metadata and the server binary digest. API timestamp age is not original presentation age. Client/model inference and remote upload are excluded.

On 2026-10-24, an Android API 37 emulator using SwiftShader and a static Andromeda screen produced these full MCP round trips with a release server. Each cell is p50 / p95 in milliseconds; each workload used two warm-ups and twenty measured samples. No compilation or tests ran during either measurement.

| Requested image | ADB first | Companion second | Companion first | ADB second |
|---|---:|---:|---:|---:|
| Native PNG | 156.16 / 161.47 | 115.99 / 117.36 | 115.22 / 116.47 | 154.33 / 158.04 |
| JPEG, width 720, quality 80 | 170.91 / 173.68 | 52.43 / 55.63 | 51.37 / 52.67 | 168.96 / 172.16 |
| WebP, width 720, quality 80 | 196.01 / 200.76 | 92.81 / 94.78 | 92.13 / 93.07 | 194.68 / 198.38 |

All 240 measured requests succeeded. Raw [forward](results/accessibility-emulator-2026-10-24-forward.json) and [reverse](results/accessibility-emulator-2026-10-24-reverse.json) reports include timing components, bytes and artifact hashes. These results support an improvement on this emulator; they do not establish physical-phone latency, perceptual equivalence, model response time, live-preview performance or superiority over another product. Physical capability-upgrade and timing checks were subsequently completed below.

The same protocol on the physical OnePlus CPH2653 (Android 16/API 36, USB, static Andromeda screen) produced these results after its bridge update. The existing accessibility grant and pairing survived the update.

| Requested image | ADB first | Companion second | Companion first | ADB second |
|---|---:|---:|---:|---:|
| Native PNG | 428.03 / 470.00 | 264.99 / 311.10 | 262.49 / 273.11 | 430.12 / 453.73 |
| JPEG, width 720, quality 80 | 461.26 / 486.73 | 79.78 / 90.91 | 72.92 / 91.07 | 457.65 / 482.61 |
| WebP, width 720, quality 80 | 495.21 / 537.87 | 199.74 / 212.71 | 199.66 / 210.55 | 499.68 / 511.23 |

All 240 physical requests succeeded; see [forward](results/accessibility-oneplus-2026-10-24-forward.json) and [reverse](results/accessibility-oneplus-2026-10-24-reverse.json) metadata. The installed release desktop's managed MCP connection also returned a companion image, and its on-demand preview displayed 94 ms in one acceptance observation. These measurements establish an improvement for this phone/build/screen. They are not general latency guarantees or measurements of model reasoning, live video, game completion, perceptual equivalence or other products.

## Native gesture endpoints and deterministic recovery

`benchmark_interaction.py` requires explicit `--allow-input` and refuses to act unless it observes Andromeda's private debug gesture pad. Build/install the debug companion, pair it, start its managed or standalone session, and open **Connection controls → Open gesture test pad**. Then run:

```sh
python3 benchmarks/benchmark_interaction.py \
  --server target/release/andromeda --profile release --transport usb \
  --device YOUR_SERIAL --state-dir YOUR_PRIVATE_STATE_DIRECTORY \
  --adb /path/to/adb --samples 12 --allow-input --output NEW_RESULT.json
```

The pad records Android MotionEvent raw start/end coordinates and event durations. The harness compares them to requested native coordinates rather than treating a successful tool result as proof of arrival. It also opens/closes a test dialog three times and recovers with Back three times. Observation retries are bounded and counted; input is never retried. These fixtures do not establish success rates for arbitrary apps or model reasoning.

On 2026-10-24, the [OnePlus USB run](results/interaction-oneplus-2026-10-24.json) completed all 48 samples: 12 taps, 12 diagonal swipes, 12 simultaneous two-finger gestures and 12 screenshot-then-tap sequences. Every recorded start/end point matched the requested coordinate (maximum error 0 px). Swipes requested 250, 500 or 1000 ms; maximum event-duration error was 11 ms for ADB swipes and 0 ms for the tested companion two-finger gestures. Intermediate trajectory accuracy and other pointer counts were not measured.

Tap MCP acknowledgement p50/p95 was **64.86/93.66 ms**. Scripted JPEG-720 capture through tap acknowledgement was **163.57/184.51 ms**, excluding model thinking and the later completion check. All six dialog tasks completed with zero input retries. The [emulator run](results/interaction-emulator-2026-10-24.json) also completed 48/48 endpoint checks and six tasks; its maximum ADB swipe-duration error was 36 ms. Two transient `ui_unavailable` observations were retried on the emulator; none occurred on the phone. No warm-ups were discarded in these interaction runs. The debug measurement view adds its own event-processing work.

Early harness probes exposed a coordinate-measurement error during window animation, uppercase native dialog labels, and transient missing accessibility roots during dialog transitions. The final harness uses the MotionEvent's raw coordinate offset, case-insensitive fixture labels and bounded observation polling. Those pilot failures were harness defects and are not counted as successful final samples. Raw failed probes retain the [animated-offset/dialog check](results/development/interaction-raw-offset-dialog-2026-10-24.json), [native label mismatch](results/development/interaction-dialog-case-2026-10-24.json), and [transient root failure](results/development/interaction-transient-root-2026-10-24.json).

## Matched open-source native-PNG comparison

The [comparison plan](../docs/research/mcp-comparison-plan.md) pins Mobile Next `@mobilenext/mobile-mcp@1.0.12` and MetaMask `@metamask/device-mcp@0.5.0` with their source revisions. `comparison/package-lock.json` pins the resolved dependency graph. Install it with `npm ci --ignore-scripts --prefix benchmarks/comparison`, then run:

```sh
python3 benchmarks/compare_mcp.py \
  --andromeda target/release/andromeda --profile release --transport usb \
  --state-dir YOUR_PRIVATE_STATE_DIRECTORY --packages benchmarks/comparison \
  --device YOUR_SERIAL --adb /path/to/adb --samples 20 --warmup 2 \
  --surface 'Static non-sensitive screen' --output NEW_RESULT.json
```

Repeat with `--reverse` and a new output. Optional `--allow-input` adds a separate repeated-Home acknowledgement phase after capture. It first prepares the launcher with Home; return the phone to its prior app afterward. Mobile Next telemetry is disabled, MetaMask uses an isolated empty working/temp directory, and all host screenshots are removed after each workload. No recording or helper APK is installed by the selected comparison paths.

Both capture runs below used the same physical OnePlus, USB connection, 1080×2376 geometry, static debug pad, release Andromeda server, Node 24, two warm-ups and twenty measured samples per implementation. Actual PNG signatures, chunk checksums, decompression and native dimensions were verified outside measured latency. Each cell is p50 / p95 in milliseconds.

| Native-PNG implementation | Forward order | Reverse order |
|---|---:|---:|
| Andromeda, companion API | 196.18 / 213.91 | 175.21 / 223.03 |
| Andromeda, forced ADB | 383.63 / 415.11 | 372.33 / 406.59 |
| Mobile Next, ADB compatibility mode | 499.97 / 526.52 | 504.94 / 539.00 |
| MetaMask, default ADB | 434.60 / 460.47 | 440.99 / 477.67 |

All **160 measured captures** succeeded. Mobile Next's default backend returns resized JPEG and was **not measured** here; compatibility mode is explicit so output format/dimensions match. This is not a ranking of default product performance. Geometry validation, disk writes, subprocesses and encoding differ and remain inside each tool's measured work. MetaMask's own remote temporary-file cleanup is included. CPU/RSS fields cover only server processes, excluding child ADB/mobilecli processes and client parsing; they are not total resource-use comparisons.

The reverse run also measured 20 Home acknowledgements per configuration: Andromeda 36.45/40.64 ms with ADB capture selected and 36.56/40.22 ms with companion capture selected; both use the **same ADB key-input path**. Mobile Next compatibility mode measured 62.16/67.52 ms and MetaMask 56.59/59.79 ms. All 80 input samples succeeded. This input phase has one ordering only and does not measure when the launcher finished rendering.

Raw [forward](results/mcp-comparison-oneplus-2026-10-24-forward.json) and [reverse](results/mcp-comparison-oneplus-2026-10-24-reverse.json) files retain startup, warm-ups, every sample, environment, battery and lock/binary hashes. USB disconnected after both measurements; after the operator reconnected it, no temporary comparison PNGs remained on the phone, and the original Running project was restored. These results support improvement for this native-PNG workload on this phone, not a fastest-product or general reliability claim.
