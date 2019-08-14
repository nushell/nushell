use crate::errors::ShellError;
use crate::parser::hir::SyntaxType;
use crate::parser::registry::{CommandRegistry, Signature};
use crate::prelude::*;
use crate::utils::FileStructure;
use std::path::PathBuf;

pub struct Cpy;

impl StaticCommand for Cpy {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        cp(args, registry)
    }

    fn name(&self) -> &str {
        "cp"
    }

    fn signature(&self) -> Signature {
        Signature::build("cp")
            .named("file", SyntaxType::Any)
            .switch("recursive")
    }
}

pub fn cp(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let mut source = PathBuf::from(args.shell_manager.path());
    let mut destination = PathBuf::from(args.shell_manager.path());
    let name_span = args.call_info.name_span;
    let args = args.evaluate_once(registry)?;

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

    let sources = glob::glob(&source.to_string_lossy());

    if sources.is_err() {
        return Err(ShellError::labeled_error(
            "Invalid pattern.",
            "Invalid pattern.",
            args.nth(0).unwrap().span(),
        ));
    }

    let sources: Vec<_> = sources.unwrap().collect();

    if sources.len() == 1 {
        if let Ok(entry) = &sources[0] {
            if entry.is_dir() && !args.has("recursive") {
                return Err(ShellError::labeled_error(
                    "is a directory (not copied). Try using \"--recursive\".",
                    "is a directory (not copied). Try using \"--recursive\".",
                    args.nth(0).unwrap().span(),
                ));
            }

            let mut sources: FileStructure = FileStructure::new();

            sources.walk_decorate(&entry);

            if entry.is_file() {
                let strategy = |(source_file, _depth_level)| {
                    if destination.exists() {
                        let mut new_dst = dunce::canonicalize(destination.clone()).unwrap();
                        new_dst.push(entry.file_name().unwrap());
                        (source_file, new_dst)
                    } else {
                        (source_file, destination.clone())
                    }
                };

                for (ref src, ref dst) in sources.paths_applying_with(strategy) {
                    if src.is_file() {
                        match std::fs::copy(src, dst) {
                            Err(e) => {
                                return Err(ShellError::labeled_error(
                                    e.to_string(),
                                    e.to_string(),
                                    name_span,
                                ));
                            }
                            Ok(o) => o,
                        };
                    }
                }
            }

            if entry.is_dir() {
                if !destination.exists() {
                    match std::fs::create_dir_all(&destination) {
                        Err(e) => {
                            return Err(ShellError::labeled_error(
                                e.to_string(),
                                e.to_string(),
                                name_span,
                            ));
                        }
                        Ok(o) => o,
                    };

                    let strategy = |(source_file, depth_level)| {
                        let mut new_dst = destination.clone();
                        let path = dunce::canonicalize(&source_file).unwrap();

                        let mut comps: Vec<_> = path
                            .components()
                            .map(|fragment| fragment.as_os_str())
                            .rev()
                            .take(1 + depth_level)
                            .collect();

                        comps.reverse();

                        for fragment in comps.iter() {
                            new_dst.push(fragment);
                        }

                        (PathBuf::from(&source_file), PathBuf::from(new_dst))
                    };

                    for (ref src, ref dst) in sources.paths_applying_with(strategy) {
                        if src.is_dir() {
                            if !dst.exists() {
                                match std::fs::create_dir_all(dst) {
                                    Err(e) => {
                                        return Err(ShellError::labeled_error(
                                            e.to_string(),
                                            e.to_string(),
                                            name_span,
                                        ));
                                    }
                                    Ok(o) => o,
                                };
                            }
                        }

                        if src.is_file() {
                            match std::fs::copy(src, dst) {
                                Err(e) => {
                                    return Err(ShellError::labeled_error(
                                        e.to_string(),
                                        e.to_string(),
                                        name_span,
                                    ));
                                }
                                Ok(o) => o,
                            };
                        }
                    }
                } else {
                    destination.push(entry.file_name().unwrap());

                    match std::fs::create_dir_all(&destination) {
                        Err(e) => {
                            return Err(ShellError::labeled_error(
                                e.to_string(),
                                e.to_string(),
                                name_span,
                            ));
                        }
                        Ok(o) => o,
                    };

                    let strategy = |(source_file, depth_level)| {
                        let mut new_dst = dunce::canonicalize(&destination).unwrap();
                        let path = dunce::canonicalize(&source_file).unwrap();

                        let mut comps: Vec<_> = path
                            .components()
                            .map(|fragment| fragment.as_os_str())
                            .rev()
                            .take(1 + depth_level)
                            .collect();

                        comps.reverse();

                        for fragment in comps.iter() {
                            new_dst.push(fragment);
                        }

                        (PathBuf::from(&source_file), PathBuf::from(new_dst))
                    };

                    for (ref src, ref dst) in sources.paths_applying_with(strategy) {
                        if src.is_dir() {
                            if !dst.exists() {
                                match std::fs::create_dir_all(dst) {
                                    Err(e) => {
                                        return Err(ShellError::labeled_error(
                                            e.to_string(),
                                            e.to_string(),
                                            name_span,
                                        ));
                                    }
                                    Ok(o) => o,
                                };
                            }
                        }

                        if src.is_file() {
                            match std::fs::copy(src, dst) {
                                Err(e) => {
                                    return Err(ShellError::labeled_error(
                                        e.to_string(),
                                        e.to_string(),
                                        name_span,
                                    ));
                                }
                                Ok(o) => o,
                            };
                        }
                    }
                }
            }
        }
    } else {
        if destination.exists() {
            if !sources.iter().all(|x| (x.as_ref().unwrap()).is_file()) {
                return Err(ShellError::labeled_error(
                    "Copy aborted (directories found). Recursive copying in patterns not supported yet (try copying the directory directly)",
                    "Copy aborted (directories found). Recursive copying in patterns not supported yet (try copying the directory directly)",
                    args.nth(0).unwrap().span(),
                ));
            }

            for entry in sources {
                if let Ok(entry) = entry {
                    let mut to = PathBuf::from(&destination);
                    to.push(&entry.file_name().unwrap());

                    if entry.is_file() {
                        match std::fs::copy(&entry, &to) {
                            Err(e) => {
                                return Err(ShellError::labeled_error(
                                    e.to_string(),
                                    e.to_string(),
                                    args.nth(0).unwrap().span(),
                                ));
                            }
                            Ok(o) => o,
                        };
                    }
                }
            }
        } else {
            return Err(ShellError::labeled_error(
                format!(
                    "Copy aborted. (Does {:?} exist?)",
                    &destination.file_name().unwrap()
                ),
                format!(
                    "Copy aborted. (Does {:?} exist?)",
                    &destination.file_name().unwrap()
                ),
                args.nth(1).unwrap().span(),
            ));
        }
    }

    Ok(OutputStream::empty())
}
