package io.andromeda.companion;

import android.app.Activity;
import android.content.Context;
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
    private final Handler main = new Handler(Looper.getMainLooper());
    public CompanionModule(Context context) { super(context); }

    @LynxMethod public void status(Callback done) { done.invoke(JavaOnlyMap.from(snapshot(null))); }

    @LynxMethod public void action(String action, Callback done) {
        main.post(() -> {
            String error = null;
            try {
                ControlService service = ControlService.instance;
                if ("stop".equals(action)) {
                    if (service != null) service.stopSession();
                } else if ("accessibility".equals(action)) {
                    Activity activity = activity();
                    activity.startActivity(new Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS));
                } else if ("diagnostics".equals(action) && BuildConfig.DEBUG) {
                    Activity activity = activity();
                    activity.startActivity(new Intent().setClassName(activity, "io.andromeda.companion.ProbeActivity"));
                } else {
                    if (service == null) throw new IllegalStateException("Enable Andromeda in Accessibility settings first.");
                    switch (action) {
                        case "pair" -> service.openPairing();
                        case "start" -> { if (!service.startSession()) throw new IllegalStateException("Pair your computer first."); }
                        case "forget" -> service.forgetComputer();
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

    private Map<String,Object> snapshot(String error) {
        Map<String,Object> status = new HashMap<>();
        ControlService service = ControlService.instance;
        status.put("enabled", service != null);
        status.put("paired", service != null && service.hasPairing());
        status.put("session_active", service != null && service.sessionSeconds() > 0);
        status.put("pairing_seconds", service == null ? 0 : service.pairingSeconds());
        status.put("session_seconds", service == null ? 0 : service.sessionSeconds());
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
