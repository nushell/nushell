//! Path equality in Rust is defined by comparing their `components()`. However,
//! `Path::components()` will perform its own normalization, which makes
//! `assert_eq!` not suitable testing.
//!
//! This module provides two macros, `assert_path_eq!` and `assert_path_ne!`,
//! which converts path to string before comparison. They accept PathBuf, Path,
//! String, and &str as parameters.

#[macro_export]
macro_rules! assert_path_eq {
    ($left:expr, $right:expr $(,)?) => {
        assert_eq!(
            AsRef::<Path>::as_ref(&$left).to_str().unwrap(),
            AsRef::<Path>::as_ref(&$right).to_str().unwrap()
        )
    };
}

#[macro_export]
macro_rules! assert_path_ne {
    ($left:expr, $right:expr $(,)?) => {
        assert_ne!(
            AsRef::<Path>::as_ref(&$left).to_str().unwrap(),
            AsRef::<Path>::as_ref(&$right).to_str().unwrap()
        )
    };
}

#[cfg(test)]
mod test {
    use std::path::{Path, PathBuf};

    #[test]
    fn assert_path_eq_works() {
        assert_path_eq!(PathBuf::from("/foo/bar"), Path::new("/foo/bar"));
        assert_path_eq!(PathBuf::from("/foo/bar"), String::from("/foo/bar"));
        assert_path_eq!(PathBuf::from("/foo/bar"), "/foo/bar");
        assert_path_eq!(Path::new("/foo/bar"), String::from("/foo/bar"));
        assert_path_eq!(Path::new("/foo/bar"), "/foo/bar");
        assert_path_eq!(Path::new(r"\foo\bar"), r"\foo\bar");

        assert_path_ne!(PathBuf::from("/foo/bar/."), Path::new("/foo/bar"));
        assert_path_ne!(PathBuf::from("/foo/bar/."), String::from("/foo/bar"));
        assert_path_ne!(PathBuf::from("/foo/bar/."), "/foo/bar");
        assert_path_ne!(Path::new("/foo/./bar"), String::from("/foo/bar"));
        assert_path_ne!(Path::new("/foo/./bar"), "/foo/bar");
        assert_path_ne!(Path::new(r"\foo\bar"), r"/foo/bar");
        assert_path_ne!(Path::new(r"/foo/bar"), r"\foo\bar");
    }
}
