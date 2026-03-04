# clawclawclaw Android ProGuard Rules
# Goal: Smallest possible APK

# ============================================
# KEEP NATIVE BRIDGE
# ============================================
-keep class ai.clawclawclaw.android.bridge.** { *; }
-keepclassmembers class ai.clawclawclaw.android.bridge.** { *; }

# Keep JNI methods
-keepclasseswithmembernames class * {
    native <methods>;
}

# ============================================
# KEEP DATA CLASSES
# ============================================
-keep class ai.clawclawclaw.android.data.** { *; }
-keepclassmembers class ai.clawclawclaw.android.data.** { *; }

# ============================================
# KOTLIN SERIALIZATION
# ============================================
-keepattributes *Annotation*, InnerClasses
-dontnote kotlinx.serialization.AnnotationsKt
-keepclassmembers class kotlinx.serialization.json.** {
    *** Companion;
}
-keepclasseswithmembers class kotlinx.serialization.json.** {
    kotlinx.serialization.KSerializer serializer(...);
}

# ============================================
# AGGRESSIVE OPTIMIZATIONS
# ============================================

# Remove logging in release
-assumenosideeffects class android.util.Log {
    public static int v(...);
    public static int d(...);
    public static int i(...);
}

# KEEP Kotlin null checks - stripping them hides bugs and causes crashes
# (Previously removed; CodeRabbit HIGH severity fix)
# -assumenosideeffects class kotlin.jvm.internal.Intrinsics { ... }

# Optimize enums
-optimizations !code/simplification/enum*

# Remove unused Compose stuff
-dontwarn androidx.compose.**

# ============================================
# SIZE OPTIMIZATIONS
# ============================================

# Merge classes where possible
-repackageclasses ''
-allowaccessmodification

# Remove unused code paths
-optimizationpasses 5

# Don't keep attributes we don't need
-keepattributes SourceFile,LineNumberTable  # Keep for crash reports
-renamesourcefileattribute SourceFile
