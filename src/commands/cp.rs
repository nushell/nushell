use crate::commands::command::RunnablePerItemContext;
use crate::errors::ShellError;
use crate::parser::hir::SyntaxType;
use crate::parser::registry::{CommandRegistry, Signature};
use crate::prelude::*;
use crate::utils::FileStructure;
use std::path::PathBuf;

pub struct Cpy;

#[derive(Deserialize)]
pub struct CopyArgs {
    src: Tagged<PathBuf>,
    dst: Tagged<PathBuf>,
    recursive: Tagged<bool>,
}

impl PerItemCommand for Cpy {
    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        shell_manager: &ShellManager,
        _input: Tagged<Value>,
    ) -> Result<VecDeque<ReturnValue>, ShellError> {
        call_info.process(shell_manager, cp)?.run()
    }

    fn name(&self) -> &str {
        "cp"
    }

    fn signature(&self) -> Signature {
        Signature::build("cp")
            .required("src", SyntaxType::Path)
            .required("dst", SyntaxType::Path)
            .named("file", SyntaxType::Any)
            .switch("recursive")
    }
}

fn cp(
    CopyArgs {
        src,
        dst,
        recursive,
    }: CopyArgs,
    context: &RunnablePerItemContext,
) -> Result<VecDeque<ReturnValue>, ShellError> {
    let name_span = context.name;

    let mut source = PathBuf::from(context.shell_manager.path());
    let mut destination = PathBuf::from(context.shell_manager.path());

    source.push(&src.item);
    destination.push(&dst.item);

    let sources: Vec<_> = match glob::glob(&source.to_string_lossy()) {
        Ok(files) => files.collect(),
        Err(_) => {
            return Err(ShellError::labeled_error(
                "Invalid pattern.",
                "Invalid pattern.",
                src.tag,
            ))
        }
    };

    if sources.len() == 1 {
        if let Ok(entry) = &sources[0] {
            if entry.is_dir() && !recursive.item {
                return Err(ShellError::labeled_error(
                    "is a directory (not copied). Try using \"--recursive\".",
                    "is a directory (not copied). Try using \"--recursive\".",
                    src.tag,
                ));
            }

            let mut sources: FileStructure = FileStructure::new();

            sources.walk_decorate(&entry)?;

            if entry.is_file() {
                let strategy = |(source_file, _depth_level)| {
                    if destination.exists() {
                        let mut new_dst = dunce::canonicalize(destination.clone())?;
                        if let Some(name) = entry.file_name() {
                            new_dst.push(name);
                        }
                        Ok((source_file, new_dst))
                    } else {
                        Ok((source_file, destination.clone()))
                    }
                };

                let sources = sources.paths_applying_with(strategy)?;

                for (ref src, ref dst) in sources {
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
                        let path = dunce::canonicalize(&source_file)?;

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

                        Ok((PathBuf::from(&source_file), PathBuf::from(new_dst)))
                    };

                    let sources = sources.paths_applying_with(strategy)?;

                    for (ref src, ref dst) in sources {
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
                    match entry.file_name() {
                        Some(name) => destination.push(name),
                        None => {
                            return Err(ShellError::labeled_error(
                                "Copy aborted. Not a valid path",
                                "Copy aborted. Not a valid path",
                                name_span,
                            ))
                        }
                    }

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
                        let mut new_dst = dunce::canonicalize(&destination)?;
                        let path = dunce::canonicalize(&source_file)?;

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

                        Ok((PathBuf::from(&source_file), PathBuf::from(new_dst)))
                    };

                    let sources = sources.paths_applying_with(strategy)?;

                    for (ref src, ref dst) in sources {
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
            if !sources.iter().all(|x| match x {
                Ok(f) => f.is_file(),
                Err(_) => false,
            }) {
                return Err(ShellError::labeled_error(
                    "Copy aborted (directories found). Recursive copying in patterns not supported yet (try copying the directory directly)",
                    "Copy aborted (directories found). Recursive copying in patterns not supported yet (try copying the directory directly)",
                    src.tag,
                ));
            }

            for entry in sources {
                if let Ok(entry) = entry {
                    let mut to = PathBuf::from(&destination);

                    match entry.file_name() {
                        Some(name) => to.push(name),
                        None => {
                            return Err(ShellError::labeled_error(
                                "Copy aborted. Not a valid path",
                                "Copy aborted. Not a valid path",
                                name_span,
                            ))
                        }
                    }

                    if entry.is_file() {
                        match std::fs::copy(&entry, &to) {
                            Err(e) => {
                                return Err(ShellError::labeled_error(
                                    e.to_string(),
                                    e.to_string(),
                                    src.tag,
                                ));
                            }
                            Ok(o) => o,
                        };
                    }
                }
            }
        } else {
            let destination_file_name = {
                match destination.file_name() {
                    Some(name) => PathBuf::from(name),
                    None => {
                        return Err(ShellError::labeled_error(
                            "Copy aborted. Not a valid destination",
                            "Copy aborted. Not a valid destination",
                            name_span,
                        ))
                    }
                }
            };

            return Err(ShellError::labeled_error(
                format!("Copy aborted. (Does {:?} exist?)", destination_file_name),
                format!("Copy aborted. (Does {:?} exist?)", destination_file_name),
                &dst.span(),
            ));
        }
    }

    Ok(VecDeque::new())
}
