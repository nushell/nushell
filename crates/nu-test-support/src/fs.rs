use nu_path::AbsolutePathBuf;
use std::io::Read;
use std::path::{Path, PathBuf};

pub enum Stub<'a> {
    FileWithContent(&'a str, &'a str),
    FileWithContentToBeTrimmed(&'a str, &'a str),
    EmptyFile(&'a str),
    FileWithPermission(&'a str, bool),
}

pub fn file_contents(full_path: impl AsRef<Path>) -> String {
    let mut file = std::fs::File::open(full_path.as_ref()).expect("can not open file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("can not read file");
    contents
}

pub fn file_contents_binary(full_path: impl AsRef<Path>) -> Vec<u8> {
    let mut file = std::fs::File::open(full_path.as_ref()).expect("can not open file");
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).expect("can not read file");
    contents
}

pub fn line_ending() -> String {
    #[cfg(windows)]
    {
        String::from("\r\n")
    }

    #[cfg(not(windows))]
    {
        String::from("\n")
    }
}

pub fn delete_file_at(full_path: impl AsRef<Path>) {
    let full_path = full_path.as_ref();

    if full_path.exists() {
        std::fs::remove_file(full_path).expect("can not delete file");
    }
}

pub fn create_file_at(full_path: impl AsRef<Path>) -> Result<(), std::io::Error> {
    let full_path = full_path.as_ref();

    if full_path.parent().is_some() {
        panic!("path exists");
    }

    std::fs::write(full_path, b"fake data")
}

pub fn copy_file_to(source: &str, destination: &str) {
    std::fs::copy(source, destination).expect("can not copy file");
}

pub fn files_exist_at(files: Vec<impl AsRef<Path>>, path: impl AsRef<Path>) -> bool {
    files.iter().all(|f| {
        let mut loc = PathBuf::from(path.as_ref());
        loc.push(f);
        loc.exists()
    })
}

pub fn delete_directory_at(full_path: &str) {
    std::fs::remove_dir_all(PathBuf::from(full_path)).expect("can not remove directory");
}

pub fn executable_path() -> PathBuf {
    let mut path = binaries();
    path.push("nu");
    path.into()
}

pub fn installed_nu_path() -> PathBuf {
    let path = std::env::var_os(crate::NATIVE_PATH_ENV_VAR);
    which::which_in("nu", path, ".").unwrap_or_else(|_| executable_path())
}

pub fn root() -> AbsolutePathBuf {
    let manifest_dir = if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        AbsolutePathBuf::try_from(manifest_dir).expect("CARGO_MANIFEST_DIR is not an absolute path")
    } else {
        AbsolutePathBuf::try_from(env!("CARGO_MANIFEST_DIR"))
            .expect("CARGO_MANIFEST_DIR is not an absolute path")
    };

    let test_path = manifest_dir.join("Cargo.lock");
    if test_path.exists() {
        manifest_dir
    } else {
        manifest_dir
            .parent()
            .expect("Couldn't find the debug binaries directory")
            .parent()
            .expect("Couldn't find the debug binaries directory")
            .into()
    }
}

pub fn binaries() -> AbsolutePathBuf {
    let build_target = std::env::var("CARGO_BUILD_TARGET").unwrap_or_default();

    let profile = if let Ok(env_profile) = std::env::var("NUSHELL_CARGO_PROFILE") {
        env_profile
    } else if cfg!(debug_assertions) {
        "debug".into()
    } else {
        "release".into()
    };

    std::env::var("CARGO_TARGET_DIR")
        .ok()
        .and_then(|p| AbsolutePathBuf::try_from(p).ok())
        .unwrap_or_else(|| root().join("target"))
        .join(build_target)
        .join(profile)
}

pub fn fixtures() -> AbsolutePathBuf {
    root().join("tests").join("fixtures")
}

pub fn assets() -> AbsolutePathBuf {
    root().join("tests").join("assets")
}

pub fn in_directory(str: impl AsRef<Path>) -> String {
    let path = str.as_ref();
    let path = if path.is_relative() {
        root().join(path).into_any()
    } else {
        path.into()
    };

    path.display().to_string()
}
