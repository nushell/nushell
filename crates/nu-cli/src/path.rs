// Copyright (C) 2020 Kevin Dc
//
// This file is part of nushell.
//
// nushell is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// nushell is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with nushell.  If not, see <http://www.gnu.org/licenses/>.

use std::io;
use std::path::{Component, Path, PathBuf};

pub fn normalize(path: impl AsRef<Path>) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.as_ref().components() {
        match component {
            Component::Normal(normal) => {
                if let Some(normal) = normal.to_str() {
                    if normal.chars().all(|c| c == '.') {
                        for _ in 0..(normal.len() - 1) {
                            normalized.push("..");
                        }
                    } else {
                        normalized.push(normal);
                    }
                } else {
                    normalized.push(normal);
                }
            }
            c => normalized.push(c.as_os_str()),
        }
    }

    normalized
}

fn canonicalize_core<P, Q>(relative_to: P, path: Q) -> PathBuf
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let path = normalize(path);
    let (relative_to, path) = if path.is_absolute() {
        let components: Vec<_> = path.components().collect();
        let separator = components
            .iter()
            .enumerate()
            .find(|(_, c)| c == &&Component::CurDir || c == &&Component::ParentDir);

        if let Some((index, _)) = separator {
            let (absolute, relative) = components.split_at(index);
            let absolute: PathBuf = absolute.iter().collect();
            let relative: PathBuf = relative.iter().collect();

            (absolute, relative)
        } else {
            (relative_to.as_ref().to_path_buf(), path)
        }
    } else {
        (relative_to.as_ref().to_path_buf(), path)
    };

    if path.is_relative() {
        let mut result = relative_to;
        path.components().for_each(|component| match component {
            Component::ParentDir => {
                result.pop();
            }
            Component::Normal(normal) => result.push(normal),
            _ => {}
        });

        result
    } else {
        path
    }
}

pub fn canonicalize_existing<P, Q>(relative_to: P, path: Q) -> io::Result<PathBuf>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let canonicalized = canonicalize_core(relative_to, path);
    let path = match std::fs::read_link(&canonicalized) {
        Ok(resolved) => resolved,
        Err(e) => {
            if canonicalized.exists() {
                canonicalized
            } else {
                return Err(e);
            }
        }
    };

    Ok(dunce::simplified(&path).to_path_buf())
}

#[allow(dead_code)]
pub fn canonicalize_missing<P, Q>(relative_to: P, path: Q) -> PathBuf
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let canonicalized = canonicalize_core(relative_to, path);
    let path = match std::fs::read_link(&canonicalized) {
        Ok(resolved) => resolved,
        Err(_) => canonicalized,
    };

    dunce::simplified(&path).to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn normalize_three_dots() {
        assert_eq!(PathBuf::from("../.."), normalize("..."));
    }

    #[test]
    fn normalize_three_dots_with_redundant_dot() {
        assert_eq!(PathBuf::from("./../.."), normalize("./..."));
    }

    #[test]
    fn canonicalize_missing_two_dots() {
        let relative_to = Path::new("/foo/bar");
        let path = Path::new("..");

        assert_eq!(
            PathBuf::from("/foo"), // missing path
            canonicalize_missing(relative_to, path)
        );
    }

    #[test]
    fn canonicalize_missing_three_dots() {
        let relative_to = Path::new("/foo/bar/baz");
        let path = Path::new("...");

        assert_eq!(
            PathBuf::from("/foo"), // missing path
            canonicalize_missing(relative_to, path)
        );
    }

    #[test]
    fn canonicalize_missing_three_dots_with_redundant_dot() {
        let relative_to = Path::new("/foo/bar/baz");
        let path = Path::new("./...");

        assert_eq!(
            PathBuf::from("/foo"), // missing path
            canonicalize_missing(relative_to, path)
        );
    }

    #[test]
    fn canonicalize_existing_three_dots() -> io::Result<()> {
        let relative_to = Path::new("/foo/bar/");
        let path = Path::new("...");

        assert_eq!(
            PathBuf::from("/"), // existing path
            canonicalize_existing(relative_to, path)?
        );

        Ok(())
    }

    #[test]
    fn canonicalize_existing_three_dots_should_fail() {
        let relative_to = Path::new("/foo/bar/baz"); // '/foo' is missing
        let path = Path::new("...");

        assert!(canonicalize_existing(relative_to, path).is_err());
    }
}
