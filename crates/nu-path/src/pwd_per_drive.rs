/// Usage for pwd_per_drive on windows
///
/// See nu_protocol::engine::pwd_per_drive_helper;
///
use std::path::Path;

/// Ensure a path has a trailing `\\` or '/'
/// ```
/// use nu_path::ensure_trailing_delimiter;
///
/// assert_eq!(ensure_trailing_delimiter("E:"), r"E:\");
/// assert_eq!(ensure_trailing_delimiter(r"e:\"), r"e:\");
/// assert_eq!(ensure_trailing_delimiter("c:/"), "c:/");
/// ```
pub fn ensure_trailing_delimiter(path: &str) -> String {
    if !path.ends_with('\\') && !path.ends_with('/') {
        format!(r"{}\", path)
    } else {
        path.to_string()
    }
}

/// Remove leading quote and matching quote at back
/// "D:\\"Music -> D:\\Music
/// ```
/// use nu_path::bash_strip_redundant_quotes;
///
/// let input = r#""D:\Music""#;
/// let result = Some(r"D:\Music".to_string());
/// assert_eq!(result, bash_strip_redundant_quotes(input));
///
/// let input = r#"""""D:\Music"""""#;
/// assert_eq!(result, bash_strip_redundant_quotes(input));
///
/// let input = r#""D:\Mus"ic"#;
/// assert_eq!(result, bash_strip_redundant_quotes(input));
/// let input = r#""D:"\Music"#;
/// assert_eq!(result, bash_strip_redundant_quotes(input));
///
/// let input = r#""D":\Music"#;
/// assert_eq!(result, bash_strip_redundant_quotes(input));
///
/// let input = r#"""D:\Music"#;
/// assert_eq!(result, bash_strip_redundant_quotes(input));
///
/// let input = r#"""''"""D:\Mu sic"""''"""#;
/// let result = Some(r#""D:\Mu sic""#.to_string());
/// assert_eq!(result, bash_strip_redundant_quotes(input));
///
/// assert_eq!(bash_strip_redundant_quotes(""), Some("".to_string()));
/// assert_eq!(bash_strip_redundant_quotes("''"), Some("".to_string()));
/// assert_eq!(bash_strip_redundant_quotes("'''"), None);
/// assert_eq!(bash_strip_redundant_quotes("'''M'"), Some("M".to_string()));
/// assert_eq!(
///     bash_strip_redundant_quotes("'''M '"),
///     Some("'M '".to_string())
/// );
/// assert_eq!(
///     bash_strip_redundant_quotes(r#"""''"""D:\Mu sic"""''"""#),
///     crate::pwd_per_drive::bash_strip_redundant_quotes(r#""D:\Mu sic""#.to_string())
/// );
/// ```
pub fn bash_strip_redundant_quotes(input: &str) -> Option<String> {
    let mut result = String::new();
    let mut i = 0;
    let chars: Vec<char> = input.chars().collect();

    let mut no_quote_start_pos = 0;
    while i < chars.len() {
        let current_char = chars[i];

        if current_char == '"' || current_char == '\'' {
            if i > no_quote_start_pos {
                result.push_str(&input[no_quote_start_pos..i]);
            }

            let mut j = i + 1;
            let mut has_space = false;

            // Look for the matching quote
            while j < chars.len() && chars[j] != current_char {
                if chars[j].is_whitespace() {
                    has_space = true;
                }
                j += 1;
            }

            // Check if the matching quote exists
            if j < chars.len() && chars[j] == current_char {
                if has_space {
                    // Push the entire segment including quotes
                    result.push_str(&input[i..=j]);
                } else {
                    // Push the inner content without quotes
                    result.push_str(&input[i + 1..j]);
                }
                i = j + 1; // Move past the closing quote
                no_quote_start_pos = i;
                continue;
            } else {
                // No matching quote found, return None
                return None;
            }
        }
        i += 1;
    }

    if i > no_quote_start_pos + 1 {
        result.push_str(&input[no_quote_start_pos..i]);
    }
    // Return the result if matching quotes are found
    Some(result)
}

/// cmd_strip_all_double_quotes
/// ```
/// use nu_path::cmd_strip_all_double_quotes;
/// assert_eq!("t t", cmd_strip_all_double_quotes("t\" \"t\"\""));
/// ```
pub fn cmd_strip_all_double_quotes(input: &str) -> String {
    input.replace("\"", "")
}

/// get_full_path_name_w
/// Call windows system API (via omnipath crate) to expand
/// absolute path
/// ```
///  use nu_path::get_full_path_name_w;
///
///  let result = get_full_path_name_w("C:");
///  assert!(result.is_some());
///  let path = result.unwrap();
///  assert!(path.starts_with(r"C:\"));
///
///  let result = get_full_path_name_w(r"c:nushell\src");
///  assert!(result.is_some());
///  let path = result.unwrap();
///  assert!(path.starts_with(r"C:\") || path.starts_with(r"c:\"));
///  assert!(path.ends_with(r"nushell\src"));
/// ```
pub fn get_full_path_name_w(path_str: &str) -> Option<String> {
    use omnipath::sys_absolute;
    if let Ok(path_sys_abs) = sys_absolute(Path::new(path_str)) {
        Some(path_sys_abs.to_str()?.to_string())
    } else {
        None
    }
}
