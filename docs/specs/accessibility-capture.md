# Bounded accessibility capture

This implements the original efficient-capture/performance requirement and the user's request to separate device capture, encoding, tool overhead and model time.

- Prefer fresh Android accessibility screenshots when the optional bridge is paired and capable; ordinary ADB capture remains available for older/unpaired devices. Do not use native screenrecord or require a persistent recorder process.
- Preserve existing MCP image options, default PNG fidelity, native screen coordinates and scaling metadata. Use the requested supported codec and dimensions on-device to reduce transfer cost. Do not decode/re-encode an already matching lossy codec.
- Validate image bytes, MIME, dimensions, native geometry and rotation before delivery. Android checks geometry before/after acquisition; successful companion frames do not need additional ADB geometry round trips. No stale frame cache.
- Capture is observation: paused managed projects can still observe when their tool/device scope allows it. Keep pairing credentials private and retain explicit Stop/cancellation fencing for active requests.
- Bound capture concurrency, API wait, native pixels, encoded response, host decode allocation and dimensions. Close every hardware buffer and bitmap, including late callbacks after timeout/EOF/Stop.
- Handle Android rate limits and missing capability honestly. Auto may fall back once for unavailable capture; explicit companion selection must report its error. Protected-screen or invalid-data failures must not trigger another backend.
- Record the backend, transport/acquisition time, device capture/processing and host image processing separately. API timestamp age is not original presentation age.
- Test same-component capability upgrade on an isolated emulator before the physical phone. Keep device/image samples private, retain only benchmark metadata, and separate emulator results from physical-device evidence. Compare identical requested dimensions/nominal quality; codec implementations are not perceptually calibrated.
- Do not enable or run hosted Actions. Do not disrupt game workflows or repeat the rejected native recorder experiment. A passing simulator/emulator result alone does not close the release performance gates.
