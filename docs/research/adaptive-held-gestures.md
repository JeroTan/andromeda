# Adaptive held gestures: investigation

Checked 2026-10-24. **Planned, not implemented.** The current companion constructs complete, preplanned strokes with `StrokeDescription(path, start, duration)` and dispatches the assembled gesture. It does not expose adaptive continuation through the public protocol. [Implementation](../../android-companion/app/src/main/java/io/andromeda/companion/ControlService.java).

Android API 26 makes continuation feasible: `willContinue=true` keeps a pointer down after a gesture completes; `continueStroke(...)` creates its next segment. The new path must start at the preceding endpoint, and its start time is relative to the new gesture. A final segment uses `willContinue=false`. [Official stroke API](https://developer.android.com/reference/android/accessibilityservice/GestureDescription.StrokeDescription).

The proposed controller needs one owner per device, an explicit bounded lease, ordered continuation updates, and a local monotonic expiry/watchdog. It must reject stale owners and late updates, reserve a terminal release segment, and handle Stop, disconnect, expiry, dispatch rejection, cancellation, and service teardown without waiting for a model response. Track release acknowledgement separately from a requested release; loss of the service cannot promise an injected terminal event. These are design requirements for the proposed capability, not existing behavior.

Android dispatch is asynchronous and can cancel ongoing gestures, including user gestures; permission and result callbacks must remain part of arbitration. [Official service API](https://developer.android.com/reference/android/accessibilityservice/AccessibilityService#dispatchGesture(android.accessibilityservice.GestureDescription,android.accessibilityservice.AccessibilityService.GestureResultCallback,android.os.Handler)).

Models and subagents may propose updates, but variable inference, transport, and scheduling delays provide no bounded response deadline. Local scheduling must own timing and expiry; it is not a hard real-time guarantee. Implementation and public-protocol validation remain future work, not a release blocker.
