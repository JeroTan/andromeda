package io.andromeda.companion;

import android.app.Activity;
import android.content.Context;
import android.content.ComponentName;
import android.content.Intent;
import android.content.res.Configuration;
import android.os.Handler;
import android.os.Looper;
import android.provider.Settings;
import android.util.DisplayMetrics;
import android.util.TypedValue;
import com.lynx.jsbridge.LynxMethod;
import com.lynx.jsbridge.LynxModule;
import com.lynx.react.bridge.Callback;
import com.lynx.react.bridge.JavaOnlyMap;
import com.lynx.tasm.behavior.LynxContext;
import java.util.HashMap;
import java.util.Map;

/** Presentation-only controls. Tokens and device input never cross this bridge. */
public final class CompanionModule extends LynxModule {
    /** Debug-only instrumentation hook; release builds ignore it. */
    static volatile boolean testServiceDisconnected;
    private final Handler main = new Handler(Looper.getMainLooper());
    public CompanionModule(Context context) { super(context); }

    @LynxMethod public void status(Callback done) { done.invoke(JavaOnlyMap.from(snapshot(null))); }

    @LynxMethod public void action(String action, Callback done) {
        main.post(() -> {
            String error = null;
            try {
                ControlService service = ControlService.instance;
                if ("settings".equals(action) || "session-settings".equals(action) || "background-settings".equals(action) || "back".equals(action)) {
                    MainActivity host = (MainActivity) activity();
                    host.navigate("back".equals(action) ? ("session".equals(host.screen()) || "background".equals(host.screen()) ? "settings" : "home") : "settings".equals(action) ? "settings" : "background-settings".equals(action) ? "background" : "session");
                } else if ("battery-exemption".equals(action)) {
                    BackgroundAccess.requestBatteryExemption(activity());
                } else if ("app-battery-settings".equals(action)) {
                    BackgroundAccess.openAppSettings(activity());
                } else if ("timer-off".equals(action) || "timer-15".equals(action) || "timer-30".equals(action) || "timer-60".equals(action)) {
                    int minutes = "timer-off".equals(action) ? 0 : Integer.parseInt(action.substring(6));
                    if (!mContext.getSharedPreferences("pairing", Context.MODE_PRIVATE).edit().putInt("session_timer_minutes", minutes).commit())
                        throw new IllegalStateException("Could not save the session timer. Try again.");
                } else if ("forget".equals(action)) {
                    if (service != null) service.forgetComputer();
                    else if (!mContext.getSharedPreferences("pairing", Context.MODE_PRIVATE).edit().remove("token").commit())
                        throw new IllegalStateException("Could not remove the pairing. Try again.");
                } else if ("stop".equals(action)) {
                    if (service != null) service.stopSession();
                } else if (action.startsWith("project-pause:") || action.startsWith("project-resume:") || action.startsWith("project-stop:")) {
                    String[] parts=action.split(":",2);
                    if(service!=null && parts.length==2 && parts[1].matches("[0-9a-f]{32}")) service.changeProject(parts[1],parts[0].equals("project-pause")?"paused":parts[0].equals("project-resume")?"running":"stopped");
                } else if ("refresh".equals(action)) {
                    // Return a fresh snapshot; Android owns accessibility service binding.
                } else if ("accessibility".equals(action) || "accessibility-settings".equals(action)) {
                    if ("accessibility-settings".equals(action) || !accessibilityEnabled())
                        activity().startActivity(new Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS));
                } else if ("diagnostics".equals(action) && BuildConfig.DEBUG) {
                    Activity activity = activity();
                    activity.startActivity(new Intent().setClassName(activity, "io.andromeda.companion.ProbeActivity"));
                } else {
                    if (service == null || !service.ready()) throw new IllegalStateException(accessibilityEnabled() ? "Accessibility permission is enabled. Android is still connecting the service; check again shortly." : "Enable Andromeda in Accessibility settings first.");
                    switch (action) {
                        case "pair" -> service.openPairing();
                        case "start" -> { if (!service.startSession()) throw new IllegalStateException("Pair your computer first."); }
                        case "disable" -> { service.stopSession(); service.disableSelf(); }
                        default -> throw new IllegalStateException("This control is unavailable.");
                    }
                }
            } catch (IllegalStateException failure) { error = failure.getMessage(); }
            catch (Exception failure) { error = "Android could not complete this action. Try again from the app."; }
            done.invoke(JavaOnlyMap.from(snapshot(error)));
        });
    }

    private Activity activity() {
        Activity activity = mContext instanceof LynxContext context ? context.getActivity() : null;
        if (activity == null || activity.isFinishing() || activity.isDestroyed()) throw new IllegalStateException("Return to the Andromeda app and try again.");
        return activity;
    }

    private boolean accessibilityEnabled() {
        ComponentName own = new ComponentName(mContext, ControlService.class);
        String enabled = Settings.Secure.getString(mContext.getContentResolver(), Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES);
        if (enabled == null) return false;
        for (String entry : enabled.split(":")) {
            if (own.equals(ComponentName.unflattenFromString(entry))) return true;
        }
        return false;
    }

    private Map<String,Object> snapshot(String error) {
        Map<String,Object> status = new HashMap<>();
        ControlService service = ControlService.instance;
        status.put("battery_exempt", BackgroundAccess.batteryExempt(mContext));
        status.put("background_restricted", BackgroundAccess.restricted(mContext));
        status.put("manufacturer", android.os.Build.MANUFACTURER);
        status.put("enabled", accessibilityEnabled());
        status.put("service_ready", service != null && service.ready() && !(BuildConfig.DEBUG && testServiceDisconnected));
        status.put("paired", !mContext.getSharedPreferences("pairing", Context.MODE_PRIVATE).getString("token", "").isEmpty());
        status.put("desktop_connected", service != null && service.desktopConnected());
        status.put("projects_json", service == null ? "[]" : service.projectsSnapshot().toString());
        status.put("session_active", service != null && service.sessionActive());
        status.put("session_expired", service != null && service.sessionExpired());
        status.put("last_contact_seconds", service == null ? -1 : service.lastContactSeconds());
        status.put("pairing_seconds", service == null ? 0 : service.pairingSeconds());
        status.put("session_seconds", service == null ? 0 : service.sessionSeconds());
        status.put("timer_minutes", mContext.getSharedPreferences("pairing", Context.MODE_PRIVATE).getInt("session_timer_minutes", 0));
        status.put("dark", (mContext.getResources().getConfiguration().uiMode & Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES);
        status.put("developer", BuildConfig.DEBUG);
        DisplayMetrics metrics = mContext.getResources().getDisplayMetrics();
        status.put("title_size", sp(48, metrics));
        status.put("body_size", sp(16, metrics));
        status.put("small_size", sp(12, metrics));
        status.put("button_size", sp(15, metrics));
        if (error != null) status.put("error", error);
        return status;
    }
    private float sp(float value, DisplayMetrics metrics) { return TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_SP, value, metrics) / metrics.density; }
}
