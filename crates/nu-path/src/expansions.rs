use std::io;
use std::path::{Path, PathBuf};

use super::dots::{expand_dots, expand_ndots};
use super::tilde::expand_tilde;

// Trace a relative path back to its root starting from current directory.
// Returns error if not possible.
// If path is absolute, just return it.
// fn absolutize(path: impl AsRef<Path>) -> io::Result<PathBuf> {
//     if path.as_ref().is_absolute() {
//         Ok(path.as_ref().into())
//     } else {
//         Ok(std::env::current_dir()?.join(path))
//     }
// }

// Trace a relative path back to its root starting from a custom directory.
// Returns error if not possible.
// If path is absolute, just return it.
// fn absolutize_with<P, Q>(path: P, relative_to: Q) -> io::Result<PathBuf>
// where
//     P: AsRef<Path>,
//     Q: AsRef<Path>,
// {
//     if path.as_ref().is_absolute() {
//         Ok(path.as_ref().into())
//     } else {
//         if relative_to.as_ref().is_absolute() {
//             Ok(relative_to.as_ref().join(path))
//         } else {
//             Ok(absolutize(relative_to)?.join(path))
//         }
//     }
// }

pub fn canonicalize(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    dunce::canonicalize(path)
}

pub fn canonicalize_with<P, Q>(path: P, relative_to: Q) -> io::Result<PathBuf>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let path = path.as_ref();
    let relative_to = relative_to.as_ref();

    let path = if path == Path::new(".") {
        // Joining a Path with '.' appends a '.' at the end, making the prompt
        // more ugly - so we don't do anything, which should result in an equal
        // path on all supported systems.
        relative_to.to_owned()
    } else if path.starts_with("~") {
        // TODO: No need for this branch
        expand_tilde(path)
    } else {
        relative_to.join(path)
    };

    canonicalize(path)
}

// Expands ~ to home and shortens paths by removing unecessary ".." and "."
// where possible. Also expands "...+" appropriately.
// Does not convert to absolute form nor does it resolve symlinks.
pub fn expand_path(path: impl AsRef<Path>) -> PathBuf {
    let path = expand_tilde(path);
    let path = expand_ndots(path);
    expand_dots(path)
}

pub fn expand_path_with<P, Q>(path: P, relative_to: Q) -> PathBuf
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let path = path.as_ref();
    let relative_to = relative_to.as_ref();

    let path = if path == Path::new(".") {
        // Joining a Path with '.' appends a '.' at the end, making the prompt
        // more ugly - so we don't do anything, which should result in an equal
        // path on all supported systems.
        relative_to.into()
    } else if path.starts_with("~") {
        // TODO: No need for this branch
        expand_tilde(path)
    } else {
        relative_to.join(path)
    };

    expand_path(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    // use std::io;

    // TODO: Reformulate for expand_path
    // #[test]
    // fn canonicalize_with_and_without_relative() -> io::Result<()> {
    //     let relative_to = Path::new("/foo/bar");
    //     let path = Path::new("../..");
    //     let full_path = Path::new("/foo/bar/../..");

    //     assert_eq!(
    //         canonicalize(full_path)?,
    //         canonicalize_with(path, relative_to)?,
    //     );

    //     Ok(())
    // }

    // TODO: Reformulate for expand_path
    // #[test]
    // fn canonicalize_should_succeed() -> io::Result<()> {
    //     let relative_to = Path::new("/foo/bar");
    //     let path = Path::new("../..");

    //     assert_eq!(
    //         PathBuf::from("/"), // existing path
    //         canonicalize_with(path, relative_to)?,
    //     );

    //     Ok(())
    // }

    #[test]
    fn canonicalize_should_fail() {
        let relative_to = Path::new("/foo/bar/baz"); // '/foo' is missing
        let path = Path::new("../..");

        assert!(canonicalize_with(path, relative_to).is_err());
    }
}
