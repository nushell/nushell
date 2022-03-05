#[cfg(windows)]
fn main() {
    embed_resource::compile_for("assets/nushell.rc", &["nu"])
}

#[cfg(not(windows))]
fn main() {}
