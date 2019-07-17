#![allow(dead_code)]

pub use std::path::PathBuf;

#[macro_export]
macro_rules! nu {
    ($out:ident, $cwd:expr, $commands:expr) => {
        pub use std::io::prelude::*;
        pub use std::process::{Command, Stdio};
        pub use std::error::Error;

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

pub fn create_file_at(full_path: &str) {
    std::fs::write(PathBuf::from(full_path), "fake data".as_bytes()).expect("can not create file");
}

pub fn file_exists_at(full_path: &str) -> bool {
    PathBuf::from(full_path).exists()
}

pub fn delete_directory_at(full_path: &str) {
    std::fs::remove_dir_all(PathBuf::from(full_path)).expect("can not remove directory");
}

pub fn create_directory_at(full_path: &str) {
    std::fs::create_dir(PathBuf::from(full_path)).expect("can not create directory");
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
