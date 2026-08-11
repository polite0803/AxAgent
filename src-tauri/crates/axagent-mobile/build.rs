// SPDX-License-Identifier: AGPL-3.0-only

fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("apple")
        && (target.contains("ios") || target.contains("tvos") || target.contains("watchos"))
    {
        println!("cargo:rustc-cfg=mobile");
        println!("cargo:warning=Building axagent-mobile for Apple mobile target: {}", target);
    }

    println!("cargo::rustc-check-cfg=cfg(mobile)");
}
