import groovy.json.JsonSlurper
import java.nio.file.Files
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

val buildsRelease = gradle.startParameter.taskNames.any { requested ->
    val task = requested.substringAfterLast(':')
    // `build` and `assemble` package the release variant too.
    task.contains("Release", ignoreCase = true) || task == "build" || task == "assemble"
}
if (!hasReleaseSigning && buildsRelease) {
    logger.warn(
        "TabTool: release signing is not configured. Set TABTOOL_ANDROID_KEYSTORE, " +
            "TABTOOL_ANDROID_KEYSTORE_PASSWORD, TABTOOL_ANDROID_KEY_ALIAS and " +
            "TABTOOL_ANDROID_KEY_PASSWORD to sign a real release; this build is debug-signed.",
    )
}

// cargo ndk writes src/main/jniLibs and Gradle only packages what it finds
// there. It reports "Copying libraries" even when it leaves an earlier
// variant's copy in place, and one directory cannot serve both variants, so a
// silent mismatch would ship release libraries in a debug APK or the reverse.
// The check compares the bytes and fails instead of packaging the wrong ones.
val universalDirectory = rootProject.layout.projectDirectory.dir("../../..")
val rustSources = files(
    universalDirectory.dir("crates")
        .asFileTree.matching { include("**/src/**/*.rs", "**/Cargo.toml") },
    universalDirectory.dir("apps/tabtool-android/src").asFileTree.matching { include("**/*.rs") },
    universalDirectory.file("apps/tabtool-android/Cargo.toml"),
    universalDirectory.file("Cargo.toml"),
    universalDirectory.file("Cargo.lock"),
)
val abiTriples = mapOf("arm64-v8a" to "aarch64-linux-android", "x86_64" to "x86_64-linux-android")

/** The variants the requested tasks package; an aggregate asks for both. */
fun requestedProfiles(): Pair<Boolean, Boolean> {
    val tasks = gradle.startParameter.taskNames.map { it.substringAfterLast(':') }
    val aggregate = tasks.any { it == "assemble" || it == "build" }
    val debug = aggregate || tasks.any { it.contains("debug", ignoreCase = true) }
    val release = aggregate || tasks.any { it.contains("release", ignoreCase = true) }
    return debug to release
}

val verifyNativeLibraries by tasks.registering {
    description = "Checks that src/main/jniLibs holds the libraries cargo built for the requested variant."
    val libraries = abiTriples.keys.associateWith {
        layout.projectDirectory.file("src/main/jniLibs/$it/libmain.so").asFile
    }
    val sources = rustSources.files.filter { it.isFile }
    val (wantsDebug, wantsRelease) = requestedProfiles()
    doLast {
        check(!(wantsDebug && wantsRelease)) {
            "One src/main/jniLibs directory cannot serve both variants. Run " +
                "./gradlew assembleDebug or ./gradlew assembleRelease after cargo ndk."
        }
        val profile = when {
            wantsRelease -> "release"
            wantsDebug -> "debug"
            // A task that packages nothing (for example `lint`) does not need
            // the byte check.
            else -> null
        }
        val cargoCommand = if (profile == "release") "cargo ndk --release" else "cargo ndk"
        val newestSource = sources.maxOfOrNull { it.lastModified() } ?: 0L
        libraries.forEach { (abi, packaged) ->
            check(packaged.isFile) {
                "Missing ${packaged.path}. Run $cargoCommand before Gradle (docs/android.md)."
            }
            check(packaged.lastModified() >= newestSource) {
                "${packaged.path} is older than the newest Rust source; run $cargoCommand again."
            }
            if (profile == null) {
                return@forEach
            }
            val built = universalDirectory.file("target/${abiTriples.getValue(abi)}/$profile/libmain.so")
            check(built.asFile.isFile) {
                "No ${built.asFile.path}. Run $cargoCommand before Gradle (docs/android.md)."
            }
            check(Files.mismatch(packaged.toPath(), built.asFile.toPath()) == -1L) {
                "${packaged.path} is not the $profile library cargo built; run $cargoCommand again."
            }
        }
    }
}

tasks.named("preBuild") {
    dependsOn(verifyNativeLibraries)
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

    // The native libraries ship uncompressed and page-aligned and load from the
    // APK in place, so GameActivity never extracts them at install time. This is
    // the Android Gradle Plugin default; stating it keeps a future change
    // deliberate, and `scripts/android-verify` checks the result.
    packaging {
        jniLibs {
            useLegacyPackaging = false
        }
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
