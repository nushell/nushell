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
    pub full: bool,
    #[serde(rename = "short-names")]
    pub short_names: bool,
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
            .switch("full", "list all available columns for each entry")
            .switch("short-names", "only print the file names and not the path")
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
