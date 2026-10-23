plugins { id("com.android.application") }
android {
    namespace = "io.andromeda.companion"
    compileSdk = 37
    defaultConfig {
        applicationId = "io.andromeda.companion"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
        testInstrumentationRunner = "io.andromeda.companion.CompanionUiProbe"
    }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
    buildFeatures { buildConfig = true }
    sourceSets["main"].assets.srcDir("../ui/dist")
}
dependencies {
    implementation("org.lynxsdk.lynx:lynx:4.1.1")
    implementation("androidx.core:core:1.19.1")
}
tasks.named("preBuild") { doFirst { check(file("../ui/dist/main.lynx.bundle").isFile) { "Build the offline interface with npm --prefix ui run build first." } } }
