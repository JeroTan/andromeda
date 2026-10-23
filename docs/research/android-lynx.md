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

A published npm set with matching peer ranges is ReactLynx `0.128.0`, `react-rsbuild-plugin` `0.21.0`, Rspeedy `0.18.2`, Rsbuild `2.2.14`, and `@lynx-js/types` `4.1.0`. Pin the lockfile and target `pluginReactLynx({ engineVersion: '3.9' })`; engine targeting is independent of npm package versions. This establishes declared compatibility, not device validation. [Plugin metadata](https://registry.npmjs.org/@lynx-js%2Freact-rsbuild-plugin/0.21.0), [Rspeedy metadata](https://registry.npmjs.org/@lynx-js%2Frspeedy/0.18.2), [engine compatibility](https://lynxjs.org/guide/compatibility)

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

## Implementation addendum: configuration and bridge details

**Font sizing.** Lynx CSS documents `px` as Android `dp`; its supported length list does not include `sp`. The documented scaling route is `builder.setFontScale(scale)` at creation and `lynxView.updateFontScale(scale)` afterward; both font size and line height scale. The 4.1.1 builder defaults to `1.0`, so system scaling is not established merely by creating a view. [CSS lengths](https://lynxjs.org/api/css/data-type/length), [large text API](https://lynxjs.org/api/elements/built-in/text#big-font)

Android warns that `Configuration.fontScale` is no longer an exact size multiplier. For native SP behavior, convert each typography token in the host and pass the resulting Lynx CSS size through global props:

```java
float lynxPx = TypedValue.applyDimension(
    TypedValue.COMPLEX_UNIT_SP, tokenSp, metrics) / metrics.density;
```

Keep Lynx’s font scale at `1.0` for that approach and leave legacy `enable-font-scaling` off to prevent a second scaling pass. Recompute tokens after configuration changes. This is the proposed Andromeda integration; verify resulting layout at 200% scale. [Android fontScale contract](https://developer.android.com/reference/android/content/res/Configuration#fontScale), [4.1.1 sources: `LynxBaseConfigurator`, `BaseTextShadowNode`](https://repo.maven.apache.org/maven2/org/lynxsdk/lynx/lynx/4.1.1/lynx-4.1.1-sources.jar)

**Dark mode.** The builder defaults to `LynxColorScheme.LIGHT`. Read `Configuration.uiMode & UI_MODE_NIGHT_MASK`; seed `builder.setColorScheme(LynxColorScheme.DARK/LIGHT)` before rendering and use `lynxView.updateColorScheme(...)` on changes. For class/CSS-variable themes, also publish `appTheme` with `updateGlobalProps(Map<String,Object>)`; ReactLynx’s `useGlobalProps()` rerenders when it changes. Reapply current configuration after Activity recreation and when returning from Settings; use `onConfigurationChanged` only if the host explicitly handles those changes. [4.1.1 sources: `LynxColorScheme`, `LynxView`](https://repo.maven.apache.org/maven2/org/lynxsdk/lynx/lynx/4.1.1/lynx-4.1.1-sources.jar), [theming](https://lynxjs.org/guide/styling/custom-theming), [reactive global props](https://lynxjs.org/api/react/function.useglobalprops), [Android recreation](https://developer.android.com/guide/topics/resources/runtime-changes)

**Activity context.** A normal per-view module receives a `LynxContext`; a shared module can receive application context. With `builder.build(activity)`, resolve `((LynxContext) mContext).getActivity()` when needed, check for null/finishing/destroyed, and dispatch UI work to the main thread. Do not cast `mContext` directly to Activity or cache the result across recreation. `getContext()` can itself return a wrapper. [4.1.1 sources: `LynxModule`, `LynxBaseContext`, `ContextUtils`](https://repo.maven.apache.org/maven2/org/lynxsdk/lynx/lynx/4.1.1/lynx-4.1.1-sources.jar)

**Status callbacks.** Use `callback.invoke(JavaOnlyMap.from(status))` for a flat status map. `CallbackImpl` wraps arguments in `JavaOnlyArray`, whose type inspection accepts `ReadableMap` and rejects a plain `HashMap`. `JavaOnlyMap.from` copies shallowly: convert nested maps/arrays too. Invoke each callback once; use a fresh callback for each refresh. The separate `updateGlobalProps(Map)` API accepts ordinary maps and should not be confused with this callback boundary. [Java callback path](https://github.com/lynx-family/lynx/blob/release/4.1/core/runtime/js/bindings/modules/android/callback_impl.cc), [4.1.1 sources: `CallbackImpl`, `JavaOnlyArray`, `JavaOnlyMap`](https://repo.maven.apache.org/maven2/org/lynxsdk/lynx/lynx/4.1.1/lynx-4.1.1-sources.jar)

**Native button semantics.** The Android property is plural: `accessibility-traits="button"`. The source maps it to `android.widget.Button`. Give controls explicit labels and tap activation; keep decorative children out of traversal:

```tsx
<view flatten={false} accessibility-element={true}
  accessibility-label="Stop session" accessibility-traits="button"
  accessibility-enable-tap={true} bindtap={stopSession}>
  <text accessibility-element={false}>Stop session</text>
</view>
```

The 4.1.1 accessibility delegate routes accessibility clicks to bound tap/click handlers. Validate TalkBack activation and role on the built APK; attributes alone do not prove the rendered accessibility tree. [4.1.1 sources: `PropsConstants`, `LynxAccessibilityHelper`, `CustomAccessibilityDelegateCompat`](https://repo.maven.apache.org/maven2/org/lynxsdk/lynx/lynx/4.1.1/lynx-4.1.1-sources.jar)

Local build follow-up: the compiler bundled with this pinned npm set rejects engine target 4.1 and reports support through 3.9. Targeting 3.9 builds a single 113.1 kB `main.lynx.bundle`; the Android runtime remains 4.1.1. This is a backward-targeted bundle, with device integration still pending. Disable `tools.rspack.optimization.splitChunks` for this single-bundle offline UI; Rsbuild 2 warns that the older `performance.chunkSplit` option is ignored.

The first isolated-emulator launch exposed `NoClassDefFoundError: androidx/core/util/Consumer`. The Lynx artifact does not bring this host runtime class transitively; the companion now declares `androidx.core:core:1.19.1` explicitly (verified in Google Maven metadata), which also supplies the missing AndroidX annotation classes during compilation. APK compilation passes; runtime validation continues.

The emulator initially exposed only explicitly labeled view wrappers while TalkBack was off. The 4.1.1 `LynxEnv` source provides `enablePlatformObservability(true)` to initialize its delegate without requiring touch exploration. The host now sets this before creating views; it exposes the app's native semantics without granting a device accessibility service. Its source requires `accessibility-enable-tap=true` for advertised click actions. Verify the resulting tree and real screen-reader behavior separately.

Accessibility follow-up: the virtual delegate exposed text and click actions, but its 4.1.1 node provider reports implementation class names rather than the declared button traits. The companion now retains the view-based `enableA11y: true` mode, disables default container aggregation (`enableAccessibilityElement: false`) through config plugin 0.2.5, and uses non-flattened text. With TalkBack active and UiAutomation configured not to suppress it, the emulator reports Stop and Enable accessibility as `android.widget.Button`, the title as a heading, and an accessibility ACTION_CLICK opens Android Settings. The test uses public Android UI automation, not internal service state.
