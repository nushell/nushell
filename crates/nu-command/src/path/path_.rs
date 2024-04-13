use nu_engine::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct PathCommand;

impl Command for PathCommand {
    fn name(&self) -> &str {
        "path"
    }

    fn signature(&self) -> Signature {
        Signature::build("path")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .category(Category::Path)
    }

    fn usage(&self) -> &str {
        "Explore and manipulate paths."
    }

    fn extra_usage(&self) -> &str {
        r#"You must use one of the following subcommands. Using this command as-is will only produce this help message.

There are three ways to represent a path:

* As a path literal, e.g., '/home/viking/spam.txt'
* As a structured path: a table with 'parent', 'stem', and 'extension' (and
* 'prefix' on Windows) columns. This format is produced by the 'path parse'
  subcommand.
* As a list of path parts, e.g., '[ / home viking spam.txt ]'. Splitting into
  parts is done by the `path split` command.

All subcommands accept all three variants as an input. Furthermore, the 'path
join' subcommand can be used to join the structured path or path parts back into
the path literal."#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::string(
            get_full_help(
                &PathCommand.signature(),
                &PathCommand.examples(),
                engine_state,
                stack,
                self.is_parser_keyword(),
            ),
            call.head,
        )
        .into_pipeline_data())
    }
}
