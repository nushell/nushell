use crate::fs::executable_path;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::Path;
use std::process::{Command, ExitStatus};

pub trait Executable {
    fn execute(&self) -> NuResult;
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

#[derive(Clone, Debug)]
pub struct NuProcess {
    pub arguments: Vec<OsString>,
    pub environment_vars: HashMap<String, Option<OsString>>,
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

impl Default for NuProcess {
    fn default() -> Self {
        Self {
            arguments: vec![],
            environment_vars: HashMap::default(),
            cwd: None,
        }
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
        let mut command = Command::new(&executable_path());

        if let Some(cwd) = self.get_cwd() {
            command.current_dir(cwd);
        }

        for arg in &self.arguments {
            command.arg(arg);
        }

        command
    }
}
