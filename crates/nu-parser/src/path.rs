use std::path::{Component, Path, PathBuf};

fn expand_ndots(path: &str) -> String {
    let path = Path::new(path);
    let mut expanded = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Normal(normal) => {
                if let Some(normal) = normal.to_str() {
                    if normal.chars().all(|c| c == '.') {
                        for _ in 0..(normal.len() - 1) {
                            expanded.push("..");
                        }
                    } else {
                        expanded.push(normal);
                    }
                } else {
                    expanded.push(normal);
                }
            }

            c => expanded.push(c.as_os_str()),
        }
    }

    expanded.to_string_lossy().to_string()
}

pub fn expand_path(path: &str) -> String {
    let tilde_expansion = shellexpand::tilde(path);
    expand_ndots(&tilde_expansion)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_in_relative_path() {
        assert_eq!(String::from("../.."), expand_path("..."));
    }

    #[test]
    fn expand_in_absolute_path() {
        assert_eq!(String::from("/foo/../.."), expand_path("/foo/..."));
    }
}
