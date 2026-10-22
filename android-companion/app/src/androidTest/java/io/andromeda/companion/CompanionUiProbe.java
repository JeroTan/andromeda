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

/** Exercises the public Android accessibility interface without suppressing screen readers. */
public final class CompanionUiProbe extends Instrumentation {
    private Bundle arguments;
    @Override public void onCreate(Bundle arguments) { super.onCreate(arguments); this.arguments = arguments; start(); }
    @Override public void onStart() {
        Bundle result = new Bundle();
        try {
            UiAutomation automation = getUiAutomation(UiAutomation.FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES);
            boolean pad = "pad".equals(arguments.getString("screen"));
            Intent launch = new Intent().setClassName("io.andromeda.companion", pad ? "io.andromeda.companion.ProbeActivity" : "io.andromeda.companion.MainActivity").addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TOP);
            getTargetContext().startActivity(launch);
            AccessibilityNodeInfo root = waitForRoot(automation, "io.andromeda.companion");
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
                if (target == null) throw new AssertionError("No accessible clickable control: " + label);
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
