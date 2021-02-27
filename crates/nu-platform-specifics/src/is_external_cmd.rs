#[cfg(any(target_arch = "wasm32", not(feature = "which")))]
pub fn is_external_cmd(#[allow(unused)] name: &str) -> bool {
    true
}

#[cfg(all(unix, feature = "which"))]
pub fn is_external_cmd(#[allow(unused)] name: &str) -> bool {
    which::which(name).is_ok()
}

#[cfg(all(windows, feature = "which"))]
pub fn is_external_cmd(#[allow(unused)] name: &str) -> bool {
    if which::which(name).is_ok() {
        true
    } else {
        // Reference: https://ss64.com/nt/syntax-internal.html
        let cmd_builtins = [
            "assoc", "break", "color", "copy", "date", "del", "dir", "dpath", "echo", "erase",
            "for", "ftype", "md", "mkdir", "mklink", "move", "path", "ren", "rename", "rd",
            "rmdir", "start", "time", "title", "type", "ver", "verify", "vol",
        ];

        cmd_builtins.contains(&name)
    }
}
