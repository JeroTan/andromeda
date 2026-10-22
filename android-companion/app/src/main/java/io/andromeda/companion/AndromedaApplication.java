package io.andromeda.companion;

import android.app.Application;
import com.lynx.tasm.LynxEnv;

public final class AndromedaApplication extends Application {
    @Override public void onCreate() {
        super.onCreate();
        LynxEnv.inst().init(this, null, null, null);
    }
}
