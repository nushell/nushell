/// Returns true if `name` refers to an external command
pub fn is_external_cmd(#[allow(unused)] name: &str) -> bool {
    #[cfg(not(feature = "which"))]
    {
        // we can't perform this check, so just assume it can be found
        true
    }

    #[cfg(all(feature = "which", unix))]
    {
        which::which(name).is_ok()
    }

    #[cfg(all(feature = "which", windows))]
    {
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
}
