// use std::fs::read_link;
// use std::io::{BufReader, BufWriter, ErrorKind, Read, Write};
// use std::path::PathBuf;
// use std::sync::atomic::AtomicBool;
// use std::sync::Arc;

// use nu_engine::env::current_dir;
// use nu_engine::CallExt;
// use nu_path::{canonicalize_with, expand_path_with};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    Spanned, SyntaxShape, Type, Value,
};
use std::ffi::{OsStr, OsString};
// use super::util::try_interaction;

// use crate::filesystem::util::FileStructure;
// use crate::progress_bar;

use uu_cp::*;

#[derive(Clone)]
pub struct Ucp;

impl Command for Ucp {
    fn name(&self) -> &str {
        "ucp"
    }

    fn usage(&self) -> &str {
        "Copy files."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["copy", "file", "files"]
    }

    fn signature(&self) -> Signature {
        Signature::build("cp")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("source", SyntaxShape::GlobPattern, "the place to copy from")
            .required("destination", SyntaxShape::Filepath, "the place to copy to")
            .switch(
                "recursive",
                "copy recursively through subdirectories",
                Some('r'),
            )
            .switch(
                "verbose",
                "show successful copies in addition to failed copies (default:false)",
                Some('v'),
            )
            .switch("update",
                "copy only when the SOURCE file is newer than the destination file or when the destination file is missing",
                Some('u')
            )
            // TODO: add back in additional features
            // .switch("force", "suppress error when no file", Some('f'))
            .switch("interactive", "ask user to confirm action", Some('i'))
            .switch(
                "no-symlink",
                "no symbolic links are followed, only works if -r is active",
                Some('n'),
            )
            .switch("progress", "enable progress bar", Some('p'))
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // Get the App
        // let mut app = uu_cp::uu_app();
        // app.print_help()?;

        // Create uucore::Args somehow from nushell args
        let s1 = "cp".to_string();
        let s2 = "-h".to_string();
        let args = vec![OsString::from(s1), OsString::from(s2)];
        // Pass uucore::Args to app.uumain
        uu_cp::uumain(&mut args.into_iter());
        Ok(PipelineData::empty())
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
            Example {
                description: "Recursively copy dir_a to dir_b, and print the feedbacks",
                example: "cp -r -v dir_a dir_b",
                result: None,
            },
            Example {
                description: "Move many files into a directory",
                example: "cp *.txt dir_a",
                result: None,
            },
            Example {
                description: "Copy only if source file is newer than target file",
                example: "cp -u a b",
                result: None,
            },
        ]
    }
}
