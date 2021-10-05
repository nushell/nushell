use std::env::current_dir;
use std::path::PathBuf;

use nu_engine::{eval_expression, CallExt};
use nu_path::canonicalize_with;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{IntoValueStream, ShellError, Signature, SyntaxShape, Value};

use crate::filesystem::util::FileStructure;

pub struct Cp;

impl Command for Cp {
    fn name(&self) -> &str {
        "cp"
    }

    fn usage(&self) -> &str {
        "Copy files."
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
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        _input: Value,
    ) -> Result<Value, ShellError> {
        let source: String = call.req(context, 0)?;
        let destination: String = call.req(context, 1)?;

        let path: PathBuf = current_dir().unwrap();
        let source = path.join(source.as_str());
        let destination = path.join(destination.as_str());

        let mut sources =
            glob::glob(&source.to_string_lossy()).map_or_else(|_| Vec::new(), Iterator::collect);
        if sources.is_empty() {
            return Err(ShellError::FileNotFound(call.positional[0].span));
        }

        if sources.len() > 1 && !destination.is_dir() {
            return Err(ShellError::MoveNotPossible {
                source_message: "Can't move many files".to_string(),
                source_span: call.positional[0].span,
                destination_message: "into single file".to_string(),
                destination_span: call.positional[1].span,
            });
        }

        let any_source_is_dir = sources.iter().any(|f| matches!(f, Ok(f) if f.is_dir()));
        let recursive = call
            .named
            .iter()
            .fold(false, |acc, p| acc || { &p.0 == "recursive" });
        if any_source_is_dir && !recursive {
            return Err(ShellError::MoveNotPossibleSingle(
                "Directories must be copied using \"--recursive\"".to_string(),
                call.positional[0].span,
            ));
        }

        // for entry in sources.into_iter().flatten() {
        //     let mut sources = FileStructure::new();
        //     sources.walk_decorate(&entry)?;

        //     if entry.is_file() {
        //         let sources = sources.paths_applying_with(|(source_file, _depth_level)| {
        //             if destination.is_dir() {
        //                 let mut dest = canonicalize_with(&destination.item, &path)?;
        //                 if let Some(name) = entry.file_name() {
        //                     dest.push(name);
        //                 }
        //                 Ok((source_file, dest))
        //             } else {
        //                 Ok((source_file, destination.clone()))
        //             }
        //         })?;

        //         for (src, dst) in sources {
        //             if src.is_file() {
        //                 std::fs::copy(src, dst).map_err(|e| {
        //                     ShellError::labeled_error(e.to_string(), e.to_string(), &name_tag)
        //                 })?;
        //             }
        //         }
        //     } else if entry.is_dir() {
        //         let destination = if !destination.exists() {
        //             destination.clone()
        //         } else {
        //             match entry.file_name() {
        //                 Some(name) => destination.join(name),
        //                 None => {
        //                     return Err(ShellError::labeled_error(
        //                         "Copy aborted. Not a valid path",
        //                         "not a valid path",
        //                         dst.tag,
        //                     ))
        //                 }
        //             }
        //         };

        //         std::fs::create_dir_all(&destination).map_err(|e| {
        //             ShellError::labeled_error(e.to_string(), e.to_string(), &dst.tag)
        //         })?;

        //         let sources = sources.paths_applying_with(|(source_file, depth_level)| {
        //             let mut dest = destination.clone();
        //             let path = canonicalize_with(&source_file, &path)?;

        //             let comps: Vec<_> = path
        //                 .components()
        //                 .map(|fragment| fragment.as_os_str())
        //                 .rev()
        //                 .take(1 + depth_level)
        //                 .collect();

        //             for fragment in comps.into_iter().rev() {
        //                 dest.push(fragment);
        //             }

        //             Ok((PathBuf::from(&source_file), dest))
        //         })?;

        //         let dst_tag = &dst.tag;
        //         for (src, dst) in sources {
        //             if src.is_dir() && !dst.exists() {
        //                 std::fs::create_dir_all(&dst).map_err(|e| {
        //                     ShellError::labeled_error(e.to_string(), e.to_string(), dst_tag)
        //                 })?;
        //             }

        //             if src.is_file() {
        //                 std::fs::copy(&src, &dst).map_err(|e| {
        //                     ShellError::labeled_error(e.to_string(), e.to_string(), &name_tag)
        //                 })?;
        //             }
        //         }
        //     }
        // }

        Ok(Value::Nothing { span: call.head })
    }
}
