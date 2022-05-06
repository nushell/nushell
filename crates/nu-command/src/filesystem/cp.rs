use std::path::PathBuf;

use nu_engine::env::current_dir;
use nu_engine::CallExt;
use nu_path::canonicalize_with;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape};

use crate::filesystem::util::FileStructure;

const GLOB_PARAMS: nu_glob::MatchOptions = nu_glob::MatchOptions {
    case_sensitive: true,
    require_literal_separator: false,
    require_literal_leading_dot: false,
};

#[derive(Clone)]
pub struct Cp;

#[allow(unused_must_use)]
impl Command for Cp {
    fn name(&self) -> &str {
        "cp"
    }

    fn usage(&self) -> &str {
        "Copy files."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["cp", "copy", "file", "files"]
    }

    fn signature(&self) -> Signature {
        Signature::build("cp")
            .required("source", SyntaxShape::GlobPattern, "the place to copy from")
            .required("destination", SyntaxShape::Filepath, "the place to copy to")
            .switch(
                "recursive",
                "copy recursively through subdirectories",
                Some('r'),
            )
            // TODO: add back in additional features
            // .switch("force", "suppress error when no file", Some('f'))
            // .switch("interactive", "ask user to confirm action", Some('i'))
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let src: Spanned<String> = call.req(engine_state, stack, 0)?;
        let dst: Spanned<String> = call.req(engine_state, stack, 1)?;
        let recursive = call.has_flag("recursive");

        let path = current_dir(engine_state, stack)?;
        let source = path.join(src.item.as_str());
        let destination = path.join(dst.item.as_str());

        let sources: Vec<_> = match nu_glob::glob_with(&source.to_string_lossy(), GLOB_PARAMS) {
            Ok(files) => files.collect(),
            Err(e) => {
                return Err(ShellError::GenericError(
                    e.to_string(),
                    "invalid pattern".to_string(),
                    Some(src.span),
                    None,
                    Vec::new(),
                ))
            }
        };

        if sources.is_empty() {
            return Err(ShellError::GenericError(
                "No matches found".into(),
                "no matches found".into(),
                Some(src.span),
                None,
                Vec::new(),
            ));
        }

        if sources.len() > 1 && !destination.is_dir() {
            return Err(ShellError::GenericError(
                "Destination must be a directory when copying multiple files".into(),
                "is not a directory".into(),
                Some(dst.span),
                None,
                Vec::new(),
            ));
        }

        let any_source_is_dir = sources.iter().any(|f| matches!(f, Ok(f) if f.is_dir()));

        if any_source_is_dir && !recursive {
            return Err(ShellError::GenericError(
                "Directories must be copied using \"--recursive\"".into(),
                "resolves to a directory (not copied)".into(),
                Some(src.span),
                None,
                Vec::new(),
            ));
        }

        for entry in sources.into_iter().flatten() {
            let mut sources = FileStructure::new();
            sources.walk_decorate(&entry, engine_state, stack)?;

            if entry.is_file() {
                let sources = sources.paths_applying_with(|(source_file, _depth_level)| {
                    if destination.is_dir() {
                        let mut dest = canonicalize_with(&dst.item, &path)?;
                        if let Some(name) = entry.file_name() {
                            dest.push(name);
                        }
                        Ok((source_file, dest))
                    } else {
                        Ok((source_file, destination.clone()))
                    }
                })?;

                for (src, dst) in sources {
                    if src.is_file() {
                        std::fs::copy(src, dst).map_err(|e| {
                            ShellError::GenericError(
                                e.to_string(),
                                e.to_string(),
                                Some(call.head),
                                None,
                                Vec::new(),
                            )
                        })?;
                    }
                }
            } else if entry.is_dir() {
                let destination = if !destination.exists() {
                    destination.clone()
                } else {
                    match entry.file_name() {
                        Some(name) => destination.join(name),
                        None => {
                            return Err(ShellError::GenericError(
                                "Copy aborted. Not a valid path".into(),
                                "not a valid path".into(),
                                Some(dst.span),
                                None,
                                Vec::new(),
                            ))
                        }
                    }
                };

                std::fs::create_dir_all(&destination).map_err(|e| {
                    ShellError::GenericError(
                        e.to_string(),
                        e.to_string(),
                        Some(dst.span),
                        None,
                        Vec::new(),
                    )
                })?;

                let sources = sources.paths_applying_with(|(source_file, depth_level)| {
                    let mut dest = destination.clone();
                    let path = canonicalize_with(&source_file, &path)?;

                    #[allow(clippy::needless_collect)]
                    let comps: Vec<_> = path
                        .components()
                        .map(|fragment| fragment.as_os_str())
                        .rev()
                        .take(1 + depth_level)
                        .collect();

                    for fragment in comps.into_iter().rev() {
                        dest.push(fragment);
                    }

                    Ok((PathBuf::from(&source_file), dest))
                })?;

                for (s, d) in sources {
                    if s.is_dir() && !d.exists() {
                        std::fs::create_dir_all(&d).map_err(|e| {
                            ShellError::GenericError(
                                e.to_string(),
                                e.to_string(),
                                Some(dst.span),
                                None,
                                Vec::new(),
                            )
                        })?;
                    }

                    if s.is_file() {
                        std::fs::copy(&s, &d).map_err(|e| {
                            ShellError::GenericError(
                                e.to_string(),
                                e.to_string(),
                                Some(call.head),
                                None,
                                Vec::new(),
                            )
                        })?;
                    }
                }
            }
        }

        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Copy myfile to dir_b",
                example: "cp myfile dir_b",
                result: None,
            },
            Example {
                description: "Recursively copy dir_a to dir_b",
                example: "cp -r dir_a dir_b",
                result: None,
            },
        ]
    }
}
