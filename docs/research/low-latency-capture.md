# Low-latency capture investigation

Updated 2026-10-23. The current Andromeda implementation does **not** establish sub-500 ms or sub-100 ms fresh-frame delivery on the physical phone.

## Measurements and metric boundaries

The release capture baseline on the OnePlus CPH2653, Android 16/API 36, USB, 1080×2376, measured roughly 2 seconds per PNG-derived observation; see the [raw baseline](../../benchmarks/results/oneplus-cph2653-2026-10-23.json). Later interactive observations varied with screen content. One instrumented call spent 787 ms in Android PNG capture/ADB transfer and 78 ms decoding/resizing/encoding; geometry checks and other controller work brought it to 979 ms. WebP reduced payload bytes, not the Android capture time.

A three-sample exploratory external-ADB comparison returned native raw frames in 679–726 ms (10,264,336 bytes) and PNG in 1,032–1,160 ms (about 1.25 MB). This was a dynamic screen, not a controlled matched-frame benchmark. Raw transport alone did not meet 500 ms. [Exploratory samples](../../benchmarks/results/oneplus-raw-vs-png-2026-10-23.json).

Keep these separate:

- **Cold start:** deployment, connection, encoder setup, decoder initialization and first usable frame.
- **Frame interval/FPS:** spacing between delivered frames. A 33 ms interval does not prove a 33 ms-old image.
- **Frame age/delivery:** time from device presentation/capture to usable host pixels, requiring correlated clocks or a controlled visible-response experiment.
- **Tool return:** waiting for an eligible frame plus image conversion, framing and client transport. Model processing happens after delivery and is a separate measurement.

## Candidate: pinned scrcpy video transport

The official project advertises 35–70 ms mirroring latency and approximately one-second first-image startup. Those are upstream claims, not Andromeda results. [scrcpy README](https://github.com/Genymobile/scrcpy/tree/v5.0.1)

scrcpy uses Android MediaCodec video output and a host decoder, rather than repeated PNG screenshots. Its server runs through `app_process` as the authorized shell user. Client and server versions must match. Static-screen frame repetition and rotation handling matter for a useful fresh-frame cache. [Tagged developer documentation](https://github.com/Genymobile/scrcpy/blob/v5.0.1/doc/develop.md)

Pin and verify both the server artifact and protocol to **v5.0.1**. The tagged Java writer uses big-endian 12-byte session/media headers: bit 63 marks a session packet; media uses bit 62 for configuration and bit 61 for a keyframe, with a 32-bit packet length. Use the tagged source as the wire authority; prose still contains older-version examples. [Streamer.java](https://github.com/Genymobile/scrcpy/blob/v5.0.1/server/src/main/java/com/genymobile/scrcpy/device/Streamer.java)

## Proposed integration and acceptance

This is a design proposal, not shipped functionality:

1. Establish one operator-enabled capture session per selected device, with audio and scrcpy control disabled. Retain normal Andromeda authorization for all input.
2. Own the server process, sockets and temporary artifact explicitly. Test disconnect, startup failure and cleanup on an isolated emulator before physical-device use. Bound startup, packet size, frame size and decoder concurrency.
3. Decode continuously into a single latest-frame slot, discarding superseded frames. Carry source timestamps, session generation, dimensions and local receive time. Never allow a FIFO backlog to masquerade as a fresh screen.
4. Fence observations after input: a pre-action cached frame cannot verify the resulting screen. Invalidate frames on rotation, disconnect, Stop and device/project changes. Reject stale frames instead of claiming an old cache is a fast capture.
5. Measure cold start, warm request time, frame age and controlled visible response separately, with raw samples and failure counts. Target sub-500 ms first; treat sub-100 ms as an unverified warm-stream goal.

## Rejected experiment on the physical phone

A bounded native `screenrecord --output-format=h264` pipe produced no usable frames on this OnePlus. While asleep it reported `INVALID_LAYER_STACK`; while awake its processes outlived the requested duration. The host pipeline ended, but the phone's `screen_record` policy denied ADB signals to the two stalled native processes. The operator restarted the phone to clear them; verification found zero remaining test recorders.

The throwaway harness is excluded from Git. **Do not repeat native screenrecord experiments on this device.** Choose a separately owned backend and verify its teardown on an emulator first. No sub-100/500 ms result was obtained from this failed experiment, and no captured video was retained.
