use ansi_term::Color;
use nu_protocol::{CallInfo, Value};
use std::error::Error;
use std::fmt;
use std::path::Path;
use std::process::{Command, Stdio};

pub struct Start {
    pub filenames: Vec<String>,
    pub application: Option<String>,
}

#[derive(Debug)]
pub struct StartError {
    msg: String,
}

impl StartError {
    fn new(msg: &str) -> StartError {
        StartError {
            msg: msg.to_owned(),
        }
    }
}

impl fmt::Display for StartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {}",
            Color::Red.bold().paint("start error"),
            self.msg
        )
    }
}

impl Error for StartError {}

impl Start {
    pub fn parse(&mut self, call_info: CallInfo, input: Vec<Value>) {
        input.iter().for_each(|val| {
            if val.is_some() {
                self.parse_value(val);
            }
        });
        self.parse_filenames(&call_info);
        self.parse_application(&call_info);
    }

    fn add_filename(&mut self, filename: String) {
        if Path::new(&filename).exists() || url::Url::parse(&filename).is_ok() {
            self.filenames.push(filename);
        } else {
            print_warning(format!(
                "The file '{}' does not exist",
                Color::White.bold().paint(filename)
            ));
        }
    }

    fn parse_filenames(&mut self, call_info: &CallInfo) {
        let candidates = match &call_info.args.positional {
            Some(values) => values
                .iter()
                .map(|val| val.as_string())
                .collect::<Result<Vec<String>, _>>()
                .unwrap_or_else(|_| vec![]),
            None => vec![],
        };

        for candidate in candidates {
            self.add_filename(candidate);
        }
    }

    fn parse_application(&mut self, call_info: &CallInfo) {
        self.application = if let Some(app) = call_info.args.get("application") {
            match app.as_string() {
                Ok(name) => Some(name),
                Err(_) => None,
            }
        } else {
            None
        };
    }

    pub fn parse_value(&mut self, input: &Value) {
        if let Ok(filename) = input.as_string() {
            self.add_filename(filename);
        } else {
            print_warning(format!("Could not convert '{:?}' to string", input));
        }
    }

    #[cfg(target_os = "macos")]
    pub fn exec(&mut self) -> Result<(), StartError> {
        let mut args = vec![];
        args.append(&mut self.filenames);

        if let Some(app_name) = &self.application {
            args.append(&mut vec![String::from("-a"), app_name.to_string()]);
        }
        exec_cmd("open", &args)
    }

    #[cfg(target_os = "windows")]
    pub fn exec(&mut self) -> Result<(), StartError> {
        if let Some(app_name) = &self.application {
            for file in &self.filenames {
                match open::with(file, app_name) {
                    Ok(_) => continue,
                    Err(_) => {
                        return Err(StartError::new(
                            "Failed to open file with specified application",
                        ))
                    }
                }
            }
        } else {
            for file in &self.filenames {
                match open::that(file) {
                    Ok(_) => continue,
                    Err(_) => {
                        return Err(StartError::new(
                            "Failed to open file with default application",
                        ))
                    }
                }
            }
        }
        Ok(())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    pub fn exec(&mut self) -> Result<(), StartError> {
        let mut args = vec![];
        args.append(&mut self.filenames);

        if let Some(app_name) = &self.application {
            exec_cmd(&app_name, &args)
        } else {
            for cmd in &["xdg-open", "gnome-open", "kde-open", "wslview"] {
                if let Err(_) = exec_cmd(cmd, &args) {
                    continue;
                }
            }
            Err(StartError::new(
                "Failed to open file(s) with xdg-open. gnome-open, kde-open, and wslview",
            ))
        }
    }
}

fn print_warning(msg: String) {
    println!("{}: {}", Color::Yellow.bold().paint("warning"), msg);
}

fn exec_cmd(cmd: &str, args: &[String]) -> Result<(), StartError> {
    if args.is_empty() {
        return Err(StartError::new("No file(s) or application provided"));
    }
    let status = match Command::new(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .args(args)
        .status()
    {
        Ok(exit_code) => exit_code,
        Err(_) => return Err(StartError::new("Failed to run native open syscall")),
    };
    if status.success() {
        Ok(())
    } else {
        Err(StartError::new(
            "Failed to run start. Hint: The file(s)/application may not exist",
        ))
    }
}
