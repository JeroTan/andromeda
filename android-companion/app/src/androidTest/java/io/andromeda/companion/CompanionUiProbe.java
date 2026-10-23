package io.andromeda.companion;

import android.app.Activity;
import android.app.Instrumentation;
import android.app.UiAutomation;
import android.content.Intent;
import android.graphics.Rect;
import android.os.Bundle;
import android.os.SystemClock;
import android.view.accessibility.AccessibilityNodeInfo;
import org.json.JSONArray;
import org.json.JSONObject;
import java.util.ArrayDeque;

/** Exercises public Android UI; the permission-state scenario deliberately suspends services. */
public final class CompanionUiProbe extends Instrumentation {
    private Bundle arguments;
    @Override public void onCreate(Bundle arguments) { super.onCreate(arguments); this.arguments = arguments; start(); }
    @Override public void onStart() {
        Bundle result = new Bundle();
        try {
            boolean permissionState = "permission-state".equals(arguments.getString("scenario"));
            // Never suppress accessibility services from the probe; Android can clear the enabled
            // service list while suppressing, which would hide the permission state under test.
            UiAutomation automation = getUiAutomation(UiAutomation.FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES);
            boolean pad = "pad".equals(arguments.getString("screen"));
            Intent launch = new Intent().setClassName("io.andromeda.companion", pad ? "io.andromeda.companion.ProbeActivity" : "io.andromeda.companion.MainActivity").addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | (permissionState ? Intent.FLAG_ACTIVITY_CLEAR_TASK : Intent.FLAG_ACTIVITY_CLEAR_TOP));
            if (permissionState) launch.putExtra("io.andromeda.companion.TEST_SERVICE_DISCONNECTED", true);
            getTargetContext().startActivity(launch);
            AccessibilityNodeInfo root = waitForRoot(automation, "io.andromeda.companion");
            if (permissionState) {
                // The visible title is above the fold; the detailed status line lives in the
                // scrollable panel below it.
                awaitLabel(automation, "Permission set.\nConnecting…");
                click(automation, "Check connection");
                awaitLabel(automation, "Permission set.\nConnecting…");
                result.putString("status", "passed");
                result.putBoolean("permission_state", true);
                finish(Activity.RESULT_OK, result);
                return;
            }
            if ("timer-persistence".equals(arguments.getString("scenario"))) {
                click(automation, "Settings");
                click(automation, "Session");
                awaitLabel(automation, "15 minutes, selected");
                click(automation, "Off");
                awaitLabel(automation, "Off, selected");
                result.putString("status", "passed");
                result.putBoolean("timer_persistence", true);
                finish(Activity.RESULT_OK, result);
                return;
            }
            if ("session-settings".equals(arguments.getString("scenario"))) {
                click(automation, "Settings");
                click(automation, "Session");
                awaitLabel(automation, "Off, selected");
                click(automation, "15 minutes");
                awaitLabel(automation, "15 minutes, selected");
                click(automation, "Back to Settings");
                awaitLabel(automation, "Session timer · 15 minutes");
                click(automation, "Session");
                awaitLabel(automation, "15 minutes, selected");
                click(automation, "Back to Settings");
                click(automation, "Back to home");
                result.putString("status", "passed");
                result.putBoolean("session_settings", true);
                finish(Activity.RESULT_OK, result);
                return;
            }
            int scrolls = Math.min(5, Integer.parseInt(arguments.getString("scroll", "0")));
            for (int i = 0; i < scrolls; i++) {
                AccessibilityNodeInfo scroll = scrollable(root);
                if (scroll == null || !scroll.performAction(AccessibilityNodeInfo.ACTION_SCROLL_FORWARD)) break;
                try { automation.waitForIdle(100, 2000); } catch (java.util.concurrent.TimeoutException ignored) {}
                root = automation.getRootInActiveWindow();
            }
            String label = arguments.getString("click");
            JSONArray nodes = new JSONArray();
            AccessibilityNodeInfo target = null;
            ArrayDeque<AccessibilityNodeInfo> pending = new ArrayDeque<>();
            pending.add(root);
            while (!pending.isEmpty()) {
                AccessibilityNodeInfo node = pending.removeFirst();
                if (node.isVisibleToUser()) {
                    String description = String.valueOf(node.getContentDescription());
                    String text = String.valueOf(node.getText());
                    Rect bounds = new Rect(); node.getBoundsInScreen(bounds);
                    nodes.put(new JSONObject().put("label", description).put("text", text).put("class", node.getClassName()).put("clickable", node.isClickable()).put("scrollable", node.isScrollable()).put("heading", android.os.Build.VERSION.SDK_INT >= 28 && node.isHeading()).put("bounds", bounds.toShortString()));
                    if (label != null && label.equals(description) && node.isClickable()) target = node;
                }
                for (int i = 0; i < node.getChildCount(); i++) { AccessibilityNodeInfo child = node.getChild(i); if (child != null) pending.add(child); }
            }
            result.putString("nodes", nodes.toString());
            if (label != null) {
                // A cold TalkBack start may publish the button action after the first root.
                target = awaitLabel(automation, label, true);
                if (!target.performAction(AccessibilityNodeInfo.ACTION_CLICK)) throw new AssertionError("Android rejected the accessibility click");
                result.putString("clicked", label);
                if ("Enable accessibility".equals(label)) {
                    waitForRoot(automation, "com.android.settings");
                    result.putBoolean("settings_opened", true);
                }
            }
            result.putString("status", "passed");
            finish(Activity.RESULT_OK, result);
        } catch (Throwable failure) {
            result.putString("status", "failed");
            result.putString("failure", failure.toString());
            finish(Activity.RESULT_CANCELED, result);
        }
    }
    private AccessibilityNodeInfo awaitLabel(UiAutomation automation, String label) {
        return awaitLabel(automation, label, false);
    }
    private AccessibilityNodeInfo awaitLabel(UiAutomation automation, String label, boolean requireClickable) {
        long deadline = SystemClock.elapsedRealtime() + 10000;
        while (SystemClock.elapsedRealtime() < deadline) {
            AccessibilityNodeInfo root = automation.getRootInActiveWindow();
            if (root != null) {
                ArrayDeque<AccessibilityNodeInfo> pending = new ArrayDeque<>();
                pending.add(root);
                int count = 0;
                while (!pending.isEmpty() && count++ < 1000) {
                    AccessibilityNodeInfo node = pending.removeFirst();
                    if (node.isVisibleToUser() && (!requireClickable || node.isClickable()) && (label.contentEquals(node.getContentDescription() == null ? "" : node.getContentDescription()) || label.contentEquals(node.getText() == null ? "" : node.getText()))) return node;
                    for (int i = 0; i < node.getChildCount(); i++) { AccessibilityNodeInfo child = node.getChild(i); if (child != null) pending.add(child); }
                }
            }
            SystemClock.sleep(50);
        }
        throw new AssertionError("No visible setting: " + label);
    }
    private void click(UiAutomation automation, String label) {
        AccessibilityNodeInfo node = awaitLabel(automation, label);
        if (node.isClickable() && node.performAction(AccessibilityNodeInfo.ACTION_CLICK)) return;
        // Lynx exposes native accessibility actions when a screen reader is enabled.
        // Keep the navigation check usable on emulator images without TalkBack as well.
        Rect bounds = new Rect(); node.getBoundsInScreen(bounds);
        long now = SystemClock.uptimeMillis();
        android.view.MotionEvent down = android.view.MotionEvent.obtain(now, now, android.view.MotionEvent.ACTION_DOWN, bounds.exactCenterX(), bounds.exactCenterY(), 0);
        android.view.MotionEvent up = android.view.MotionEvent.obtain(now, now + 50, android.view.MotionEvent.ACTION_UP, bounds.exactCenterX(), bounds.exactCenterY(), 0);
        down.setSource(android.view.InputDevice.SOURCE_TOUCHSCREEN);
        up.setSource(android.view.InputDevice.SOURCE_TOUCHSCREEN);
        try {
            if (!automation.injectInputEvent(down, true) || !automation.injectInputEvent(up, true)) throw new AssertionError("Could not tap setting: " + label);
        } finally { down.recycle(); up.recycle(); }
    }
    private AccessibilityNodeInfo scrollable(AccessibilityNodeInfo root) {
        ArrayDeque<AccessibilityNodeInfo> nodes = new ArrayDeque<>(); nodes.add(root);
        int count = 0;
        while (!nodes.isEmpty() && count++ < 1000) {
            AccessibilityNodeInfo node = nodes.removeFirst();
            if (node.isVisibleToUser() && node.isScrollable()) return node;
            for (int i = 0; i < node.getChildCount(); i++) { AccessibilityNodeInfo child = node.getChild(i); if (child != null) nodes.add(child); }
        }
        return null;
    }
    private boolean hasStopControl(AccessibilityNodeInfo root) {
        ArrayDeque<AccessibilityNodeInfo> nodes = new ArrayDeque<>(); nodes.add(root);
        int count = 0;
        while (!nodes.isEmpty() && count++ < 1000) {
            AccessibilityNodeInfo node = nodes.removeFirst();
            if ("Stop companion session".contentEquals(node.getContentDescription() == null ? "" : node.getContentDescription())) return true;
            for (int i = 0; i < node.getChildCount(); i++) { AccessibilityNodeInfo child = node.getChild(i); if (child != null) nodes.add(child); }
        }
        return false;
    }
    private AccessibilityNodeInfo waitForRoot(UiAutomation automation, String packageName) {
        long deadline = SystemClock.elapsedRealtime() + 10000;
        while (SystemClock.elapsedRealtime() < deadline) {
            AccessibilityNodeInfo root = automation.getRootInActiveWindow();
            if (root != null && packageName.contentEquals(root.getPackageName())) {
                if (!packageName.equals("io.andromeda.companion") || "pad".equals(arguments.getString("screen")) || hasStopControl(root)) return root;
            }
            SystemClock.sleep(50);
        }
        throw new AssertionError("No active window for " + packageName);
    }
}
