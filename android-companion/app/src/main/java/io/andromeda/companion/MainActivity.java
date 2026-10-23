package io.andromeda.companion;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;
import android.content.res.Configuration;
import android.graphics.Color;
import android.view.View;
import android.widget.FrameLayout;
import com.lynx.tasm.LynxColorScheme;
import com.lynx.tasm.LynxView;
import com.lynx.tasm.LynxViewBuilder;

public final class MainActivity extends Activity {
    private LynxView lynxView;
    private AssetBundleProvider provider;
    private String screen = "home";
    private android.window.OnBackInvokedCallback backCallback;
    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        if (BuildConfig.DEBUG) CompanionModule.testServiceDisconnected = getIntent().getBooleanExtra("io.andromeda.companion.TEST_SERVICE_DISCONNECTED", false);
        if (state != null) screen = state.getString("screen", "home");
        boolean dark = (getResources().getConfiguration().uiMode & Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES;
        FrameLayout frame = new FrameLayout(this);
        frame.setBackgroundColor(Color.parseColor(dark ? "#17171c" : "#e9e7ee"));
        frame.setOnApplyWindowInsetsListener((view, insets) -> {
            view.setPadding(insets.getSystemWindowInsetLeft(), insets.getSystemWindowInsetTop(), insets.getSystemWindowInsetRight(), insets.getSystemWindowInsetBottom());
            return insets;
        });
        getWindow().getDecorView().setSystemUiVisibility(dark ? 0 : View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR | View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR);
        provider = new AssetBundleProvider(this);
        LynxViewBuilder builder = new LynxViewBuilder();
        builder.setTemplateProvider(provider);
        builder.setFontScale(1.0f); // Typography tokens are converted from native SP in the module.
        builder.setColorScheme(dark ? LynxColorScheme.DARK : LynxColorScheme.LIGHT);
        builder.registerModule("Companion", CompanionModule.class);
        lynxView = builder.build(this);
        frame.addView(lynxView, new FrameLayout.LayoutParams(-1, -1));
        setContentView(frame);
        frame.requestApplyInsets();
        lynxView.renderTemplateUrl("main.lynx.bundle", "{}");
        navigate(screen);
    }
    public String screen() { return screen; }
    public void navigate(String destination) {
        screen = destination;
        if (lynxView != null) lynxView.updateGlobalProps(java.util.Map.of("screen", screen));
        if (android.os.Build.VERSION.SDK_INT >= 33) {
            if (!"home".equals(screen) && backCallback == null) {
                backCallback = () -> navigate("session".equals(screen) ? "settings" : "home");
                getOnBackInvokedDispatcher().registerOnBackInvokedCallback(android.window.OnBackInvokedDispatcher.PRIORITY_DEFAULT, backCallback);
            } else if ("home".equals(screen) && backCallback != null) {
                getOnBackInvokedDispatcher().unregisterOnBackInvokedCallback(backCallback);
                backCallback = null;
            }
        }
    }
    @Override public void onBackPressed() {
        if (!"home".equals(screen)) navigate("session".equals(screen) ? "settings" : "home");
        else super.onBackPressed();
    }
    @Override protected void onSaveInstanceState(Bundle state) {
        state.putString("screen", screen);
        super.onSaveInstanceState(state);
    }
    @Override protected void onResume() {
        super.onResume();
        if (lynxView != null) {
            lynxView.onEnterForeground();
            lynxView.updateGlobalProps(java.util.Map.of("screen", screen, "refresh_id", android.os.SystemClock.elapsedRealtime()));
        }
    }
    @Override protected void onPause() { if (lynxView != null) lynxView.onEnterBackground(); super.onPause(); }
    @Override protected void onDestroy() {
        if (android.os.Build.VERSION.SDK_INT >= 33 && backCallback != null) {
            getOnBackInvokedDispatcher().unregisterOnBackInvokedCallback(backCallback);
            backCallback = null;
        }
        if (lynxView != null) { lynxView.destroy(); lynxView = null; }
        if (provider != null) provider.close();
        super.onDestroy();
    }
}
