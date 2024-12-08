use crate::{
    engine::{EngineState, Stack},
    Span, Value,
};
#[cfg(windows)]
use nu_path::{
    bash_strip_redundant_quotes, ensure_trailing_delimiter, env_var_for_drive,
    extract_drive_letter, get_full_path_name_w, need_expand,
};
use std::path::{Path, PathBuf};

#[cfg(windows)]
pub fn set_pwd(stack: &mut Stack, path: &Path) {
    if let Some(drive) = extract_drive_letter(path) {
        let value = Value::string(path.to_string_lossy(), Span::unknown());
        stack.add_env_var(env_var_for_drive(drive), value.clone());
    }
}

// get pwd for drive:
// 1. From env_var, if no,
// 2. From sys_absolute, if no,
// 3. Construct root path to drives
#[cfg(windows)]
fn get_pwd_on_drive(stack: &Stack, engine_state: &EngineState, drive_letter: char) -> String {
    let env_var_for_drive = env_var_for_drive(drive_letter);
    let mut abs_pwd: Option<String> = None;
    if let Some(pwd) = stack.get_env_var(engine_state, &env_var_for_drive) {
        if let Ok(pwd_string) = pwd.clone().into_string() {
            abs_pwd = Some(pwd_string);
        }
    }
    if abs_pwd.is_none() {
        if let Some(sys_pwd) = get_full_path_name_w(&format!("{}:", drive_letter)) {
            abs_pwd = Some(sys_pwd);
        }
    }
    if let Some(pwd) = abs_pwd {
        ensure_trailing_delimiter(&pwd)
    } else {
        format!(r"{}:\", drive_letter)
    }
}

#[cfg(windows)]
pub fn expand_pwd(stack: &Stack, engine_state: &EngineState, path: &Path) -> Option<PathBuf> {
    if let Some(path_str) = path.to_str() {
        if let Some(path_string) = bash_strip_redundant_quotes(path_str) {
            if need_expand(&path_string) {
                if let Some(drive_letter) = extract_drive_letter(Path::new(&path_string)) {
                    let mut base =
                        PathBuf::from(get_pwd_on_drive(stack, engine_state, drive_letter));
                    // Combine PWD with the relative path
                    // need_expand() and extract_drive_letter() all ensure path_str.len() >= 2
                    base.push(&path_string[2..]); // Join PWD with path parts after "C:"
                    return Some(base);
                }
            }
            if path_string != path_str {
                return Some(PathBuf::from(&path_string));
            }
        }
    }
    None
}

// Helper stub/proxy for nu_path::expand_path_with::<P, Q>(path, relative_to, expand_tilde)
// Facilitates file system commands to easily gain the ability to expand PWD-per-drive
pub fn expand_path_with<P, Q>(
    _stack: &Stack,
    _engine_state: &EngineState,
    path: P,
    relative_to: Q,
    expand_tilde: bool,
) -> PathBuf
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    #[cfg(windows)]
    if let Some(abs_path) = expand_pwd(_stack, _engine_state, path.as_ref()) {
        return abs_path;
    }

    nu_path::expand_path_with::<P, Q>(path, relative_to, expand_tilde)
}

#[cfg(windows)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_pwd() {
        let mut stack = Stack::new();
        let path_str = r"c:\uesrs\nushell";
        let path = Path::new(path_str);
        set_pwd(&mut stack, path);
        let engine_state = EngineState::new();
        assert_eq!(
            stack
                .get_env_var(&engine_state, &env_var_for_drive('c'))
                .unwrap()
                .clone()
                .into_string()
                .unwrap(),
            path_str.to_string()
        );
    }

    #[test]
    fn test_get_pwd_on_drive() {
        let mut stack = Stack::new();
        let path_str = r"c:\users\nushell";
        let path = Path::new(path_str);
        set_pwd(&mut stack, path);
        let engine_state = EngineState::new();
        let result = format!(r"{path_str}\");
        assert_eq!(result, get_pwd_on_drive(&stack, &engine_state, 'c'));
    }

    #[test]
    fn test_expand_pwd() {
        let mut stack = Stack::new();
        let path_str = r"c:\users\nushell";
        let path = Path::new(path_str);
        set_pwd(&mut stack, path);
        let engine_state = EngineState::new();

        let rel_path = Path::new("c:.config");
        let result = format!(r"{path_str}\.config");
        assert_eq!(
            Some(result.as_str()),
            expand_pwd(&stack, &engine_state, rel_path)
                .unwrap()
                .as_path()
                .to_str()
        );
    }

    #[test]
    fn test_expand_path_with() {
        let mut stack = Stack::new();
        let path_str = r"c:\users\nushell";
        let path = Path::new(path_str);
        set_pwd(&mut stack, path);
        let engine_state = EngineState::new();

        let rel_path = Path::new("c:.config");
        let result = format!(r"{path_str}\.config");
        assert_eq!(
            Some(result.as_str()),
            expand_path_with(&stack, &engine_state, rel_path, Path::new(path_str), false)
                .as_path()
                .to_str()
        );
    }
}
