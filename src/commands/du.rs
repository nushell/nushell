use crate::commands::command::RunnablePerItemContext;
use crate::prelude::*;
use glob::*;
use nu_errors::ShellError;
use nu_protocol::{
    CallInfo, ReturnSuccess, ReturnValue, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue,
    Value,
};
use nu_source::Tagged;
use std::path::PathBuf;

const NAME: &str = "du";
const GLOB_PARAMS: MatchOptions = MatchOptions {
    case_sensitive: true,
    require_literal_separator: true,
    require_literal_leading_dot: false,
};

const DIR: &str = DIR;

pub struct Du;

#[derive(Deserialize)]
pub struct DuArgs {
    path: Tagged<Option<PathBuf>>,
    all: bool,
    deref: bool,
    exclude: Tagged<Option<String>>,
    max_depth: Tagged<Option<u64>>,
    min_size: Tagged<Option<u64>>,
}

impl PerItemCommand for Du {
    fn name(&self) -> &str {
        NAME
    }

    fn signature(&self) -> Signature {
        Signature::build(NAME)
            .optional("path", SyntaxShape::Pattern, "starting directory")
            .switch("all", "Output File sizes as well as directory sizes")
            .switch("deref", "Dereference symlinks to their targets for size")
            .named("exclude", SyntaxShape::Pattern, "Exclude these file names")
            .named("max-depth", SyntaxShape::Int, "Directory recursion limit")
    }

    fn usage(&self) -> &str {
        "Find disk usage sizes of specified items"
    }

    fn run(
        &self,
        call_info: &CallInfo,
        registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        input: Value,
    ) -> Result<OutputStream, ShellError> {
        call_info
            .process(&raw_args.shell_manager, raw_args.ctrl_c.clone(), du)?
            .run()
    }
}

fn du(args: DuArgs, ctx: &RunnablePerItemContext) -> Result<OutputStream, ShellError> {}
