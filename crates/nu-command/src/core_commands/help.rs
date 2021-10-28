use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    span, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Spanned, SyntaxShape, Value,
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

    let full_commands = engine_state.get_signatures_with_examples();

    if let Some(f) = find {
        let search_string = f.item;
        let mut found_cmds_vec = Vec::new();

        for cmd in full_commands {
            let mut cols = vec![];
            let mut vals = vec![];

            let key = cmd.0.name.clone();
            let c = cmd.0.usage.clone();
            let e = cmd.0.extra_usage.clone();
            if key.to_lowercase().contains(&search_string)
                || c.to_lowercase().contains(&search_string)
                || e.to_lowercase().contains(&search_string)
            {
                cols.push("name".into());
                vals.push(Value::String {
                    val: key,
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
            for cmd in full_commands {
                let mut cols = vec![];
                let mut vals = vec![];

                let key = cmd.0.name.clone();
                let c = cmd.0.usage.clone();
                let e = cmd.0.extra_usage.clone();

                cols.push("name".into());
                vals.push(Value::String {
                    val: key,
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
            let mut output = String::new();

            for r in &rest {
                if !name.is_empty() {
                    name.push(' ');
                }
                name.push_str(&r.item);
            }

            for cmd in full_commands {
                if cmd.0.name == name {
                    let help = get_full_help(&cmd.0, &cmd.1, engine_state);
                    output.push_str(&help);
                }
            }

            if !output.is_empty() {
                Ok(Value::String {
                    val: output,
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

        // FIXME: the fancy help stuff needs to be reimplemented
        /*
        if rest[0].item == "commands" {
            let mut sorted_names = scope.get_command_names();
            sorted_names.sort();

            let (mut subcommand_names, command_names) = sorted_names
                .into_iter()
                // private only commands shouldn't be displayed
                .filter(|cmd_name| {
                    scope
                        .get_command(cmd_name)
                        .filter(|command| !command.is_private())
                        .is_some()
                })
                .partition::<Vec<_>, _>(|cmd_name| cmd_name.contains(' '));

            fn process_name(
                dict: &mut TaggedDictBuilder,
                cmd_name: &str,
                scope: Scope,
                rest: Vec<Tagged<String>>,
                name: Tag,
            ) -> Result<(), ShellError> {
                let document_tag = rest[0].tag.clone();
                let value = command_dict(
                    scope.get_command(cmd_name).ok_or_else(|| {
                        ShellError::labeled_error(
                            format!("Could not load {}", cmd_name),
                            "could not load command",
                            document_tag,
                        )
                    })?,
                    name,
                );

                dict.insert_untagged("name", cmd_name);
                dict.insert_untagged(
                    "description",
                    value
                        .get_data_by_key("usage".spanned_unknown())
                        .ok_or_else(|| {
                            ShellError::labeled_error(
                                "Expected a usage key",
                                "expected a 'usage' key",
                                &value.tag,
                            )
                        })?
                        .as_string()?,
                );

                Ok(())
            }

            fn make_subcommands_table(
                subcommand_names: &mut Vec<String>,
                cmd_name: &str,
                scope: Scope,
                rest: Vec<Tagged<String>>,
                name: Tag,
            ) -> Result<Value, ShellError> {
                let (matching, not_matching) =
                    subcommand_names.drain(..).partition(|subcommand_name| {
                        subcommand_name.starts_with(&format!("{} ", cmd_name))
                    });
                *subcommand_names = not_matching;
                Ok(if !matching.is_empty() {
                    UntaggedValue::table(
                        &(matching
                            .into_iter()
                            .map(|cmd_name: String| -> Result<_, ShellError> {
                                let mut short_desc = TaggedDictBuilder::new(name.clone());
                                process_name(
                                    &mut short_desc,
                                    &cmd_name,
                                    scope.clone(),
                                    rest.clone(),
                                    name.clone(),
                                )?;
                                Ok(short_desc.into_value())
                            })
                            .collect::<Result<Vec<_>, _>>()?[..]),
                    )
                    .into_value(name)
                } else {
                    UntaggedValue::nothing().into_value(name)
                })
            }

            let iterator =
                command_names
                    .into_iter()
                    .map(move |cmd_name| -> Result<_, ShellError> {
                        let mut short_desc = TaggedDictBuilder::new(name.clone());
                        process_name(
                            &mut short_desc,
                            &cmd_name,
                            scope.clone(),
                            rest.clone(),
                            name.clone(),
                        )?;
                        short_desc.insert_value(
                            "subcommands",
                            make_subcommands_table(
                                &mut subcommand_names,
                                &cmd_name,
                                scope.clone(),
                                rest.clone(),
                                name.clone(),
                            )?,
                        );
                        ReturnSuccess::value(short_desc.into_value())
                    });

            Ok(iterator.into_action_stream())
        } else if rest[0].item == "generate_docs" {
            Ok(ActionStream::one(ReturnSuccess::value(generate_docs(
                &scope,
            ))))
        } else if rest.len() == 2 {
            // Check for a subcommand
            let command_name = format!("{} {}", rest[0].item, rest[1].item);
            if let Some(command) = scope.get_command(&command_name) {
                Ok(ActionStream::one(ReturnSuccess::value(
                    UntaggedValue::string(get_full_help(command.stream_command(), &scope))
                        .into_value(Tag::unknown()),
                )))
            } else {
                Ok(ActionStream::empty())
            }
        } else if let Some(command) = scope.get_command(&rest[0].item) {
            Ok(ActionStream::one(ReturnSuccess::value(
                UntaggedValue::string(get_full_help(command.stream_command(), &scope))
                    .into_value(Tag::unknown()),
            )))
        } else {
            Err(ShellError::labeled_error(
                "Can't find command (use 'help commands' for full list)",
                "can't find command",
                rest[0].tag.span,
            ))
        }
        */
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

/*
fn for_spec(name: &str, ty: &str, required: bool, tag: impl Into<Tag>) -> Value {
    let tag = tag.into();

    let mut spec = TaggedDictBuilder::new(tag);

    spec.insert_untagged("name", UntaggedValue::string(name));
    spec.insert_untagged("type", UntaggedValue::string(ty));
    spec.insert_untagged(
        "required",
        UntaggedValue::string(if required { "yes" } else { "no" }),
    );

    spec.into_value()
}

pub fn signature_dict(signature: Signature, tag: impl Into<Tag>) -> Value {
    let tag = tag.into();
    let mut sig = TaggedListBuilder::new(&tag);

    for arg in &signature.positional {
        let is_required = matches!(arg.0, PositionalType::Mandatory(_, _));

        sig.push_value(for_spec(arg.0.name(), "argument", is_required, &tag));
    }

    if signature.rest_positional.is_some() {
        let is_required = false;
        sig.push_value(for_spec("rest", "argument", is_required, &tag));
    }

    for (name, ty) in &signature.named {
        match ty.0 {
            NamedType::Mandatory(_, _) => sig.push_value(for_spec(name, "flag", true, &tag)),
            NamedType::Optional(_, _) => sig.push_value(for_spec(name, "flag", false, &tag)),
            NamedType::Switch(_) => sig.push_value(for_spec(name, "switch", false, &tag)),
        }
    }

    sig.into_value()
}

fn command_dict(command: Command, tag: impl Into<Tag>) -> Value {
    let tag = tag.into();

    let mut cmd_dict = TaggedDictBuilder::new(&tag);

    cmd_dict.insert_untagged("name", UntaggedValue::string(command.name()));

    cmd_dict.insert_untagged("type", UntaggedValue::string("Command"));

    cmd_dict.insert_value("signature", signature_dict(command.signature(), tag));
    cmd_dict.insert_untagged("usage", UntaggedValue::string(command.usage()));

    cmd_dict.into_value()
}

*/
