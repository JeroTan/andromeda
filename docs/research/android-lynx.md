# ReactLynx Android integration

Checked 2026-10-22 against publisher documentation, Maven artifacts, and the **4.1.1 source JAR**. No dependencies were installed. The APIs below were inspected, not compiled in Andromeda; full build and integration checks belong in CI.

## Versions and dependencies

Use **Lynx 4.1.1**, the highest plain stable version in Maven metadata. The metadata’s `latest`/`release` fields currently select `4.1.1-dev`; do not use dynamic versions. The stable POM pins its matching PrimJS, base, graphics, trace, JS SDK, and service API dependencies to 4.1.1, including `primjsWasm`. PrimJS 4.2.1 exists separately; it is not the dependency selected by this Lynx release. [Metadata](https://repo.maven.apache.org/maven2/org/lynxsdk/lynx/lynx/maven-metadata.xml), [4.1.1 POM](https://repo.maven.apache.org/maven2/org/lynxsdk/lynx/lynx/4.1.1/lynx-4.1.1.pom)

The existing project already uses `mavenCentral()`. Explicit coordinates matching the integration guide are:

```kotlin
dependencies {
    implementation("org.lynxsdk.lynx:lynx:4.1.1")
    implementation("org.lynxsdk.lynx:lynx-jssdk:4.1.1")
    implementation("org.lynxsdk.lynx:lynx-trace:4.1.1")
    implementation("org.lynxsdk.lynx:primjs:4.1.1")
}
```

The last three are also dependencies of `lynx`. Start with built-in views/text and CSS decoration. Images require an image service; `<svg>` requires `xelement`, `xelement-svg` at 4.1.1 and `servalsvg:0.0.1-alpha.3`, plus `XElementBehaviors` registration. Do not add HTTP or DevTool services to the offline production companion merely because the general sample includes them. [Integration guide](https://lynxjs.org/guide/start/integrate-with-existing-apps)

A published npm set with matching peer ranges is ReactLynx `0.128.0`, `react-rsbuild-plugin` `0.21.0`, Rspeedy `0.18.2`, Rsbuild `2.2.14`, and `@lynx-js/types` `4.1.0`. Pin the lockfile and set `pluginReactLynx({ engineVersion: '4.1' })`; engine targeting is independent of npm package versions. This establishes declared compatibility, not device validation. [Plugin metadata](https://registry.npmjs.org/@lynx-js%2Freact-rsbuild-plugin/0.21.0), [Rspeedy metadata](https://registry.npmjs.org/@lynx-js%2Frspeedy/0.18.2), [engine compatibility](https://lynxjs.org/guide/compatibility)

## Offline loading and lifecycle

Build the bundle with the APK and put it under `app/src/main/assets/main.lynx.bundle`. Package every emitted chunk/font/image needed by the UI, or configure a single bundle with inline decoration; copying only the main bundle can omit runtime resources. [Build output](https://lynxjs.org/rspeedy/output)

Initialize in `Application.onCreate`, then create the view in `MainActivity`:

```java
// Application.onCreate:
LynxEnv.inst().init(this, null, null, null);

// MainActivity.onCreate, after super:
LynxViewBuilder builder = new LynxViewBuilder();
builder.setTemplateProvider(new AssetBundleProvider(getApplicationContext()));
builder.registerModule("Companion", CompanionModule.class);
lynxView = builder.build(this);
setContentView(lynxView);
lynxView.renderTemplateUrl("main.lynx.bundle", "{}");
```

`AssetBundleProvider` extends `com.lynx.tasm.provider.AbsTemplateProvider`. Its required method is `loadTemplate(String uri, Callback callback)`. Accept only the packaged bundle name, read through `context.getAssets().open(uri)` on a bounded executor, close the stream, and invoke `callback.onSuccess(byte[])` or `callback.onFailed(String)`. No URL fetcher is needed. These signatures and per-view `registerModule` are present in the published sources. [4.1.1 source JAR](https://repo.maven.apache.org/maven2/org/lynxsdk/lynx/lynx/4.1.1/lynx-4.1.1-sources.jar)

Call `lynxView.onEnterForeground()`/`onEnterBackground()` from Activity resume/pause and `lynxView.destroy()` during destruction, on the UI thread. Stop refresh callbacks and close the loader executor. Activity destruction must not replace the service’s independent session lifetime. The source explicitly requires `destroy()` to release view memory. [4.1.1 source JAR](https://repo.maven.apache.org/maven2/org/lynxsdk/lynx/lynx/4.1.1/lynx-4.1.1-sources.jar)

## Java bridge preserving ControlService

Use `com.lynx.jsbridge.LynxModule`, `LynxMethod`, and `com.lynx.react.bridge.Callback`. A minimal operation, adapted to Andromeda:

```java
public final class CompanionModule extends LynxModule {
    private final Handler main = new Handler(Looper.getMainLooper());
    public CompanionModule(Context context) { super(context); }

    @LynxMethod
    public void stop(Callback done) {
        main.post(() -> {
            ControlService service = ControlService.instance;
            if (service != null) service.stopSession();
            done.invoke("stopped");
        });
    }
}
```

Declare the matching TypeScript interface, then call `NativeModules.Companion.stop(callback)` from background-thread ReactLynx code. Native module calls are not available in main-thread scripts. Global registration also exists as `LynxEnv.inst().registerModule(name, moduleClass)`; per-view registration keeps this bridge attached to the companion screen. [Native modules](https://lynxjs.org/guide/use-native-modules), [4.1.1 source JAR](https://repo.maven.apache.org/maven2/org/lynxsdk/lynx/lynx/4.1.1/lynx-4.1.1-sources.jar)

Add explicit methods for status, opening accessibility settings, `openPairing()`, `startSession()`, and disable/revoke. Resolve the current service per call; preserve absent-service errors. Never expose the pairing token, arbitrary intents, shell commands, or direct gesture injection to presentation code. These are proposed Andromeda boundaries, based on the existing `MainActivity` and `ControlService`.

## Accessibility, licenses, and checks

Lynx documents `accessibility-element`, labels, traits, reading order, and announcements; Android implementation maps labels into native content descriptions. Test actual TalkBack traversal/activation, 200% font scale, rotation, and reachability of Stop. Prevent decorative orbital artwork from adding focus stops. Keep Android’s user-granted AccessibilityService permission flow; a UI framework does not grant it. [Lynx accessibility](https://lynxjs.org/guide/inclusion/accessibility), [Android service lifecycle](https://developer.android.com/reference/android/accessibilityservice/AccessibilityService)

Lynx and PrimJS POMs declare Apache-2.0; the Lynx Stack repository also uses Apache-2.0. Preserve bundled third-party license/NOTICE files and inspect the resolved dependency graph rather than assigning one license to the entire APK. [Lynx POM](https://repo.maven.apache.org/maven2/org/lynxsdk/lynx/lynx/4.1.1/lynx-4.1.1.pom), [PrimJS POM](https://repo.maven.apache.org/maven2/org/lynxsdk/lynx/primjs/4.1.1/primjs-4.1.1.pom), [Lynx Stack license](https://github.com/lynx-family/lynx-stack/blob/main/LICENSE)

CI should build the production bundle/APK, inspect merged permissions and packaged ABIs/resources, and exercise offline launch, denied permission, pairing expiry, session expiry, stop, and rotation on an emulator. A focused physical check must confirm TalkBack and the existing multi-touch/Unicode service behavior. No performance or permission-regression claim is established by this research alone.
