package io.andromeda.companion;

import android.app.Activity;
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
    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
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
    }
    @Override protected void onResume() { super.onResume(); if (lynxView != null) lynxView.onEnterForeground(); }
    @Override protected void onPause() { if (lynxView != null) lynxView.onEnterBackground(); super.onPause(); }
    @Override protected void onDestroy() {
        if (lynxView != null) { lynxView.destroy(); lynxView = null; }
        if (provider != null) provider.close();
        super.onDestroy();
    }
}
