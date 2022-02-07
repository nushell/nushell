/// Trim trailing path separator from a string
pub fn trim_trailing_slash(s: &str) -> &str {
    s.trim_end_matches(std::path::is_separator)
}
