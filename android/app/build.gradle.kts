import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

kotlin {
    compilerOptions { jvmTarget.set(JvmTarget.JVM_17) }
}

android {
    namespace = "com.jude.tetrisnesrust"
    compileSdk = 35
    // Java and Kotlin must target the same JVM bytecode level.
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    ndkVersion = "27.2.12479018"

    defaultConfig {
        applicationId = "com.jude.tetrisnesrust"
        minSdk = 23
        targetSdk = 35
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        // CI can build the same app for the x86_64 emulator. Published APKs
        // remain ARM64 unless explicitly overridden for testing.
        ndk { abiFilters += listOf(providers.gradleProperty("testAbi").orElse("arm64-v8a").get()) }
        versionCode = 5
        versionName = "0.3.0"
        externalNativeBuild {
            cmake {
                cppFlags += listOf("-std=c++17", "-fexceptions")
            }
        }
    }
    signingConfigs {
        create("releaseKey") {
            storeFile = file(System.getenv("RELEASE_KEYSTORE_PATH") ?: "release.keystore")
            storePassword = System.getenv("RELEASE_STORE_PASSWORD") ?: ""
            keyAlias = System.getenv("RELEASE_KEY_ALIAS") ?: ""
            keyPassword = System.getenv("RELEASE_KEY_PASSWORD") ?: ""
        }
    }
    buildTypes {
        getByName("release") {
            // If no signing secrets are configured, build an INSTALLABLE test APK
            // with the Android debug key, without making the app debuggable.
            signingConfig = if (System.getenv("RELEASE_KEYSTORE_PATH").isNullOrBlank())
                signingConfigs.getByName("debug") else signingConfigs.getByName("releaseKey")
            isMinifyEnabled = false
        }
    }
    testBuildType = "release"
    externalNativeBuild {
        cmake { path = file("src/main/cpp/CMakeLists.txt") }
    }
    packaging {
        jniLibs { useLegacyPackaging = false }
    }
}

dependencies {
    androidTestImplementation("androidx.test:runner:1.6.2")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
}
