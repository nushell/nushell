use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    span, Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
    ShellError, Signature, Spanned, SyntaxShape, Value,
};

use nu_engine::{get_full_help, CallExt};

#[derive(Clone)]
pub struct Help;

impl Command for Help {
    fn name(&self) -> &str {
        "help"
    }

    fn signature(&self) -> Signature {
        Signature::build("help")
            .rest(
                "rest",
                SyntaxShape::String,
                "the name of command to get help on",
            )
            .named(
                "find",
                SyntaxShape::String,
                "string to find in command usage",
                Some('f'),
            )
            .category(Category::Core)
    }

    fn usage(&self) -> &str {
        "Display help information about commands."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        help(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "show all commands and sub-commands",
                example: "help commands",
                result: None,
            },
            Example {
                description: "generate documentation",
                example: "help generate_docs",
                result: None,
            },
            Example {
                description: "show help for single command",
                example: "help match",
                result: None,
            },
            Example {
                description: "show help for single sub-command",
                example: "help str lpad",
                result: None,
            },
            Example {
                description: "search for string in command usage",
                example: "help --find char",
                result: None,
            },
        ]
    }
}

fn help(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let find: Option<Spanned<String>> = call.get_flag(engine_state, stack, "find")?;
    let rest: Vec<Spanned<String>> = call.rest(engine_state, stack, 0)?;

    let full_commands = engine_state.get_signatures_with_examples(false);

    if let Some(f) = find {
        let search_string = f.item.to_lowercase();
        let mut found_cmds_vec = Vec::new();

        for (sig, _, is_plugin, is_custom) in full_commands {
            let mut cols = vec![];
            let mut vals = vec![];

            let key = sig.name.clone();
            let c = sig.usage.clone();
            let e = sig.extra_usage.clone();
            if key.to_lowercase().contains(&search_string)
                || c.to_lowercase().contains(&search_string)
                || e.to_lowercase().contains(&search_string)
            {
                cols.push("name".into());
                vals.push(Value::String {
                    val: key,
                    span: head,
                });

                cols.push("category".into());
                vals.push(Value::String {
                    val: sig.category.to_string(),
                    span: head,
                });

                cols.push("is_plugin".into());
                vals.push(Value::Bool {
                    val: is_plugin,
                    span: head,
                });

                cols.push("is_custom".into());
                vals.push(Value::Bool {
                    val: is_custom,
                    span: head,
                });

                cols.push("usage".into());
                vals.push(Value::String { val: c, span: head });

                cols.push("extra_usage".into());
                vals.push(Value::String { val: e, span: head });

                found_cmds_vec.push(Value::Record {
                    cols,
                    vals,
                    span: head,
                });
            }
        }

        return Ok(found_cmds_vec
            .into_iter()
            .into_pipeline_data(engine_state.ctrlc.clone()));
    }

    if !rest.is_empty() {
        let mut found_cmds_vec = Vec::new();

        if rest[0].item == "commands" {
            for (sig, _, is_plugin, is_custom) in full_commands {
                let mut cols = vec![];
                let mut vals = vec![];

                let key = sig.name.clone();
                let c = sig.usage.clone();
                let e = sig.extra_usage.clone();

                cols.push("name".into());
                vals.push(Value::String {
                    val: key,
                    span: head,
                });

                cols.push("category".into());
                vals.push(Value::String {
                    val: sig.category.to_string(),
                    span: head,
                });

                cols.push("is_plugin".into());
                vals.push(Value::Bool {
                    val: is_plugin,
                    span: head,
                });

                cols.push("is_custom".into());
                vals.push(Value::Bool {
                    val: is_custom,
                    span: head,
                });

                cols.push("usage".into());
                vals.push(Value::String { val: c, span: head });

                cols.push("extra_usage".into());
                vals.push(Value::String { val: e, span: head });

                found_cmds_vec.push(Value::Record {
                    cols,
                    vals,
                    span: head,
                });
            }

            Ok(found_cmds_vec
                .into_iter()
                .into_pipeline_data(engine_state.ctrlc.clone()))
        } else {
            let mut name = String::new();

            for r in &rest {
                if !name.is_empty() {
                    name.push(' ');
                }
                name.push_str(&r.item);
            }

            let output = full_commands
                .iter()
                .filter(|(signature, _, _, _)| signature.name == name)
                .map(|(signature, examples, _, _)| {
                    get_full_help(signature, examples, engine_state, stack)
                })
                .collect::<Vec<String>>();

            if !output.is_empty() {
                Ok(Value::String {
                    val: output.join("======================\n\n"),
                    span: call.head,
                }
                .into_pipeline_data())
            } else {
                Err(ShellError::CommandNotFound(span(&[
                    rest[0].span,
                    rest[rest.len() - 1].span,
                ])))
            }
        }
    } else {
        let msg = r#"Welcome to Nushell.

Here are some tips to help you get started.
  * help commands - list all available commands
  * help <command name> - display help about a particular command

Nushell works on the idea of a "pipeline". Pipelines are commands connected with the '|' character.
Each stage in the pipeline works together to load, parse, and display information to you.

[Examples]

List the files in the current directory, sorted by size:
    ls | sort-by size

Get information about the current system:
    sys | get host

Get the processes on your system actively using CPU:
    ps | where cpu > 0

You can also learn more at https://www.nushell.sh/book/"#;

        Ok(Value::String {
            val: msg.into(),
            span: head,
        }
        .into_pipeline_data())
    }
}
