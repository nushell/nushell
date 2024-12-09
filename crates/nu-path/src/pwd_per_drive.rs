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
    use std::path::Path;

    if let Ok(path_sys_abs) = sys_absolute(Path::new(path_str)) {
        Some(path_sys_abs.to_str()?.to_string())
    } else {
        None
    }
}
