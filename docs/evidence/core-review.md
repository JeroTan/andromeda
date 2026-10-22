# Core implementation review

Baseline: `f128cac7757986afc06821e947574e092b66e3f6`. Initial implementation: `a22d088`. The standards and specification axes were reviewed independently by parallel review agents, as required by the repository's code-review skill.

## Standards

One hard violation: unrestricted dependency debug logging could expose full MCP requests and responses, contrary to CONTRIBUTING.md's ban on logging text, screenshots and device identifiers. A synthetic `RUST_LOG=debug` session reproduced the leak. No actionable baseline code smells were reported.

The fix restricts stderr logging to the `andromeda::audit` target. The `debug_logging_never_records_mcp_payloads_or_device_identifiers` subprocess test failed on the initial implementation and passes with the filter, including `RUST_LOG=trace`.

## Spec

Two implementation defects were identified:

1. UI trees used geometry read before the dump without checking the hierarchy rotation or the geometry afterward. This could return a portrait frame alongside landscape node bounds, violating safe orientation handling. The controller now rejects inconsistent observations. The public-boundary regression failed before the fix and passes after it.
2. Failed Android commands discarded stdout, although `am start -W` can place its useful error explanation there. The transport now preserves a bounded 4096-byte excerpt in the caller's error. The failed-launch regression failed before the fix and passes after it.

The review also identified a missing test: `adm_stop` coverage did not establish that an MCP cancellation notification closes an in-flight ADB stream. A separate protocol test now verifies that path and checks that the server remains usable.

The full specification still includes companion/multitouch, persistent capture, per-action approval, benchmark comparisons, remaining compatibility gates and release/cleanup. These are tracked in the [evidence ledger](../implementation-status.md); this core review does not declare them complete. No scope creep was identified.

Initial findings: Standards 1 (payload logging); Spec 2 (inconsistent rotation, lost diagnostics), plus one missing verification path. All three defects have regression coverage; final fix review and CI evidence are recorded with the PR.
