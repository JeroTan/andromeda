# Companion stage review

Base: `5b3f8cd`; initial head: `6abe963`.

## Standards

The listener could bind after service destruction and leak its socket. `b496246` coordinates socket ownership with destruction and always closes the owned endpoint. Five disable/re-enable cycles passed through the public companion protocol.

Follow-up review found that the regression script shadowed its parsed argument namespace. `8e3b60c` fixes the parameter name. No additional standards findings were reported.

## Spec

The phone UI inferred a connected computer from an active authorization timer and hid expiration. `162706a` reports authenticated request recency, describes authorization as readiness, and distinguishes expiration from an intentional stop. The targeted reviewer verified these fixes with no new actionable defects.

The initial reviewer runs exhausted their usage allowance. Both resumed after reset and completed the targeted review. Physical acceptance evidence is recorded separately; CI remains a required merge check.
