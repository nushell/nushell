fn main() {
    println!("cargo:rerun-if-changed=scripts/build.rs");
    println!(
        "cargo:rustc-env=NU_FEATURES={}",
        std::env::var("CARGO_CFG_FEATURE").expect("set by cargo")
    );

    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/nu_logo.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/nu_logo.ico");
        res.compile()
            .expect("Failed to run the Windows resource compiler (rc.exe)");
    }

    #[cfg(not(windows))]
    {
        // Tango uses dynamic linking, to allow us to dynamically change between two bench suit at runtime.
        // This is currently not supported on non nightly rust, on windows.
        println!("cargo:rustc-link-arg-benches=-rdynamic");
    }
}
