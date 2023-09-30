use crate::completions::{matches, CompletionOptions};
use std::path::Path;

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
                    let mut path_string = pathdiff::diff_paths(path.clone(), original_cwd)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .into_owned();
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
