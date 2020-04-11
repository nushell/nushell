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

pub struct AllowMissing(pub bool);

pub fn canonicalize<P, Q>(
    relative_to: P,
    path: Q,
    allow_missing: AllowMissing,
) -> io::Result<PathBuf>
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

    let path = if path.is_relative() {
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
    };

    let path = match std::fs::read_link(&path) {
        Ok(resolved) => resolved,
        Err(e) => {
            // Fail if path doesn't exists or is not symlink
            // Check if we allow missing paths or the path exists
            if allow_missing.0 || path.exists() {
                path
            } else {
                return Err(e);
            }
        }
    };

    // De-UNC paths
    Ok(dunce::simplified(&path).to_path_buf())
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
    fn canonicalize_two_dots_and_allow_missing() -> io::Result<()> {
        let relative_to = Path::new("/foo/bar"); // does not exists
        let path = Path::new("..");

        assert_eq!(
            PathBuf::from("/foo"),
            canonicalize(relative_to, path, AllowMissing(true))?
        );

        Ok(())
    }

    #[test]
    fn canonicalize_three_dots_and_allow_missing() -> io::Result<()> {
        let relative_to = Path::new("/foo/bar/baz"); // missing path
        let path = Path::new("...");

        assert_eq!(
            PathBuf::from("/foo"),
            canonicalize(relative_to, path, AllowMissing(true))?
        );

        Ok(())
    }

    #[test]
    fn canonicalize_three_dots_with_redundant_dot_and_allow_missing() -> io::Result<()> {
        let relative_to = Path::new("/foo/bar/baz"); // missing path
        let path = Path::new("./...");

        assert_eq!(
            PathBuf::from("/foo"),
            canonicalize(relative_to, path, AllowMissing(true))?
        );

        Ok(())
    }

    #[test]
    fn canonicalize_three_dots_and_disallow_missing() -> io::Result<()> {
        let relative_to = Path::new("/foo/bar/"); // root is not missing
        let path = Path::new("...");

        assert_eq!(
            PathBuf::from("/"),
            canonicalize(relative_to, path, AllowMissing(false))?
        );

        Ok(())
    }

    #[test]
    fn canonicalize_three_dots_and_disallow_missing_should_fail() {
        let relative_to = Path::new("/foo/bar/baz"); // foo is missing
        let path = Path::new("...");

        assert!(canonicalize(relative_to, path, AllowMissing(false)).is_err());
    }
}
