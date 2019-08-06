use crate::errors::ShellError;
use crate::parser::hir::SyntaxType;
use crate::parser::registry::{CommandConfig, NamedType, PositionalType};
use crate::prelude::*;
use indexmap::IndexMap;
use std::path::{Path, PathBuf};

pub struct Copycp;

impl Command for Copycp {
    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        cp(args)
    }

    fn name(&self) -> &str {
        "cp"
    }

    fn config(&self) -> CommandConfig {
        let mut named: IndexMap<String, NamedType> = IndexMap::new();
        named.insert("recursive".to_string(), NamedType::Switch);

        CommandConfig {
            name: self.name().to_string(),
            positional: vec![PositionalType::mandatory("file", SyntaxType::Path)],
            rest_positional: false,
            named,
            is_sink: false,
            is_filter: false,
        }
    }
}

pub fn cp(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let mut source = PathBuf::from(args.shell_manager.path());
    let mut destination = PathBuf::from(args.shell_manager.path());


    match args
        .nth(0)
        .ok_or_else(|| ShellError::string(&format!("No file or directory specified")))?
        .as_string()?
        .as_str()
    {
        file => {
            source.push(file);
        }
    }

    match args
        .nth(1)
        .ok_or_else(|| ShellError::string(&format!("No file or directory specified")))?
        .as_string()?
        .as_str()
    {
        file => {
            destination.push(file);
        }
    }

    let (sources, destinations) = (
        glob::glob(&source.to_string_lossy()),
        glob::glob(&destination.to_string_lossy()),
    );

    if sources.is_err() || destinations.is_err() {
        return Err(ShellError::string("Invalid pattern."));
    }

    let (sources, destinations): (Vec<_>, Vec<_>) =
        (sources.unwrap().collect(), destinations.unwrap().collect());

    if sources.len() == 1 {
        if let Ok(entry) = &sources[0] {
            if entry.is_file() {
                if destinations.len() == 1 {
                    if let Ok(dst) = &destinations[0] {
                        if dst.is_file() {
                            std::fs::copy(entry, dst);
                        }

                        if dst.is_dir() {
                            destination.push(entry.file_name().unwrap());
                            std::fs::copy(entry, destination);
                        }
                    }
                } else if destinations.is_empty() {
                    if destination.is_dir() {
                        destination.push(entry.file_name().unwrap());
                        std::fs::copy(entry, destination);
                    } else {
                        std::fs::copy(entry, destination);
                    }
                }
            }

            if entry.is_dir() {
                if destinations.len() == 1 {
                    if let Ok(dst) = &destinations[0] {
                        if dst.is_dir() && !args.has("recursive") {
                            return Err(ShellError::string(&format!(
                                "{:?} is a directory (not copied)",
                                entry.to_string_lossy()
                            )));
                        }


                        if dst.is_dir() && args.has("recursive") {
                            let entries = std::fs::read_dir(&entry);

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
                                            args.call_info.name_span,
                                        ));
                                    }
                                }
                                Ok(o) => o,
                            };

                            let mut x = dst.clone();

                            //x.pop();
                            x.push(entry.file_name().unwrap());


                            std::fs::create_dir(&x).expect("can not create directory");

                            for entry in entries {
                                let entry = entry?;
                                let file_path = entry.path();
                                let file_name = file_path.file_name().unwrap();

                                let mut d = PathBuf::new();
                                d.push(&x);
                                d.push(file_name);

                                std::fs::copy(entry.path(), d);
                            }
                        }
                    }
                }
            }
        }
    }
    /*
    if destination.is_dir() {
        if source.is_file() {
            let file_name = source.file_name().expect("");
            let file_name = file_name.to_str().expect("");
            destination.push(Path::new(file_name));

            match std::fs::copy(source, destination) {
                Err(_error) => return Err(ShellError::string("can not copy file")),
                Ok(_) => return Ok(OutputStream::empty()),
            }
        } else if source.is_dir() {

                return Err(ShellError::string(&format!(
                    "{:?} is a directory (not copied)",
                    source.to_string_lossy()
                )));

        }
    }

    match std::fs::copy(source, destination) {
        Err(_error) => Err(ShellError::string("can not copy file")),
        Ok(_) => Ok(OutputStream::empty()),
    }*/
    Ok(OutputStream::empty())
}
