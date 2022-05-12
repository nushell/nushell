// Skip embedding resource during tests because it's fairly slow and not needed
#[cfg(all(windows, not(test)))]
fn main() {
    embed_resource::compile_for("assets/nushell.rc", &["nu"])
}

#[cfg(any(not(windows), test))]
fn main() {}
