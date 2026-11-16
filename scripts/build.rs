fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").expect("set by cargo");
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").expect("set by cargo");
    println!("cargo:rerun-if-changed=scripts/build.rs");
    println!(
        "cargo:rustc-env=NU_FEATURES={}",
        std::env::var("CARGO_CFG_FEATURE").expect("set by cargo")
    );

    if target_os == "windows" {
        println!("cargo:rerun-if-changed=assets/nu_logo.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/nu_logo.ico");
        res.compile()
            .expect("Failed to run the Windows resource compiler (rc.exe)");
    } else {
        if target_os == "macos"
            && target_arch == "aarch64"
            && std::env::var_os("RUSTC_LINKER").is_none()
        {
            // Link with chained fixups. At Rust's default deployment target (macOS 11) the linker
            // writes classic rebase opcodes instead, and dyld rewrites every pointer in the
            // binary's constant data before `main` runs. Since macOS 13 the kernel applies chained
            // fixups page by page as pages are first touched, so startup only pays for the pages
            // it uses. macOS 11 is the first release that loads them. That is why this is limited
            // to aarch64: every arm64 Mac runs macOS 11 or later, but x86_64's default deployment
            // target (10.12) predates them, and the binary would fail to load on 10.12-10.15.
            // Cargo sets `RUSTC_LINKER` only when the build configures its own linker, and some,
            // such as `zig cc` (cargo-zigbuild), reject the flag, so only Apple's default gets it.
            println!("cargo:rustc-link-arg-bins=-Wl,-fixup_chains");
        }
        // Tango uses dynamic linking, to allow us to dynamically change between two bench suit at runtime.
        // This is currently not supported on non nightly rust, on windows.
        println!("cargo:rustc-link-arg-benches=-rdynamic");
    }
}
