use super::Director;
use crate::fs;
use crate::fs::Stub;
use nu_glob::glob;
use std::path::{Path, PathBuf};
use std::str;
use tempfile::{tempdir, TempDir};

#[derive(Default, Clone, Debug)]
pub struct EnvironmentVariable {
    pub name: String,
    pub value: String,
}

impl EnvironmentVariable {
    fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.to_string(),
            value: value.to_string(),
        }
    }
}

pub struct Playground<'a> {
    root: TempDir,
    tests: String,
    cwd: PathBuf,
    config: Option<PathBuf>,
    environment_vars: Vec<EnvironmentVariable>,
    dirs: &'a Dirs,
}

#[derive(Default, Clone)]
pub struct Dirs {
    pub root: PathBuf,
    pub test: PathBuf,
    pub fixtures: PathBuf,
}

impl Dirs {
    pub fn formats(&self) -> PathBuf {
        self.fixtures.join("formats")
    }

    pub fn root(&self) -> &Path {
        self.root.as_path()
    }

    pub fn test(&self) -> &Path {
        self.test.as_path()
    }
}

impl<'a> Playground<'a> {
    pub fn root(&self) -> &Path {
        self.root.path()
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    pub fn back_to_playground(&mut self) -> &mut Self {
        self.cwd = PathBuf::from(self.root()).join(self.tests.clone());
        self
    }

    pub fn play(&mut self) -> &mut Self {
        self
    }

    pub fn setup(topic: &str, block: impl FnOnce(Dirs, &mut Playground)) {
        let root = tempdir().expect("Couldn't create a tempdir");
        let nuplay_dir = root.path().join(topic);

        if PathBuf::from(&nuplay_dir).exists() {
            std::fs::remove_dir_all(PathBuf::from(&nuplay_dir)).expect("can not remove directory");
        }

        std::fs::create_dir(PathBuf::from(&nuplay_dir)).expect("can not create directory");

        let fixtures = fs::fixtures();
        let cwd = std::env::current_dir().expect("Could not get current working directory.");
        let fixtures = nu_path::canonicalize_with(fixtures.clone(), cwd).unwrap_or_else(|e| {
            panic!(
                "Couldn't canonicalize fixtures path {}: {:?}",
                fixtures.display(),
                e
            )
        });

        let mut playground = Playground {
            root,
            tests: topic.to_string(),
            cwd: nuplay_dir,
            config: None,
            environment_vars: Vec::default(),
            dirs: &Dirs::default(),
        };

        let playground_root = playground.root.path();

        let cwd = std::env::current_dir().expect("Could not get current working directory.");
        let test =
            nu_path::canonicalize_with(playground_root.join(topic), cwd).unwrap_or_else(|e| {
                panic!(
                    "Couldn't canonicalize test path {}: {:?}",
                    playground_root.join(topic).display(),
                    e
                )
            });

        let cwd = std::env::current_dir().expect("Could not get current working directory.");
        let root = nu_path::canonicalize_with(playground_root, cwd).unwrap_or_else(|e| {
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

        playground.dirs = &dirs;

        block(dirs.clone(), &mut playground);
    }

    pub fn with_config(&mut self, source_file: impl AsRef<Path>) -> &mut Self {
        self.config = Some(source_file.as_ref().to_path_buf());
        self
    }

    pub fn with_env(&mut self, name: &str, value: &str) -> &mut Self {
        self.environment_vars
            .push(EnvironmentVariable::new(name, value));
        self
    }

    pub fn get_config(&self) -> Option<&str> {
        self.config
            .as_ref()
            .map(|cfg| cfg.to_str().expect("could not convert path."))
    }

    pub fn build(&mut self) -> Director {
        Director {
            cwd: Some(self.dirs.test().into()),
            config: self.config.clone().map(|cfg| cfg.into()),
            environment_vars: self.environment_vars.clone(),
            ..Default::default()
        }
    }

    pub fn cococo(&mut self, arg: &str) -> Director {
        self.build().cococo(arg)
    }

    pub fn pipeline(&mut self, commands: &str) -> Director {
        self.build().pipeline(commands)
    }

    pub fn mkdir(&mut self, directory: &str) -> &mut Self {
        self.cwd.push(directory);
        std::fs::create_dir_all(&self.cwd).expect("can not create directory");
        self.back_to_playground();
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn symlink(&mut self, from: impl AsRef<Path>, to: impl AsRef<Path>) -> &mut Self {
        let from = self.cwd.join(from);
        let to = self.cwd.join(to);

        let create_symlink = {
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink
            }

            #[cfg(windows)]
            {
                if from.is_file() {
                    std::os::windows::fs::symlink_file
                } else if from.is_dir() {
                    std::os::windows::fs::symlink_dir
                } else {
                    panic!("symlink from must be a file or dir")
                }
            }
        };

        create_symlink(from, to).expect("can not create symlink");
        self.back_to_playground();
        self
    }

    pub fn with_files(&mut self, files: Vec<Stub>) -> &mut Self {
        let endl = fs::line_ending();

        files
            .iter()
            .map(|f| {
                let mut path = PathBuf::from(&self.cwd);
                let mut permission_set = false;
                let mut write_able = true;
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
                    Stub::FileWithPermission(name, is_write_able) => {
                        permission_set = true;
                        write_able = is_write_able;
                        (name, "check permission".to_string())
                    }
                };

                path.push(file_name);

                std::fs::write(&path, contents.as_bytes()).expect("can not create file");
                if permission_set {
                    let err_perm = "can not set permission";
                    let mut perm = std::fs::metadata(path.clone())
                        .expect(err_perm)
                        .permissions();
                    perm.set_readonly(!write_able);
                    std::fs::set_permissions(path, perm).expect(err_perm);
                }
            })
            .for_each(drop);
        self.back_to_playground();
        self
    }

    pub fn within(&mut self, directory: &str) -> &mut Self {
        self.cwd.push(directory);
        if !(self.cwd.exists() && self.cwd.is_dir()) {
            std::fs::create_dir(&self.cwd).expect("can not create directory");
        }
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
