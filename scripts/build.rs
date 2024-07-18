#[cfg(windows)]
fn main() {
    let mut res = winresource::WindowsResource::new();
    res.set("ProductName", "Nushell");
    res.set("FileDescription", "Nushell");
    res.set("LegalCopyright", "Copyright (C) 2022");
    res.set_icon("assets/nu_logo.ico");
    res.compile()
        .expect("Failed to run the Windows resource compiler (rc.exe)");
}

#[cfg(not(windows))]
fn main() {
    // Tango uses dynamic linking, to allow us to dynamically change between two bench suit at runtime.
    // This is currently not supported on non nightly rust, on windows.
    println!("cargo:rustc-link-arg-benches=-rdynamic");
    println!("cargo:rerun-if-changed=scripts/build.rs");
}
