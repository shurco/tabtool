import groovy.json.JsonSlurper
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
}

// The version lives in app/universal/version.json, next to the workspace, and
// `cargo run -p xtask -- version bump` is the only writer. Reading the same
// file here keeps the Android build and the bump in step.
val versionFile = rootDir.resolve("../../../version.json")
require(versionFile.isFile) {
    "Missing ${versionFile.path}. Create it with \"name\" (x.y.z) and \"build\" (a positive integer)."
}
val versionEntry = JsonSlurper().parse(versionFile) as? Map<*, *>
    ?: error("${versionFile.path} must hold a JSON object with \"name\" and \"build\"")
val appVersionName = versionEntry["name"] as? String
    ?: error("${versionFile.path} has no string \"name\"")
val appVersionCode = (versionEntry["build"] as? String)?.toIntOrNull()
    ?: error("${versionFile.path} has no quoted positive integer \"build\"")
require(Regex("""\d+\.\d+\.\d+""").matches(appVersionName)) {
    "${versionFile.path}: version name must be x.y.z, found \"$appVersionName\""
}
require(appVersionCode > 0) {
    "${versionFile.path}: build must be positive, found $appVersionCode"
}

// Release signing comes from the environment; no key material is committed.
// Without the four variables the release build falls back to the debug
// keystore so the artifact stays installable, and the build says so.
val releaseKeystore = providers.environmentVariable("TABTOOL_ANDROID_KEYSTORE").orNull
val releaseKeystorePassword =
    providers.environmentVariable("TABTOOL_ANDROID_KEYSTORE_PASSWORD").orNull
val releaseKeyAlias = providers.environmentVariable("TABTOOL_ANDROID_KEY_ALIAS").orNull
val releaseKeyPassword = providers.environmentVariable("TABTOOL_ANDROID_KEY_PASSWORD").orNull
val hasReleaseSigning = listOf(
    releaseKeystore,
    releaseKeystorePassword,
    releaseKeyAlias,
    releaseKeyPassword,
).all { !it.isNullOrBlank() }

val buildsRelease = gradle.startParameter.taskNames.any { it.contains("Release", ignoreCase = true) }
if (!hasReleaseSigning && buildsRelease) {
    logger.warn(
        "TabTool: release signing is not configured. Set TABTOOL_ANDROID_KEYSTORE, " +
            "TABTOOL_ANDROID_KEYSTORE_PASSWORD, TABTOOL_ANDROID_KEY_ALIAS and " +
            "TABTOOL_ANDROID_KEY_PASSWORD to sign a real release; this build is debug-signed.",
    )
}

android {
    namespace = "app.shurco.tabtool"
    compileSdk = 36
    ndkVersion = "28.2.13676358"

    defaultConfig {
        applicationId = "app.shurco.tabtool"
        minSdk = 26
        targetSdk = 36
        versionCode = appVersionCode
        versionName = appVersionName

        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    signingConfigs {
        if (hasReleaseSigning) {
            create("release") {
                storeFile = file(releaseKeystore!!)
                storePassword = releaseKeystorePassword
                keyAlias = releaseKeyAlias
                keyPassword = releaseKeyPassword
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            signingConfig = if (hasReleaseSigning) {
                signingConfigs.getByName("release")
            } else {
                signingConfigs.getByName("debug")
            }
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        buildConfig = false
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_17)
    }
}

dependencies {
    implementation(libs.androidx.games.activity)
    implementation(libs.androidx.appcompat)
    implementation(libs.androidx.core.ktx)
}
