// Character used to separate directories in a PATH environment variable on Windows is ";".
#[cfg(target_family = "windows")]
pub const ENV_PATH_SEPARATOR_CHAR: char = ';';
// Character used to separate directories in a PATH environment variable on Linux/macOS/Unix is ":".
#[cfg(not(target_family = "windows"))]
pub const ENV_PATH_SEPARATOR_CHAR: char = ':';

// Line separator used on Windows is "\r\n".
#[cfg(target_family = "windows")]
pub const LINE_SEPARATOR_CHAR: &str = "\r\n";
// Line separator used on Linux/macOS/Unix is "\n".
#[cfg(not(target_family = "windows"))]
pub const LINE_SEPARATOR_CHAR: char = '\n';
