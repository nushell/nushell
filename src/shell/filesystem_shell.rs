use crate::commands::command::CallInfo;
use crate::object::dir_entry_dict;
use crate::prelude::*;
use crate::shell::completer::NuCompleter;
use crate::shell::shell::Shell;
use rustyline::completion::{self, Completer, FilenameCompleter};
use rustyline::error::ReadlineError;
use rustyline::hint::{Hinter, HistoryHinter};
use std::path::{Path, PathBuf};
pub struct FilesystemShell {
    crate path: PathBuf,
    completer: NuCompleter,
    hinter: HistoryHinter,
}

impl Clone for FilesystemShell {
    fn clone(&self) -> Self {
        FilesystemShell {
            path: self.path.clone(),
            completer: NuCompleter {
                file_completer: FilenameCompleter::new(),
            },
            hinter: HistoryHinter {},
        }
    }
}

impl FilesystemShell {
    pub fn basic() -> Result<FilesystemShell, std::io::Error> {
        let path = std::env::current_dir()?;

        Ok(FilesystemShell {
            path,
            completer: NuCompleter {
                file_completer: FilenameCompleter::new(),
            },
            hinter: HistoryHinter {},
        })
    }

    pub fn with_location(location: String) -> Result<FilesystemShell, std::io::Error> {
        let path = std::path::PathBuf::from(location);

        Ok(FilesystemShell {
            path,
            completer: NuCompleter {
                file_completer: FilenameCompleter::new(),
            },
            hinter: HistoryHinter {},
        })
    }
}

impl Shell for FilesystemShell {
    fn ls(&self, call_info: CallInfo, _input: InputStream) -> Result<OutputStream, ShellError> {
        let cwd = self.path.clone();
        let mut full_path = PathBuf::from(&self.path);
        match &call_info.args.nth(0) {
            Some(Tagged { item: value, .. }) => full_path.push(Path::new(&value.as_string()?)),
            _ => {}
        }
        let entries = glob::glob(&full_path.to_string_lossy());

        if entries.is_err() {
            return Err(ShellError::string("Invalid pattern."));
        }

        let mut shell_entries = VecDeque::new();
        let entries: Vec<_> = entries.unwrap().collect();

        // If this is a single entry, try to display the contents of the entry if it's a directory
        if entries.len() == 1 {
            if let Ok(entry) = &entries[0] {
                if entry.is_dir() {
                    let entries = std::fs::read_dir(&full_path);

                    let entries = match entries {
                        Err(e) => {
                            if let Some(s) = call_info.args.nth(0) {
                                return Err(ShellError::labeled_error(
                                    e.to_string(),
                                    e.to_string(),
                                    s.span(),
                                ));
                            } else {
                                return Err(ShellError::labeled_error(
                                    e.to_string(),
                                    e.to_string(),
                                    call_info.name_span,
                                ));
                            }
                        }
                        Ok(o) => o,
                    };
                    for entry in entries {
                        let entry = entry?;
                        let filepath = entry.path();
                        let filename = filepath.strip_prefix(&cwd).unwrap();
                        let value = dir_entry_dict(
                            filename,
                            &entry.metadata()?,
                            Tag::unknown_origin(call_info.name_span),
                        )?;
                        shell_entries.push_back(ReturnSuccess::value(value))
                    }
                    return Ok(shell_entries.to_output_stream());
                }
            }
        }

        // Enumerate the entries from the glob and add each
        for entry in entries {
            if let Ok(entry) = entry {
                let filename = entry.strip_prefix(&cwd).unwrap();
                let metadata = std::fs::metadata(&entry)?;
                let value = dir_entry_dict(
                    filename,
                    &metadata,
                    Tag::unknown_origin(call_info.name_span),
                )?;
                shell_entries.push_back(ReturnSuccess::value(value))
            }
        }

        Ok(shell_entries.to_output_stream())
    }

    fn cd(&self, call_info: CallInfo, _input: InputStream) -> Result<OutputStream, ShellError> {
        let path = match call_info.args.nth(0) {
            None => match dirs::home_dir() {
                Some(o) => o,
                _ => {
                    return Err(ShellError::labeled_error(
                        "Can not change to home directory",
                        "can not go to home",
                        call_info.name_span,
                    ))
                }
            },
            Some(v) => {
                let target = v.as_string()?;
                match dunce::canonicalize(self.path.join(target).as_path()) {
                    Ok(p) => p,
                    Err(_) => {
                        return Err(ShellError::labeled_error(
                            "Can not change to directory",
                            "directory not found",
                            v.span().clone(),
                        ));
                    }
                }
            }
        };

        let mut stream = VecDeque::new();
        match std::env::set_current_dir(&path) {
            Ok(_) => {}
            Err(_) => {
                if call_info.args.len() > 0 {
                    return Err(ShellError::labeled_error(
                        "Can not change to directory",
                        "directory not found",
                        call_info.args.nth(0).unwrap().span().clone(),
                    ));
                } else {
                    return Err(ShellError::string("Can not change to directory"));
                }
            }
        }
        stream.push_back(ReturnSuccess::change_cwd(path));
        Ok(stream.into())
    }

    fn path(&self) -> std::path::PathBuf {
        self.path.clone()
    }

    fn set_path(&mut self, path: &std::path::PathBuf) {
        self.path = path.clone();
    }
}

impl Completer for FilesystemShell {
    type Candidate = completion::Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<completion::Pair>), ReadlineError> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for FilesystemShell {
    fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}
