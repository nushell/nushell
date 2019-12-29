use crate::fs::line_ending;
use crate::fs::Stub;

use getset::Getters;
use glob::glob;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

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
        self.fixtures.join("formats")
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
            root,
            tests: topic.to_string(),
            cwd: nuplay_dir,
        };

        let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let playground_root = playground.root.path();

        let fixtures = project_root;
        let fixtures = fixtures
            .parent()
            .expect("Couldn't find the fixtures directory")
            .parent()
            .expect("Couldn't find the fixtures directory")
            .join("tests/fixtures");

        let fixtures = dunce::canonicalize(fixtures.clone()).unwrap_or_else(|e| {
            panic!(
                "Couldn't canonicalize fixtures path {}: {:?}",
                fixtures.display(),
                e
            )
        });

        let test = dunce::canonicalize(playground_root.join(topic)).unwrap_or_else(|e| {
            panic!(
                "Couldn't canonicalize test path {}: {:?}",
                playground_root.join(topic).display(),
                e
            )
        });

        let root = dunce::canonicalize(playground_root).unwrap_or_else(|e| {
            panic!(
                "Couldn't canonicalize tests root path {}: {:?}",
                playground_root.display(),
                e
            )
        });

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

                std::fs::write(path, contents.as_bytes()).expect("can not create file");
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

        glob.expect("invalid pattern")
            .map(|path| {
                if let Ok(path) = path {
                    path
                } else {
                    unreachable!()
                }
            })
            .collect()
    }
}
