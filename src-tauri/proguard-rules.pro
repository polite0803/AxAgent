# AxAgent Android ProGuard / R8 Keep Rules
# 保护所有 JNI 桥接类，防止 R8 压缩/混淆导致 UnsatisfiedLinkError 闪退

# ── 应用自身的 JNI 桥接类（最关键！）──
-keep class top.axagent.desktop.** { *; }
-keepclassmembers class top.axagent.desktop.** { *; }

# ── Tauri 框架 JNI 桥接类 ──
-keep class com.tauri.** { *; }
-keep class org.tauri.** { *; }
-keep class android.webkit.** { *; }

# ── Wry WebView JNI 桥接类 ──
-keep class ipc.** { *; }

# ── Serde / Rust ↔ Kotlin 序列化类 ──
-keepattributes *Annotation*
-keepattributes Signature
-keepattributes EnclosingMethod

# ── JNI 通用保护规则 ──
-keepclasseswithmembernames class * {
    native <methods>;
}
-keepclassmembers class * {
    @android.webkit.JavascriptInterface <methods>;
}

# ── 禁止混淆（JNI 依赖精确类名/方法名）──
-dontobfuscate

# ── 保留 Rust panic 回溯信息 ──
-keepattributes SourceFile, LineNumberTable
-renamesourcefileattribute SourceFile

# ── 第三方库 ──
-dontwarn com.tauri.**
-dontwarn org.tauri.**
-dontwarn ipc.**
-dontwarn top.axagent.desktop.**
