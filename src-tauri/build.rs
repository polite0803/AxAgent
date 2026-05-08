fn main() {
    // Enable cfg(mobile) when building for Android or iOS
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("android")
        || target.contains("ios")
        || cfg!(target_os = "android")
        || cfg!(target_os = "ios")
    {
        println!("cargo:rustc-cfg=mobile");
        println!("cargo:warning=Building for mobile target: {}", target);
    }

    tauri_build::build()
}
