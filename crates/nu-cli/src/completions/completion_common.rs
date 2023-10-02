use crate::completions::{matches, CompletionOptions};
use nu_path::{expand_tilde, home_dir};
use std::path::{is_separator, Path, PathBuf, MAIN_SEPARATOR_STR};

const SEP: char = std::path::MAIN_SEPARATOR;

pub fn complete_rec(
    partial: &str,
    cwd: &Path,
    original_cwd: &str,
    options: &CompletionOptions,
    dir: bool,
) -> Vec<String> {
    let (base, trail) = match partial.split_once(SEP) {
        Some((base, trail)) => (base, trail),
        None => (partial, ""),
    };

    let mut completions = vec![];

    if let Ok(result) = cwd.read_dir() {
        for entry in result.filter_map(|e| e.ok()) {
            let entry_name = entry.file_name().to_string_lossy().into_owned();
            if matches(base, &entry_name, options) {
                if trail.is_empty() {
                    let path = entry.path();
                    let mut path_string = if original_cwd == "~" {
                        let completion_without_tilde = path
                            .strip_prefix(home_dir().unwrap_or(PathBuf::new()))
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .into_owned();
                        format!("~{}{}", SEP, completion_without_tilde)
                    } else {
                        pathdiff::diff_paths(path.clone(), original_cwd)
                            .unwrap_or(path)
                            .to_string_lossy()
                            .into_owned()
                    };
                    if entry.path().is_dir() {
                        path_string.push(SEP);
                    }

                    if !dir || entry.path().is_dir() {
                        completions.push(escape_path(path_string, dir));
                    }
                } else if entry.path().is_dir() {
                    completions.extend(complete_rec(
                        trail,
                        &entry.path(),
                        original_cwd,
                        options,
                        dir,
                    ));
                }
            }
        }
    }
    completions
}

pub fn plain_listdir(dir: bool, source: &str, home: bool) -> Vec<String> {
    let mut completions = vec![];

    if let Ok(result) = Path::new(source).read_dir() {
        for entry in result.filter_map(|e| e.ok()) {
            let path = entry.path();
            let mut path_string = if home {
                let completion_without_tilde = path
                    .strip_prefix(home_dir().unwrap_or(PathBuf::new()))
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned();
                format!("~{}{}", SEP, completion_without_tilde)
            } else {
                path.to_string_lossy().into_owned()
            };

            if entry.path().is_dir() {
                path_string.push(SEP);
            }
            // If we don't want a directory and the entry is a directory, exclude it
            // Apply DeMorgan's law on the above statement
            if !dir || entry.path().is_dir() {
                completions.push(escape_path(path_string, dir));
            }
        }
    }
    completions
}
pub fn complete_item(
    want_directory: bool,
    span: nu_protocol::Span,
    partial: &str,
    cwd: &str,
    options: &CompletionOptions,
) -> Vec<(nu_protocol::Span, String)> {
    if cfg!(target_os = "windows") {
        if let [_, ':'] = partial.chars().collect::<Vec<_>>()[..] {
            return plain_listdir(want_directory, &format!("{}{}", partial, SEP), false)
                .into_iter()
                .map(|f| (span, f))
                .collect();
        }
    }

    let corrected_path = homogenize_slashes(partial);
    let tilde_exists = corrected_path.starts_with('~');
    let tilde_expanded = expand_tilde(&corrected_path);
    if corrected_path.ends_with(SEP) && tilde_expanded.exists() {
        plain_listdir(
            want_directory,
            &tilde_expanded.to_string_lossy(),
            tilde_exists,
        )
    } else {
        let mut rec_input = &corrected_path[..];
        let mut original_cwd = cwd;
        let mut cwd = Path::new(cwd);
        let home = home_dir().unwrap_or(cwd.into());
        if cfg!(target_os = "windows") {
            match corrected_path.chars().collect::<Vec<_>>()[..] {
                [_, ':', SEP, ..] => {
                    cwd = Path::new(&rec_input[0..3]);
                    original_cwd = "";
                    rec_input = &rec_input[3..];
                }
                [SEP, ..] => {
                    cwd = Path::new(MAIN_SEPARATOR_STR);
                    original_cwd = "";
                    rec_input = &rec_input[1..];
                }
                _ => {}
            };
        }
        let mut rec_input_path = Path::new(rec_input);
        if rec_input_path.starts_with("~") {
            rec_input_path = rec_input_path.strip_prefix("~").unwrap_or(rec_input_path);
            cwd = &home;
            original_cwd = "~";
        }
        while rec_input_path.starts_with("..") {
            rec_input_path = rec_input_path.strip_prefix("..").unwrap_or(rec_input_path);
            cwd = cwd.parent().unwrap_or(cwd);
        }
        while rec_input_path.starts_with(".") {
            rec_input_path = rec_input_path.strip_prefix(".").unwrap_or(rec_input_path);
        }
        complete_rec(
            rec_input_path.to_string_lossy().into_owned().as_str(),
            cwd,
            original_cwd,
            options,
            want_directory,
        )
    }
    .into_iter()
    .map(|f| (span, f))
    .collect()
}

pub fn homogenize_slashes(partial: &str) -> String {
    if cfg!(target_os = "windows") {
        partial
            .chars()
            .map(|c| if is_separator(c) { SEP } else { c })
            .collect()
    } else {
        partial.to_string()
    }
}

// Fix files or folders with quotes or hashes
pub fn escape_path(path: String, dir: bool) -> String {
    let filename_contaminated = !dir
        && path.contains([
            '\'', '"', ' ', '#', '(', ')', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
        ]);
    let dirname_contaminated = dir && path.contains(['\'', '"', ' ', '#']);
    if filename_contaminated || dirname_contaminated {
        format!("`{path}`")
    } else {
        path
    }
}
