use crate::help::{help_aliases, help_commands, help_modules};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Help;

impl Command for Help {
    fn name(&self) -> &str {
        "help"
    }

    fn signature(&self) -> Signature {
        Signature::build("help")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .rest(
                "rest",
                SyntaxShape::String,
                "The name of command, alias or module to get help on.",
            )
            .named(
                "find",
                SyntaxShape::String,
                "string to find in command names, descriptions, and search terms",
                Some('f'),
            )
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Display help information about different parts of Nushell."
    }

    fn extra_description(&self) -> &str {
        r#"`help word` searches for "word" in commands, aliases and modules, in that order.
If you want your own help implementation, create a custom command named `help` and it will also be used for `--help` invocations.
There already is an alternative `help` command in the standard library you can try with `use std/help`."#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let find: Option<Spanned<String>> = call.get_flag(engine_state, stack, "find")?;
        let rest: Vec<Spanned<String>> = call.rest(engine_state, stack, 0)?;

        if rest.is_empty() && find.is_none() {
            let msg = r#"Welcome to Nushell.

Here are some tips to help you get started.
  * help -h or help help - show available `help` subcommands and examples
  * help commands - list all available commands
  * help <name> - display help about a particular command, alias, or module
  * help --find <text to search> - search through all help commands table

Nushell works on the idea of a "pipeline". Pipelines are commands connected with the '|' character.
Each stage in the pipeline works together to load, parse, and display information to you.

[Examples]

List the files in the current directory, sorted by size:
    ls | sort-by size

Get the current system host name:
    sys host | get hostname

Get the processes on your system actively using CPU:
    ps | where cpu > 0

You can also learn more at https://www.nushell.sh/book/"#;

            Ok(Value::string(msg, head).into_pipeline_data())
        } else if find.is_some() {
            help_commands(engine_state, stack, call)
        } else {
            let result = help_aliases(engine_state, stack, call);

            let result = if let Err(ShellError::AliasNotFound { .. }) = result {
                help_commands(engine_state, stack, call)
            } else {
                result
            };

            let result = if let Err(ShellError::CommandNotFound { .. }) = result {
                help_modules(engine_state, stack, call)
            } else {
                result
            };

            if let Err(ShellError::ModuleNotFoundAtRuntime {
                mod_name: _,
                span: _,
            }) = result
            {
                Err(ShellError::NotFound {
                    span: Span::merge_many(rest.iter().map(|s| s.span)),
                })
            } else {
                result
            }
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "show help for single command, alias, or module",
                example: "help match",
                result: None,
            },
            Example {
                description: "show help for single sub-command, alias, or module",
                example: "help str join",
                result: None,
            },
            Example {
                description: "search for string in command names, descriptions, and search terms",
                example: "help --find char",
                result: None,
            },
        ]
    }
}
