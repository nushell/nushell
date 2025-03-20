use super::Director;
use crate::fs::{self, Stub};
use nu_glob::{glob, Uninterruptible};
#[cfg(not(target_arch = "wasm32"))]
use nu_path::Path;
use nu_path::{AbsolutePath, AbsolutePathBuf};
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
    _root: TempDir,
    tests: String,
    cwd: AbsolutePathBuf,
    config: Option<AbsolutePathBuf>,
    environment_vars: Vec<EnvironmentVariable>,
    dirs: &'a Dirs,
}

#[derive(Clone)]
pub struct Dirs {
    pub root: AbsolutePathBuf,
    pub test: AbsolutePathBuf,
    pub fixtures: AbsolutePathBuf,
}

impl Dirs {
    pub fn formats(&self) -> AbsolutePathBuf {
        self.fixtures.join("formats")
    }

    pub fn root(&self) -> &AbsolutePath {
        &self.root
    }

    pub fn test(&self) -> &AbsolutePath {
        &self.test
    }
}

impl Playground<'_> {
    pub fn root(&self) -> &AbsolutePath {
        &self.dirs.root
    }

    pub fn cwd(&self) -> &AbsolutePath {
        &self.cwd
    }

    pub fn back_to_playground(&mut self) -> &mut Self {
        self.cwd = self.root().join(&self.tests);
        self
    }

    pub fn play(&mut self) -> &mut Self {
        self
    }

    pub fn setup(topic: &str, block: impl FnOnce(Dirs, &mut Playground)) {
        let temp = tempdir().expect("Could not create a tempdir");

        let root = AbsolutePathBuf::try_from(temp.path())
            .expect("Tempdir is not an absolute path")
            .canonicalize()
            .expect("Could not canonicalize tempdir");

        let test = root.join(topic);
        if test.exists() {
            std::fs::remove_dir_all(&test).expect("Could not remove directory");
        }
        std::fs::create_dir(&test).expect("Could not create directory");
        let test = test
            .canonicalize()
            .expect("Could not canonicalize test path");

        let fixtures = fs::fixtures()
            .canonicalize()
            .expect("Could not canonicalize fixtures path");

        let dirs = Dirs {
            root: root.into(),
            test: test.as_path().into(),
            fixtures: fixtures.into(),
        };

        let mut playground = Playground {
            _root: temp,
            tests: topic.to_string(),
            cwd: test.into(),
            config: None,
            environment_vars: Vec::default(),
            dirs: &dirs,
        };

        block(dirs.clone(), &mut playground);
    }

    pub fn with_config(&mut self, source_file: AbsolutePathBuf) -> &mut Self {
        self.config = Some(source_file);
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

    pub fn with_files(&mut self, files: &[Stub]) -> &mut Self {
        let endl = fs::line_ending();

        files
            .iter()
            .map(|f| {
                let mut permission_set = false;
                let mut write_able = true;
                let (file_name, contents) = match *f {
                    Stub::EmptyFile(name) => (name, String::new()),
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

                let path = self.cwd.join(file_name);

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

    pub fn glob_vec(pattern: &str) -> Vec<std::path::PathBuf> {
        let glob = glob(pattern, Uninterruptible);

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
