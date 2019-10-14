#![allow(dead_code)]

use glob::glob;
pub use std::path::Path;
pub use std::path::PathBuf;

use app_dirs::{get_app_root, AppDataType};
use getset::Getters;
use std::io::Read;
use tempfile::{tempdir, TempDir};

pub trait DisplayPath {
    fn display_path(&self) -> String;
}

impl DisplayPath for PathBuf {
    fn display_path(&self) -> String {
        self.display().to_string()
    }
}

impl DisplayPath for str {
    fn display_path(&self) -> String {
        self.to_string()
    }
}

impl DisplayPath for &str {
    fn display_path(&self) -> String {
        self.to_string()
    }
}

impl DisplayPath for String {
    fn display_path(&self) -> String {
        self.clone()
    }
}

impl DisplayPath for &String {
    fn display_path(&self) -> String {
        self.to_string()
    }
}

impl DisplayPath for nu::AbsolutePath {
    fn display_path(&self) -> String {
        self.to_string()
    }
}

#[macro_export]
macro_rules! nu {
    (cwd: $cwd:expr, $path:expr, $($part:expr),*) => {{
        use $crate::helpers::DisplayPath;

        let path = format!($path, $(
            $part.display_path()
        ),*);

        nu!($cwd, &path)
    }};

    (cwd: $cwd:expr, $path:expr) => {{
        nu!($cwd, $path)
    }};

    ($cwd:expr, $path:expr) => {{
        pub use std::error::Error;
        pub use std::io::prelude::*;
        pub use std::process::{Command, Stdio};

        let commands = &*format!(
            "
                            cd {}
                            {}
                            exit",
            $crate::helpers::in_directory($cwd),
            $crate::helpers::DisplayPath::display_path(&$path)
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

        let out = String::from_utf8_lossy(&output.stdout);
        let out = out.replace("\r\n", "");
        let out = out.replace("\n", "");
        out
    }};
}

#[macro_export]
macro_rules! nu_error {
    (cwd: $cwd:expr, $path:expr, $($part:expr),*) => {{
        use $crate::helpers::DisplayPath;

        let path = format!($path, $(
            $part.display_path()
        ),*);

        nu_error!($cwd, &path)
    }};

    (cwd: $cwd:expr, $path:expr) => {{
        nu_error!($cwd, $path)
    }};

    ($cwd:expr, $path:expr) => {{
        pub use std::error::Error;
        pub use std::io::prelude::*;
        pub use std::process::{Command, Stdio};

        let commands = &*format!(
            "
                            cd {}
                            {}
                            exit",
            $crate::helpers::in_directory($cwd),
            $crate::helpers::DisplayPath::display_path(&$path)
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

        let out = String::from_utf8_lossy(&output.stderr);
        out.into_owned()
    }};
}

pub enum Stub<'a> {
    FileWithContent(&'a str, &'a str),
    FileWithContentToBeTrimmed(&'a str, &'a str),
    EmptyFile(&'a str),
}

pub struct Playground {
    root: TempDir,
    tests: String,
    cwd: PathBuf,
}

#[derive(Getters)]
#[get = "pub"]
pub struct Dirs {
    pub root: PathBuf,
    pub test: PathBuf,
    pub fixtures: PathBuf,
}

impl Dirs {
    pub fn formats(&self) -> PathBuf {
        PathBuf::from(self.fixtures.join("formats"))
    }

    pub fn config_path(&self) -> PathBuf {
        get_app_root(AppDataType::UserConfig, &nu::APP_INFO).unwrap()
    }
}

impl Playground {
    pub fn root(&self) -> &Path {
        self.root.path()
    }

    pub fn back_to_playground(&mut self) -> &mut Self {
        self.cwd = PathBuf::from(self.root()).join(self.tests.clone());
        self
    }

    pub fn setup(topic: &str, block: impl FnOnce(Dirs, &mut Playground)) {
        let root = tempdir().expect("Couldn't create a tempdir");
        let nuplay_dir = root.path().join(topic);

        if PathBuf::from(&nuplay_dir).exists() {
            std::fs::remove_dir_all(PathBuf::from(&nuplay_dir)).expect("can not remove directory");
        }

        std::fs::create_dir(PathBuf::from(&nuplay_dir)).expect("can not create directory");

        let mut playground = Playground {
            root: root,
            tests: topic.to_string(),
            cwd: nuplay_dir,
        };

        let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let playground_root = playground.root.path();

        let fixtures = project_root.join(file!());
        let fixtures = fixtures
            .parent()
            .expect("Couldn't find the fixtures directory")
            .parent()
            .expect("Couldn't find the fixtures directory")
            .join("fixtures");

        let fixtures = dunce::canonicalize(fixtures.clone()).expect(&format!(
            "Couldn't canonicalize fixtures path {}",
            fixtures.display()
        ));

        let test =
            dunce::canonicalize(PathBuf::from(playground_root.join(topic))).expect(&format!(
                "Couldn't canonicalize test path {}",
                playground_root.join(topic).display()
            ));

        let root = dunce::canonicalize(playground_root).expect(&format!(
            "Couldn't canonicalize tests root path {}",
            playground_root.display()
        ));

        let dirs = Dirs {
            root,
            test,
            fixtures,
        };

        block(dirs, &mut playground);
    }

    pub fn mkdir(&mut self, directory: &str) -> &mut Self {
        self.cwd.push(directory);
        std::fs::create_dir_all(&self.cwd).expect("can not create directory");
        self.back_to_playground();
        self
    }

    pub fn with_files(&mut self, files: Vec<Stub>) -> &mut Self {
        let endl = line_ending();

        files
            .iter()
            .map(|f| {
                let mut path = PathBuf::from(&self.cwd);

                let (file_name, contents) = match *f {
                    Stub::EmptyFile(name) => (name, "fake data".to_string()),
                    Stub::FileWithContent(name, content) => (name, content.to_string()),
                    Stub::FileWithContentToBeTrimmed(name, content) => (
                        name,
                        content
                            .lines()
                            .skip(1)
                            .map(|line| line.trim())
                            .collect::<Vec<&str>>()
                            .join(&endl),
                    ),
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
        let glob = glob(pattern);

        match glob {
            Ok(paths) => paths
                .map(|path| {
                    if let Ok(path) = path {
                        path
                    } else {
                        unreachable!()
                    }
                })
                .collect(),
            Err(_) => panic!("Invalid pattern."),
        }
    }
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

    if let Some(parent) = full_path.parent() {
        panic!(format!("{:?} exists", parent.display()));
    }

    std::fs::write(full_path, "fake data".as_bytes())
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
    let mut buf = PathBuf::new();
    buf.push("target");
    buf.push("debug");
    buf.push("nu");
    buf
}

pub fn in_directory(str: impl AsRef<Path>) -> String {
    str.as_ref().display().to_string()
}

pub fn pipeline(commands: &str) -> String {
    commands
        .lines()
        .skip(1)
        .map(|line| line.trim())
        .collect::<Vec<&str>>()
        .join(" ")
        .trim_end()
        .to_string()
}
