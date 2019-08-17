use crate::commands::command::EvaluatedWholeStreamCommandArgs;
use crate::context::SourceMap;
use crate::object::dir_entry_dict;
use crate::prelude::*;
use crate::shell::completer::NuCompleter;
use crate::shell::shell::Shell;
use rustyline::completion::FilenameCompleter;
use rustyline::hint::{Hinter, HistoryHinter};
use std::path::{Path, PathBuf};

pub struct FilesystemShell {
    crate path: String,
    completer: NuCompleter,
    hinter: HistoryHinter,
}

impl std::fmt::Debug for FilesystemShell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FilesystemShell @ {}", self.path)
    }
}

impl Clone for FilesystemShell {
    fn clone(&self) -> Self {
        FilesystemShell {
            path: self.path.clone(),
            completer: NuCompleter {
                file_completer: FilenameCompleter::new(),
                commands: self.completer.commands.clone(),
            },
            hinter: HistoryHinter {},
        }
    }
}

impl FilesystemShell {
    pub fn basic(commands: CommandRegistry) -> Result<FilesystemShell, std::io::Error> {
        let path = std::env::current_dir()?;

        Ok(FilesystemShell {
            path: path.to_string_lossy().to_string(),
            completer: NuCompleter {
                file_completer: FilenameCompleter::new(),
                commands,
            },
            hinter: HistoryHinter {},
        })
    }

    pub fn with_location(
        path: String,
        commands: CommandRegistry,
    ) -> Result<FilesystemShell, std::io::Error> {
        Ok(FilesystemShell {
            path,
            completer: NuCompleter {
                file_completer: FilenameCompleter::new(),
                commands,
            },
            hinter: HistoryHinter {},
        })
    }
}

impl Shell for FilesystemShell {
    fn name(&self, _source_map: &SourceMap) -> String {
        "filesystem".to_string()
    }

    fn ls(&self, args: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError> {
        let cwd = self.path.clone();
        let mut full_path = PathBuf::from(&self.path);
        match &args.nth(0) {
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
                            if let Some(s) = args.nth(0) {
                                return Err(ShellError::labeled_error(
                                    e.to_string(),
                                    e.to_string(),
                                    s.span(),
                                ));
                            } else {
                                return Err(ShellError::labeled_error(
                                    e.to_string(),
                                    e.to_string(),
                                    args.name_span(),
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
                            Tag::unknown_origin(args.call_info.name_span),
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
                    Tag::unknown_origin(args.call_info.name_span),
                )?;
                shell_entries.push_back(ReturnSuccess::value(value))
            }
        }

        Ok(shell_entries.to_output_stream())
    }

    fn cd(&self, args: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError> {
        let path = match args.nth(0) {
            None => match dirs::home_dir() {
                Some(o) => o,
                _ => {
                    return Err(ShellError::labeled_error(
                        "Can not change to home directory",
                        "can not go to home",
                        args.call_info.name_span,
                    ))
                }
            },
            Some(v) => {
                let target = v.as_string()?;
                let path = PathBuf::from(self.path());
                match dunce::canonicalize(path.join(target).as_path()) {
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
                if args.len() > 0 {
                    return Err(ShellError::labeled_error(
                        "Can not change to directory",
                        "directory not found",
                        args.nth(0).unwrap().span().clone(),
                    ));
                } else {
                    return Err(ShellError::string("Can not change to directory"));
                }
            }
        }
        stream.push_back(ReturnSuccess::change_cwd(
            path.to_string_lossy().to_string(),
        ));
        Ok(stream.into())
    }

    fn path(&self) -> String {
        self.path.clone()
    }

    fn set_path(&mut self, path: String) {
        let pathbuf = PathBuf::from(&path);
        let path = match dunce::canonicalize(pathbuf.as_path()) {
            Ok(path) => {
                let _ = std::env::set_current_dir(&path);
                path
            }
            _ => {
                // TODO: handle the case where the path cannot be canonicalized
                pathbuf
            }
        };
        self.path = path.to_string_lossy().to_string();
    }

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<rustyline::completion::Pair>), rustyline::error::ReadlineError> {
        self.completer.complete(line, pos, ctx)
    }

    fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}
