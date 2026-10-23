# Companion stage review

Base: `5b3f8cd`; initial head: `6abe963`.

## Standards

The listener could bind after service destruction and leak its socket. `b496246` coordinates socket ownership with destruction and always closes the owned endpoint. Five disable/re-enable cycles passed through the public companion protocol.

Follow-up review found that the regression script shadowed its parsed argument namespace. `8e3b60c` fixes the parameter name. No additional standards findings were reported.

## Spec

The phone UI inferred a connected computer from an active authorization timer and hid expiration. `162706a` reports authenticated request recency, describes authorization as readiness, and distinguishes expiration from an intentional stop. The targeted reviewer verified these fixes with no new actionable defects.

The initial reviewer runs exhausted their usage allowance. Both resumed after reset and completed the targeted review. Physical acceptance evidence is recorded separately; CI remains a required merge check.


## Follow-up review of `216b19d`

Both review axes identified preview lifecycle issues in the inherited integration: unbounded startup, immediate errors racing UI startup, and stale events affecting a replacement stream. Spec review also found a minimum-1-FPS display that overstated slow capture; standards review found launcher flags consumed after `--`. The follow-up fixes add a startup deadline and public device-API regression, per-preview event identities, event-listener readiness, cancellation/owner checks, fractional frame rates and a public launcher regression. Current local workspace tests pass. Hosted Actions is disabled at the operator's request; platform failures from the last run are recorded rather than reported as resolved remotely.

The final Standards and Spec passes reviewed `216b19d` → `60285e9` independently and found no actionable regressions. Both confirmed startup bounds, event identity/listener readiness, cancellation ownership, fractional FPS and launcher flag handling. They performed no device actions or CI runs.
