# Called from Rust over JNI by name (app/src/platform/jni.rs).
-keep class tech.lolli.toolbox.MainActivity { public *; native <methods>; }
-keep class com.google.androidgamesdk.** { *; }
# The widgets' Kotlin object, reached from MainActivity.
-keep class tech.lolli.toolbox.widget.** { *; }
