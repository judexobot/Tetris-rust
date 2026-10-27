plugins {
    id("com.android.application") version "8.7.3" apply false
    // AGP 8.7's D8 supports Kotlin 2.0 metadata; 2.2 requires a newer AGP.
    id("org.jetbrains.kotlin.android") version "2.0.21" apply false
}
