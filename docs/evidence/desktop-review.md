# Desktop stage review

Base: `1ca97bec7ce9738abf1e4aefbd96357802993eec` (merged core).
Initial reviewed head: `72be26c`.

## Standards

Two findings:

- An earlier Resume waiting for the settings lock could clear a later Stop. Stop now advances a generation under the same host lock used to resume. Resume carries the generation the operator saw; an obsolete request fails. A focused public managed-interface test reproduces the ordering and passes with the fix.
- Approvals identified a project only by its folder name. Approval payloads and dialogs now include its canonical root.

No additional actionable baseline smells were reported.

## Spec

Three findings:

- A saved device change left the previous phone's image visible. Preview results now carry their serial; saving a changed serial clears the image and invalidates pending captures.
- Projects sharing a folder name were indistinguishable in approval details. Addressed by the canonical project root above.
- The agent panel lacked MCP connection state. Managed proxies now register bounded, authenticated connection leases; the desktop reports connected clients and instruction-loading failures. Unresponsive connections expire within 15 seconds. A focused test verifies connect/drop status through the public proxy and host interfaces.

The full product remains unfinished. This review does not claim acceptance for the phone UI, npm launcher, benchmarks or release.

The standards reviewer verified both fixes at `63536e2`. The spec follow-up verified preview identity, project identity and stop ordering, then found two connection-state issues: a proxy was counted before MCP initialization, and a healthy heartbeat cleared another client's initialization error. Tracking now starts from the successful MCP initialized notification, and errors clear only on a fresh successful initialization. The focused test fails with each previous behavior and passes with both fixes. The final targeted spec review verified both fixes at `151cd43` with no new actionable defects. [CI run 39759622974](https://github.com/JeroTan/andromeda/actions/runs/39759622974) passed all seven jobs, including all package uploads. An actual packaged-app check also verified the live connected/disconnected indicator.

Initial findings: Standards 2; Spec 3. Follow-up findings: Spec 2.
