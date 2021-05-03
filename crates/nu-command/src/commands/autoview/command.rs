use crate::commands::autoview::options::ConfigExtensions;
use crate::prelude::*;
use crate::primitive::get_color_config;
use nu_data::value::format_leaf;
use nu_engine::{UnevaluatedCallInfo, WholeStreamCommand};
use nu_errors::ShellError;
use nu_protocol::hir::{self, Expression, ExternalRedirection, Literal, SpannedExpression};
use nu_protocol::{Primitive, Signature, UntaggedValue, Value};
use nu_table::TextStyle;

pub struct Command;

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "autoview"
    }

    fn signature(&self) -> Signature {
        Signature::build("autoview")
    }

    fn usage(&self) -> &str {
        "View the contents of the pipeline as a table or list."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        autoview(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Automatically view the results",
                example: "ls | autoview",
                result: None,
            },
            Example {
                description: "Autoview is also implied. The above can be written as",
                example: "ls",
                result: None,
            },
        ]
    }
}

pub fn autoview(context: CommandArgs) -> Result<OutputStream, ShellError> {
    let configuration = context.configs.lock().global_config();

    let binary = context.scope.get_command("binaryview");
    let text = context.scope.get_command("textview");
    let table = context.scope.get_command("table");

    let (mut input_stream, context) = context.split();

    if let Some(x) = input_stream.next() {
        match input_stream.next() {
            Some(y) => {
                let ctrl_c = context.ctrl_c.clone();
                let xy = vec![x, y];
                let xy_stream = xy.into_iter().chain(input_stream).interruptible(ctrl_c);

                let stream = InputStream::from_stream(xy_stream);

                if let Some(table) = table {
                    let command_args = create_default_command_args(&context).with_input(stream);
                    let result = table.run(command_args)?;
                    let _ = result.collect::<Vec<_>>();
                }
            }
            _ => {
                match x {
                    Value {
                        value: UntaggedValue::Primitive(Primitive::String(ref s)),
                        tag: Tag { anchor, span },
                    } if anchor.is_some() => {
                        if let Some(text) = text {
                            let mut stream = VecDeque::new();
                            stream.push_back(
                                UntaggedValue::string(s).into_value(Tag { anchor, span }),
                            );
                            let command_args =
                                create_default_command_args(&context).with_input(stream);
                            let result = text.run_with_actions(command_args)?;
                            let _ = result.collect::<Vec<_>>();
                        } else {
                            out!("{}", s);
                        }
                    }
                    Value {
                        value: UntaggedValue::Primitive(Primitive::String(s)),
                        ..
                    } => {
                        out!("{}", s);
                    }
                    Value {
                        value: UntaggedValue::Primitive(Primitive::FilePath(s)),
                        ..
                    } => {
                        out!("{}", s.display());
                    }
                    Value {
                        value: UntaggedValue::Primitive(Primitive::Int(n)),
                        ..
                    } => {
                        out!("{}", n);
                    }
                    Value {
                        value: UntaggedValue::Primitive(Primitive::Decimal(n)),
                        ..
                    } => {
                        // TODO: normalize decimal to remove trailing zeros.
                        // normalization will be available in next release of bigdecimal crate
                        let mut output = n.to_string();
                        if output.contains('.') {
                            output = output.trim_end_matches('0').to_owned();
                        }
                        if output.ends_with('.') {
                            output.push('0');
                        }
                        out!("{}", output);
                    }
                    Value {
                        value: UntaggedValue::Primitive(Primitive::Boolean(b)),
                        ..
                    } => {
                        out!("{}", b);
                    }
                    Value {
                        value: UntaggedValue::Primitive(Primitive::Duration(_)),
                        ..
                    } => {
                        let output = format_leaf(&x).plain_string(100_000);
                        out!("{}", output);
                    }
                    Value {
                        value: UntaggedValue::Primitive(Primitive::Filesize(_)),
                        ..
                    } => {
                        let output = format_leaf(&x).plain_string(100_000);
                        out!("{}", output);
                    }
                    Value {
                        value: UntaggedValue::Primitive(Primitive::Date(d)),
                        ..
                    } => {
                        out!("{}", d);
                    }
                    Value {
                        value: UntaggedValue::Primitive(Primitive::Range(_)),
                        ..
                    } => {
                        let output = format_leaf(&x).plain_string(100_000);
                        out!("{}", output);
                    }

                    Value {
                        value: UntaggedValue::Primitive(Primitive::Binary(ref b)),
                        ..
                    } => {
                        if let Some(binary) = binary {
                            let mut stream = VecDeque::new();
                            stream.push_back(x);
                            let command_args =
                                create_default_command_args(&context).with_input(stream);
                            let result = binary.run_with_actions(command_args)?;
                            let _ = result.collect::<Vec<_>>();
                        } else {
                            use nu_pretty_hex::*;
                            out!("{:?}", b.hex_dump());
                        }
                    }

                    Value {
                        value: UntaggedValue::Error(e),
                        ..
                    } => {
                        return Err(e);
                    }

                    Value {
                        value: UntaggedValue::Row(ref row),
                        ..
                    } => {
                        let pivot_mode = configuration.pivot_mode();

                        let term_width = context.host.lock().width();
                        if pivot_mode.is_always()
                            || (pivot_mode.is_auto()
                                && (row
                                    .entries
                                    .iter()
                                    .map(|(_, v)| v.convert_to_string())
                                    .collect::<Vec<_>>()
                                    .iter()
                                    .fold(0usize, |acc, len| acc + len.len())
                                    + row.entries.iter().count() * 2)
                                    > term_width)
                        {
                            let mut entries = vec![];
                            for (key, value) in row.entries.iter() {
                                entries.push(vec![
                                    nu_table::StyledString::new(
                                        key.to_string(),
                                        TextStyle::new()
                                            .alignment(nu_table::Alignment::Left)
                                            .fg(nu_ansi_term::Color::Green)
                                            .bold(Some(true)),
                                    ),
                                    nu_table::StyledString::new(
                                        format_leaf(value).plain_string(100_000),
                                        nu_table::TextStyle::basic_left(),
                                    ),
                                ]);
                            }

                            let color_hm = get_color_config(&configuration);

                            let table =
                                nu_table::Table::new(vec![], entries, nu_table::Theme::compact());

                            println!("{}", nu_table::draw_table(&table, term_width, &color_hm));
                        } else if let Some(table) = table {
                            let mut stream = VecDeque::new();
                            stream.push_back(x);
                            let command_args =
                                create_default_command_args(&context).with_input(stream);
                            let result = table.run(command_args)?;
                            let _ = result.collect::<Vec<_>>();
                        } else {
                            out!("{:?}", row);
                        }
                    }
                    Value {
                        value: UntaggedValue::Dataframe(df),
                        ..
                    } => {
                        if let Some(table) = table {
                            // TODO. Configure the parameter rows from file. It can be
                            // adjusted to see a certain amount of values in the head
                            let command_args =
                                create_default_command_args(&context).with_input(df.print()?);
                            let result = table.run(command_args)?;
                            let _ = result.collect::<Vec<_>>();
                        }
                    }
                    Value {
                        value: UntaggedValue::Primitive(Primitive::Nothing),
                        ..
                    } => {
                        // Do nothing
                    }
                    Value {
                        value: ref item, ..
                    } => {
                        if let Some(table) = table {
                            let mut stream = VecDeque::new();
                            stream.push_back(x);
                            let command_args =
                                create_default_command_args(&context).with_input(stream);
                            let result = table.run(command_args)?;
                            let _ = result.collect::<Vec<_>>();
                        } else {
                            out!("{:?}", item);
                        }
                    }
                }
            }
        }
    }

    Ok(InputStream::empty())
}

fn create_default_command_args(context: &RunnableContextWithoutInput) -> RawCommandArgs {
    let span = context.name.span;
    RawCommandArgs {
        host: context.host.clone(),
        ctrl_c: context.ctrl_c.clone(),
        configs: context.configs.clone(),
        current_errors: context.current_errors.clone(),
        shell_manager: context.shell_manager.clone(),
        call_info: UnevaluatedCallInfo {
            args: hir::Call {
                head: Box::new(SpannedExpression::new(
                    Expression::Literal(Literal::String(String::new())),
                    span,
                )),
                positional: None,
                named: None,
                span,
                external_redirection: ExternalRedirection::Stdout,
            },
            name_tag: context.name.clone(),
        },
        scope: Scope::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::Command;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Command {})
    }
}
