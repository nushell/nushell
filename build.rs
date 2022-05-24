#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set("ProductName", "Nushell");
    res.set("FileDescription", "Nushell");
    res.set("LegalCopyright", "Copyright (C) 2022");
    res.set_icon("assets/nu_logo.ico");
    res.compile()
        .expect("Failed to run the Windows resource compiler (rc.exe)");
}

#[cfg(not(windows))]
fn main() {}
