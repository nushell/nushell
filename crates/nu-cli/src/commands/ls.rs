use crate::commands::command::RunnablePerItemContext;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{CallInfo, Signature, SyntaxShape, Value};
use nu_source::Tagged;
use std::path::PathBuf;

pub struct Ls;

#[derive(Deserialize)]
pub struct LsArgs {
    pub path: Option<Tagged<PathBuf>>,
    pub all: bool,
    pub full: bool,
    #[serde(rename = "short-names")]
    pub short_names: bool,
    #[serde(rename = "with-symlink-targets")]
    pub with_symlink_targets: bool,
}

impl PerItemCommand for Ls {
    fn name(&self) -> &str {
        "ls"
    }

    fn signature(&self) -> Signature {
        Signature::build("ls")
            .optional(
                "path",
                SyntaxShape::Pattern,
                "a path to get the directory contents from",
            )
            .switch("all", "also show hidden files", Some('a'))
            .switch(
                "full",
                "list all available columns for each entry",
                Some('f'),
            )
            .switch(
                "short-names",
                "only print the file names and not the path",
                Some('s'),
            )
            .switch(
                "with-symlink-targets",
                "display the paths to the target files that symlinks point to",
                Some('w'),
            )
    }

    fn usage(&self) -> &str {
        "View the contents of the current or given path."
    }

    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        _input: Value,
    ) -> Result<OutputStream, ShellError> {
        call_info
            .process(&raw_args.shell_manager, raw_args.ctrl_c.clone(), ls)?
            .run()
    }
}

fn ls(args: LsArgs, context: &RunnablePerItemContext) -> Result<OutputStream, ShellError> {
    context.shell_manager.ls(args, context)
}
