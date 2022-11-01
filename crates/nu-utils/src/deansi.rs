use std::borrow::Cow;

/// Removes ANSI escape codes and some ASCII control characters
///
/// Optimized for strings that rarely contain ANSI control chars.
/// Uses fast search to avoid reallocations.
///
/// Keeps `\n` removes `\r`, `\t` etc.
///
/// If parsing fails silently returns the input string
pub fn strip_ansi_unlikely(string: &str) -> Cow<str> {
    // Check if any ascii control character except LF(0x0A = 10) is present,
    // which will be stripped. Includes the primary start of ANSI sequences ESC
    // (0x1B = decimal 27)
    if string.bytes().any(|x| matches!(x, 0..=9 | 11..=31)) {
        if let Ok(stripped) = strip_ansi_escapes::strip(string) {
            if let Ok(new_string) = String::from_utf8(stripped) {
                return Cow::Owned(new_string);
            }
        }
    }
    // Else case includes failures to parse!
    Cow::Borrowed(string)
}

/// Removes ANSI escape codes and some ASCII control characters
///
/// Optimized for strings that likely contain ANSI control chars.
///
/// Keeps `\n` removes `\r`, `\t` etc.
///
/// If parsing fails silently returns the input string
pub fn strip_ansi_likely(string: &str) -> Cow<str> {
    // Check if any ascii control character except LF(0x0A = 10) is present,
    // which will be stripped. Includes the primary start of ANSI sequences ESC
    // (0x1B = decimal 27)
    if let Ok(stripped) = strip_ansi_escapes::strip(string) {
        if let Ok(new_string) = String::from_utf8(stripped) {
            return Cow::Owned(new_string);
        }
    }
    // Else case includes failures to parse!
    Cow::Borrowed(string)
}

/// Removes ANSI escape codes and some ASCII control characters
///
/// Optimized for strings that rarely contain ANSI control chars.
/// Uses fast search to avoid reallocations.
///
/// Keeps `\n` removes `\r`, `\t` etc.
///
/// If parsing fails silently returns the input string
pub fn strip_ansi_string_unlikely(string: String) -> String {
    // Check if any ascii control character except LF(0x0A = 10) is present,
    // which will be stripped. Includes the primary start of ANSI sequences ESC
    // (0x1B = decimal 27)
    if string
        .as_str()
        .bytes()
        .any(|x| matches!(x, 0..=9 | 11..=31))
    {
        if let Ok(stripped) = strip_ansi_escapes::strip(&string) {
            if let Ok(new_string) = String::from_utf8(stripped) {
                return new_string;
            }
        }
    }
    // Else case includes failures to parse!
    string
}

/// Removes ANSI escape codes and some ASCII control characters
///
/// Optimized for strings that likely contain ANSI control chars.
///
/// Keeps `\n` removes `\r`, `\t` etc.
///
/// If parsing fails silently returns the input string
pub fn strip_ansi_string_likely(string: String) -> String {
    // Check if any ascii control character except LF(0x0A = 10) is present,
    // which will be stripped. Includes the primary start of ANSI sequences ESC
    // (0x1B = decimal 27)
    if let Ok(stripped) = strip_ansi_escapes::strip(&string) {
        if let Ok(new_string) = String::from_utf8(stripped) {
            return new_string;
        }
    }
    // Else case includes failures to parse!
    string
}
