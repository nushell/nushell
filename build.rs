fn main() {
    // convenience alias for trash support
    if cfg!(all(
        feature = "trash-support",
        not(any(target_os = "android", target_os = "ios"))
    )) {
        println!("cargo:rustc-cfg=trash");
    }

    if cfg!(all(
        feature = "trash-support",
        any(target_os = "android", target_os = "ios")
    )) {
        println!("cargo:warning=\"trash-support\" feature enabled on unsupported platform ");
    }
}
