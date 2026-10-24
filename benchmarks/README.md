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

The raw direct-ADB and external-ADB cases issue the same PNG command. The controller cases additionally validate geometry and decode the image; scaled JPEG includes resizing/encoding. These are distinct workloads, not interchangeable speed claims. The harness does not yet cover gesture accuracy, completion/recovery, model/tool overhead or matched open-source alternatives. The release ledger retains those gates.

## Committed raw runs

- [emulator-api37-2026-10-23-stream.json](results/emulator-api37-2026-10-23-stream.json): Android 17/API 37 emulator, 20 samples per workload, 2 warm-ups, zero failures, release build, battery 100%, 25.0 °C, emulator transport. Direct ADB PNG p50 413 ms / p95 437 ms; external ADB CLI 423/442; controller native PNG 462/486; controller 720px JPEG 482/515; persistent stream p50 frame interval 526 ms / p95 543 ms at 1.89 fps; Home input 20.8/43.3 ms.
- [oneplus-cph2653-2026-10-23.json](results/oneplus-cph2653-2026-10-23.json): OnePlus CPH2653, Android 16/API 36, 1080×2376, USB, 20 samples per workload, 2 warm-ups, zero failures, release build, battery 95%, 37.0 °C. Direct ADB PNG p50 1812 ms / p95 2049 ms; external ADB CLI 1864/1982; controller native PNG 1957/2112; controller 720px JPEG 1995/2034; persistent stream p50 frame interval 1937 ms / p95 1959 ms at 0.52 fps; Home input 29.3/36.0 ms.

These are device-specific baselines, not product-comparison results. The physical phone's capture path is much slower than the emulator's, so do not combine the two rows into a speed claim. Re-run on the target hardware and keep the raw JSON next to any summary.

External ADB is a transport baseline. Representative open-source MCP comparisons still need matched workloads, pinned versions and their own raw results. No fastest-product claim is established here.

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
