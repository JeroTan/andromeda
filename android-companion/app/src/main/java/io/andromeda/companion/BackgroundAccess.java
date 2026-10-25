package io.andromeda.companion;

import android.app.Activity;
import android.app.ActivityManager;
import android.content.ActivityNotFoundException;
import android.content.Context;
import android.content.Intent;
import android.net.Uri;
import android.os.Build;
import android.os.PowerManager;
import android.provider.Settings;

/** Read system policy without changing it; requests always remain owner-approved. */
final class BackgroundAccess {
    private BackgroundAccess() {}

    static boolean batteryExempt(Context context) {
        PowerManager manager = context.getSystemService(PowerManager.class);
        return manager != null && manager.isIgnoringBatteryOptimizations(context.getPackageName());
    }

    static boolean restricted(Context context) {
        ActivityManager manager = context.getSystemService(ActivityManager.class);
        return Build.VERSION.SDK_INT >= 28 && manager != null && manager.isBackgroundRestricted();
    }

    static void requestBatteryExemption(Activity activity) {
        if (batteryExempt(activity)) return;
        Intent request = new Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS,
                Uri.parse("package:" + activity.getPackageName()));
        try {
            activity.startActivity(request);
        } catch (ActivityNotFoundException | SecurityException unavailable) {
            try {
                activity.startActivity(new Intent(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS));
            } catch (ActivityNotFoundException | SecurityException noList) {
                openAppSettings(activity);
            }
        }
    }

    static void openAppSettings(Activity activity) {
        activity.startActivity(new Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                Uri.parse("package:" + activity.getPackageName())));
    }
}
