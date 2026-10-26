plugins { id("com.android.application") }

android {
    namespace = "com.jude.tetrisnesrust"
    compileSdk = 35
    ndkVersion = "27.2.12479018"

    defaultConfig {
        applicationId = "com.jude.tetrisnesrust"
        minSdk = 23
        targetSdk = 35
        ndk { abiFilters += listOf("arm64-v8a") }
        versionCode = 3
        versionName = "0.2.1"
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
    externalNativeBuild {
        cmake { path = file("src/main/cpp/CMakeLists.txt") }
    }
    packaging {
        jniLibs { useLegacyPackaging = false }
    }
}
