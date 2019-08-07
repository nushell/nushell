#![allow(dead_code)]

use glob::glob;
pub use std::path::Path;
pub use std::path::PathBuf;

use std::io::Read;

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

        let mut process = match Command::new(helpers::executable_path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(why) => panic!("Can't run test {}", why.description()),
        };

        let stdin = process.stdin.as_mut().expect("couldn't open stdin");
        stdin
            .write_all(commands.as_bytes())
            .expect("couldn't write to stdin");

        let output = process
            .wait_with_output()
            .expect("couldn't read from stdout");

        let $out = String::from_utf8_lossy(&output.stdout);
        let $out = $out.replace("\r\n", "");
        let $out = $out.replace("\n", "");
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

pub enum Stub<'a> {
    FileWithContent(&'a str, &'a str),
    EmptyFile(&'a str),
}

pub struct Playground {
    tests: String,
    cwd: PathBuf,
}

impl Playground {
    pub fn root() -> String {
        String::from("tests/fixtures/nuplayground")
    }

    pub fn test_dir_name(&self) -> String {
        self.tests.clone()
    }

    pub fn back_to_playground(&mut self) -> &mut Self {
        self.cwd = PathBuf::from([Playground::root(), self.tests.clone()].join("/"));
        self
    }

    pub fn setup_for(topic: &str) -> Playground {
        let nuplay_dir = format!("{}/{}", Playground::root(), topic);

        if PathBuf::from(&nuplay_dir).exists() {
            std::fs::remove_dir_all(PathBuf::from(&nuplay_dir)).expect("can not remove directory");
        }

        std::fs::create_dir(PathBuf::from(&nuplay_dir)).expect("can not create directory");

        Playground {
            tests: topic.to_string(),
            cwd: PathBuf::from([Playground::root(), topic.to_string()].join("/")),
        }
    }

    pub fn cd(&mut self, path: &str) -> &mut Self {
        self.cwd.push(path);
        self
    }

    pub fn with_files(&mut self, files: Vec<Stub>) -> &mut Self {
        files
            .iter()
            .map(|f| {
                let mut path = PathBuf::from(&self.cwd);

                let (file_name, contents) = match *f {
                    Stub::EmptyFile(name) => (name, "fake data"),
                    Stub::FileWithContent(name, content) => (name, content),
                };

                path.push(file_name);

                std::fs::write(PathBuf::from(path), contents.as_bytes())
                    .expect("can not create file");
            })
            .for_each(drop);
        self.back_to_playground();
        self
    }

    pub fn within(&mut self, directory: &str) -> &mut Self {
        self.cwd.push(directory);
        std::fs::create_dir(&self.cwd).expect("can not create directory");
        self
    }

    pub fn glob_vec(pattern: &str) -> Vec<PathBuf> {
        glob(pattern).unwrap().map(|r| r.unwrap()).collect()
    }
}

pub fn file_contents(full_path: &str) -> String {
    let mut file = std::fs::File::open(full_path).expect("can not open file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("can not read file");
    contents
}

pub fn normalize_string(input: &str) -> String {
    #[cfg(windows)]
    {
        input.to_string()
    }

    #[cfg(not(windows))]
    {
        format!("\"{}\"", input)
    }
}

pub fn create_file_at(full_path: &str) {
    std::fs::write(PathBuf::from(full_path), "fake data".as_bytes()).expect("can not create file");
}

pub fn copy_file_to(source: &str, destination: &str) {
    std::fs::copy(source, destination).expect("can not copy file");
}

pub fn files_exist_at(files: Vec<&Path>, path: PathBuf) -> bool {
    files.iter().all(|f| {
        let mut loc = path.clone();
        loc.push(f);
        loc.exists()
    })
}

pub fn file_exists_at(path: PathBuf) -> bool {
    path.exists()
}

pub fn dir_exists_at(path: PathBuf) -> bool {
    path.exists()
}

pub fn delete_directory_at(full_path: &str) {
    std::fs::remove_dir_all(PathBuf::from(full_path)).expect("can not remove directory");
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
