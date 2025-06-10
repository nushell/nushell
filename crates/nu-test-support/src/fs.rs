use nu_path::{AbsolutePath, AbsolutePathBuf, Path};
use std::io::Read;

pub enum Stub<'a> {
    FileWithContent(&'a str, &'a str),
    FileWithContentToBeTrimmed(&'a str, &'a str),
    EmptyFile(&'a str),
    FileWithPermission(&'a str, bool),
}

pub fn file_contents(full_path: impl AsRef<AbsolutePath>) -> String {
    let mut file = std::fs::File::open(full_path.as_ref()).expect("can not open file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("can not read file");
    contents
}

pub fn file_contents_binary(full_path: impl AsRef<AbsolutePath>) -> Vec<u8> {
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

pub fn files_exist_at(files: &[impl AsRef<Path>], path: impl AsRef<AbsolutePath>) -> bool {
    let path = path.as_ref();
    files.iter().all(|f| path.join(f.as_ref()).exists())
}

pub fn executable_path() -> AbsolutePathBuf {
    let mut path = binaries();
    path.push("nu");
    path
}

pub fn installed_nu_path() -> AbsolutePathBuf {
    let path = std::env::var_os(crate::NATIVE_PATH_ENV_VAR);
    if let Ok(path) = which::which_in("nu", path, ".") {
        AbsolutePathBuf::try_from(path).expect("installed nushell path is absolute")
    } else {
        executable_path()
    }
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
        .or_else(|_| std::env::var("CARGO_BUILD_TARGET_DIR"))
        .ok()
        .and_then(|p| AbsolutePathBuf::try_from(p).ok())
        .unwrap_or_else(|| root().join("target"))
        .join(build_target)
        .join(profile)
}

pub fn fixtures() -> AbsolutePathBuf {
    let mut path = root();
    path.push("tests");
    path.push("fixtures");
    path
}

pub fn assets() -> AbsolutePathBuf {
    let mut path = root();
    path.push("tests");
    path.push("assets");
    path
}

pub fn in_directory(path: impl AsRef<nu_path::Path>) -> AbsolutePathBuf {
    root().join(path)
}
