use super::EnvironmentVariable;
use crate::fs::{binaries as test_bins_path, executable_path};
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::Path;
use std::process::{Command, ExitStatus};

pub trait Executable {
    fn execute(&mut self) -> NuResult;
}

#[derive(Clone, Debug)]
pub struct Outcome {
    pub out: Vec<u8>,
    pub err: Vec<u8>,
}

impl Outcome {
    pub fn new(out: &[u8], err: &[u8]) -> Outcome {
        Outcome {
            out: out.to_vec(),
            err: err.to_vec(),
        }
    }
}

pub type NuResult = Result<Outcome, NuError>;

#[derive(Debug)]
pub struct NuError {
    pub desc: String,
    pub exit: Option<ExitStatus>,
    pub output: Option<Outcome>,
}

#[derive(Clone, Debug, Default)]
pub struct NuProcess {
    pub arguments: Vec<OsString>,
    pub environment_vars: Vec<EnvironmentVariable>,
    pub cwd: Option<OsString>,
}

impl fmt::Display for NuProcess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`nu")?;

        for arg in &self.arguments {
            write!(f, " {}", arg.to_string_lossy())?;
        }

        write!(f, "`")
    }
}

impl NuProcess {
    pub fn arg<T: AsRef<OsStr>>(&mut self, arg: T) -> &mut Self {
        self.arguments.push(arg.as_ref().to_os_string());
        self
    }

    pub fn args<T: AsRef<OsStr>>(&mut self, arguments: &[T]) -> &mut NuProcess {
        self.arguments
            .extend(arguments.iter().map(|t| t.as_ref().to_os_string()));
        self
    }

    pub fn cwd<T: AsRef<OsStr>>(&mut self, path: T) -> &mut NuProcess {
        self.cwd = Some(path.as_ref().to_os_string());
        self
    }

    pub fn get_cwd(&self) -> Option<&Path> {
        self.cwd.as_ref().map(Path::new)
    }

    pub fn construct(&self) -> Command {
        let mut command = Command::new(executable_path());

        if let Some(cwd) = self.get_cwd() {
            command.current_dir(cwd);
        }

        command.env_clear();

        let paths = vec![test_bins_path()];

        let paths_joined = match std::env::join_paths(paths) {
            Ok(all) => all,
            Err(_) => panic!("Couldn't join paths for PATH var."),
        };

        command.env(crate::NATIVE_PATH_ENV_VAR, paths_joined);

        for env_var in &self.environment_vars {
            command.env(&env_var.name, &env_var.value);
        }

        for arg in &self.arguments {
            command.arg(arg);
        }

        command
    }
}
