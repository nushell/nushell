use nu_errors::ShellError;
use nu_protocol::{CallInfo, Value};
use nu_source::{Tag, Tagged, TaggedItem};
use std::path::Path;

#[cfg(not(target_os = "windows"))]
use std::process::{Command, Stdio};

#[derive(Default)]
pub struct Start {
    pub tag: Tag,
    pub filenames: Vec<Tagged<String>>,
    pub application: Option<String>,
}

impl Start {
    pub fn new() -> Start {
        Start {
            tag: Tag::unknown(),
            filenames: vec![],
            application: None,
        }
    }

    pub fn parse(&mut self, call_info: CallInfo) -> Result<(), ShellError> {
        self.tag = call_info.name_tag.clone();
        self.parse_filenames(&call_info)?;
        self.parse_application(&call_info);
        Ok(())
    }

    fn add_filename(&mut self, filename: Tagged<String>) -> Result<(), ShellError> {
        if Path::new(&filename.item).exists() || url::Url::parse(&filename.item).is_ok() {
            self.filenames.push(filename);
            Ok(())
        } else {
            Err(ShellError::labeled_error(
                format!("The file '{}' does not exist", filename.item),
                "doesn't exist",
                filename.tag,
            ))
        }
    }

    fn glob_to_values(&self, value: &Value) -> Result<Vec<Tagged<String>>, ShellError> {
        let mut result = vec![];
        match glob::glob(&value.as_string()?) {
            Ok(paths) => {
                for path_result in paths {
                    match path_result {
                        Ok(path) => result
                            .push(path.to_string_lossy().to_string().tagged(value.tag.clone())),
                        Err(glob_error) => {
                            return Err(ShellError::labeled_error(
                                format!("{}", glob_error),
                                "glob error",
                                value.tag.clone(),
                            ));
                        }
                    }
                }
            }
            Err(pattern_error) => {
                return Err(ShellError::labeled_error(
                    format!("{}", pattern_error),
                    "invalid pattern",
                    value.tag.clone(),
                ))
            }
        }

        Ok(result)
    }

    fn parse_filenames(&mut self, call_info: &CallInfo) -> Result<(), ShellError> {
        let candidates = match &call_info.args.positional {
            Some(values) => {
                let mut result = vec![];

                for value in values.iter() {
                    let res = self.glob_to_values(value)?;
                    result.extend(res);
                }

                if result.is_empty() {
                    return Err(ShellError::labeled_error(
                        "No filename(s) given",
                        "no filename(s) given",
                        self.tag.span,
                    ));
                }
                result
            }
            None => {
                return Err(ShellError::labeled_error(
                    "No filename(s) given",
                    "no filename(s) given",
                    self.tag.span,
                ))
            }
        };

        for candidate in candidates {
            self.add_filename(candidate)?;
        }

        Ok(())
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

    #[cfg(target_os = "macos")]
    pub fn exec(&mut self) -> Result<(), ShellError> {
        let mut args = vec![];
        args.append(
            &mut self
                .filenames
                .iter()
                .map(|x| x.item.clone())
                .collect::<Vec<_>>(),
        );

        if let Some(app_name) = &self.application {
            args.append(&mut vec![String::from("-a"), app_name.to_string()]);
        }
        exec_cmd("open", &args, self.tag.clone())
    }

    #[cfg(target_os = "windows")]
    pub fn exec(&mut self) -> Result<(), ShellError> {
        if let Some(app_name) = &self.application {
            for file in &self.filenames {
                match open::with(&file.item, app_name) {
                    Ok(_) => continue,
                    Err(_) => {
                        return Err(ShellError::labeled_error(
                            "Failed to open file with specified application",
                            "can't open with specified application",
                            file.tag.span,
                        ))
                    }
                }
            }
        } else {
            for file in &self.filenames {
                match open::that(&file.item) {
                    Ok(_) => continue,
                    Err(_) => {
                        return Err(ShellError::labeled_error(
                            "Failed to open file with default application",
                            "can't open with default application",
                            file.tag.span,
                        ))
                    }
                }
            }
        }
        Ok(())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    pub fn exec(&mut self) -> Result<(), ShellError> {
        let mut args = vec![];
        args.append(
            &mut self
                .filenames
                .iter()
                .map(|x| x.item.clone())
                .collect::<Vec<_>>(),
        );

        if let Some(app_name) = &self.application {
            exec_cmd(&app_name, &args, self.tag.clone())
        } else {
            for cmd in &["xdg-open", "gnome-open", "kde-open", "wslview"] {
                if exec_cmd(cmd, &args, self.tag.clone()).is_err() {
                    continue;
                } else {
                    return Ok(());
                }
            }
            Err(ShellError::labeled_error(
                "Failed to open file(s) with xdg-open. gnome-open, kde-open, and wslview",
                "failed to open xdg-open. gnome-open, kde-open, and wslview",
                self.tag.span,
            ))
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn exec_cmd(cmd: &str, args: &[String], tag: Tag) -> Result<(), ShellError> {
    if args.is_empty() {
        return Err(ShellError::labeled_error(
            "No file(s) or application provided",
            "no file(s) or application provided",
            tag,
        ));
    }
    let status = match Command::new(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .args(args)
        .status()
    {
        Ok(exit_code) => exit_code,
        Err(_) => {
            return Err(ShellError::labeled_error(
                "Failed to run native open syscall",
                "failed to run native open call",
                tag,
            ))
        }
    };
    if status.success() {
        Ok(())
    } else {
        Err(ShellError::labeled_error(
            "Failed to run start. Hint: The file(s)/application may not exist",
            "failed to run",
            tag,
        ))
    }
}
