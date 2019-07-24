#![allow(dead_code)]

pub use std::path::{Path, PathBuf};

use log::trace;
use std::io::Read;
use tempdir::TempDir;

#[macro_export]
macro_rules! nu {
    ($out:ident, $cwd:expr, $commands:expr) => {
        pub use std::error::Error;
        pub use std::io::prelude::*;
        pub use std::process::{Command, Stdio};

        let commands = &*format!(
            "
                            cd {}
                            {}
                            exit",
            $cwd, $commands
        );

        let process = match Command::new(helpers::executable_path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(why) => panic!("Can't run test {}", why.description()),
        };

        match process.stdin.unwrap().write_all(commands.as_bytes()) {
            Err(why) => panic!("couldn't write to wc stdin: {}", why.description()),
            Ok(_) => {}
        }

        let mut _s = String::new();

        match process.stdout.unwrap().read_to_string(&mut _s) {
            Err(why) => panic!("couldn't read stdout: {}", why.description()),
            Ok(_) => {
                let _s = _s.replace("\r\n", "\n");
            }
        }

        let _s = _s.replace("\r\n", "");
        let $out = _s.replace("\n", "");
    };
}

#[macro_export]
macro_rules! nu_error {
    ($out:ident, $cwd:expr, $commands:expr) => {
        use std::io::prelude::*;
        use std::process::{Command, Stdio};

        let commands = &*format!(
            "
                            cd {}
                            {}
                            exit",
            $cwd, $commands
        );

        let mut process = Command::new(helpers::executable_path())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("couldn't run test");

        let stdin = process.stdin.as_mut().expect("couldn't open stdin");
        stdin
            .write_all(commands.as_bytes())
            .expect("couldn't write to stdin");

        let output = process
            .wait_with_output()
            .expect("couldn't read from stderr");
        let $out = String::from_utf8_lossy(&output.stderr);
    };
}

pub fn setup_playground_for(topic: &str) -> Result<(TempDir, TempDir, String), std::io::Error> {
    let home = TempDir::new("nuplayground")?;
    let child = TempDir::new_in(home.path(), topic)?;
    let relative = child
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .expect(&format!(
            "file name {} was not valid",
            child.path().display()
        ))
        .to_string();

    trace!(
        "created {:?} dir={}",
        child.path().display(),
        child.path().is_dir()
    );

    Ok((home, child, relative))
}

pub fn file_contents(full_path: impl AsRef<Path>) -> String {
    let mut file = std::fs::File::open(full_path).expect("can not open file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("can not read file");
    contents
}

pub fn create_file_at(full_path: impl AsRef<Path>) -> Result<(), std::io::Error> {
    let full_path = full_path.as_ref();

    assert!(
        full_path.parent().unwrap().is_dir(),
        "{:?} exists",
        full_path.parent().unwrap().display(),
    );
    std::fs::write(full_path, "fake data".as_bytes())
}

pub fn file_exists_at(full_path: &str) -> bool {
    PathBuf::from(full_path).exists()
}

pub fn delete_directory_at(full_path: &Path) {
    std::fs::remove_dir_all(PathBuf::from(full_path)).expect("can not remove directory");
}

pub fn create_directory_at(full_path: &Path) {
    let path = PathBuf::from(full_path);

    println!("{:?} - is_dir: {:?}", path, path.is_dir());

    if !path.is_dir() {
        std::fs::create_dir_all(PathBuf::from(full_path))
            .expect(&format!("can not create directory {:?}", full_path));
    }
}

pub fn executable_path() -> PathBuf {
    let mut buf = PathBuf::new();
    buf.push("target");
    buf.push("debug");
    buf.push("nu");
    buf
}

pub fn in_directory(str: &str) -> &str {
    str
}
