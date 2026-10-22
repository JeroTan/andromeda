package io.andromeda.companion;

import android.content.Context;
import com.lynx.tasm.provider.AbsTemplateProvider;
import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** The presentation bundle is shipped in the APK; there is no URL loader. */
public final class AssetBundleProvider extends AbsTemplateProvider implements AutoCloseable {
    private final Context context;
    private final ExecutorService loader = Executors.newSingleThreadExecutor();
    public AssetBundleProvider(Context context) { this.context = context.getApplicationContext(); }
    @Override public void loadTemplate(String name, Callback callback) {
        if (!"main.lynx.bundle".equals(name)) { callback.onFailed("Unknown packaged interface"); return; }
        loader.execute(() -> {
            try (InputStream input = context.getAssets().open(name); ByteArrayOutputStream output = new ByteArrayOutputStream()) {
                byte[] buffer = new byte[8192];
                int count;
                while ((count = input.read(buffer)) != -1) {
                    if (output.size() + count > 8 * 1024 * 1024) throw new java.io.IOException("Interface bundle exceeds limit");
                    output.write(buffer, 0, count);
                }
                callback.onSuccess(output.toByteArray());
            } catch (Exception failure) { callback.onFailed("Unable to load Andromeda's packaged interface"); }
        });
    }
    @Override public void close() { loader.shutdownNow(); }
}
