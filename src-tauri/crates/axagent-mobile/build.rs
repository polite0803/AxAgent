// SPDX-License-Identifier: AGPL-3.0-only

fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    let is_mobile = target.contains("apple")
        && (target.contains("ios") || target.contains("tvos") || target.contains("watchos"))
        || target.contains("android");
    if is_mobile {
        println!("cargo:rustc-cfg=mobile");
        println!("cargo:warning=Building axagent-mobile for mobile target: {}", target);
    }

    println!("cargo::rustc-check-cfg=cfg(mobile)");
}
