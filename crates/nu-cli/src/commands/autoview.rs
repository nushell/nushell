use crate::commands::UnevaluatedCallInfo;
use crate::commands::WholeStreamCommand;
use crate::data::value::format_leaf;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{hir, hir::Expression, hir::Literal, hir::SpannedExpression};
use nu_protocol::{Primitive, ReturnSuccess, Scope, Signature, UntaggedValue, Value};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

pub struct Autoview;

impl WholeStreamCommand for Autoview {
    fn name(&self) -> &str {
        "autoview"
    }

    fn signature(&self) -> Signature {
        Signature::build("autoview")
    }

    fn usage(&self) -> &str {
        "View the contents of the pipeline as a table or list."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        autoview(RunnableContext {
            input: args.input,
            registry: registry.clone(),
            shell_manager: args.shell_manager,
            host: args.host,
            ctrl_c: args.ctrl_c,
            name: args.call_info.name_tag,
            raw_input: args.raw_input,
        })
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

pub struct RunnableContextWithoutInput {
    pub shell_manager: ShellManager,
    pub host: Arc<parking_lot::Mutex<Box<dyn Host>>>,
    pub ctrl_c: Arc<AtomicBool>,
    pub registry: CommandRegistry,
    pub name: Tag,
}

impl RunnableContextWithoutInput {
    pub fn convert(context: RunnableContext) -> (InputStream, RunnableContextWithoutInput) {
        let new_context = RunnableContextWithoutInput {
            shell_manager: context.shell_manager,
            host: context.host,
            ctrl_c: context.ctrl_c,
            registry: context.registry,
            name: context.name,
        };
        (context.input, new_context)
    }
}

pub fn autoview(context: RunnableContext) -> Result<OutputStream, ShellError> {
    let binary = context.get_command("binaryview");
    let text = context.get_command("textview");
    let table = context.get_command("table");

    #[derive(PartialEq)]
    enum AutoPivotMode {
        Auto,
        Always,
        Never,
    }

    let pivot_mode = crate::data::config::config(Tag::unknown());
    let pivot_mode = if let Some(v) = pivot_mode?.get("pivot_mode") {
        match v.as_string() {
            Ok(m) if m.to_lowercase() == "auto" => AutoPivotMode::Auto,
            Ok(m) if m.to_lowercase() == "always" => AutoPivotMode::Always,
            Ok(m) if m.to_lowercase() == "never" => AutoPivotMode::Never,
            _ => AutoPivotMode::Always,
        }
    } else {
        AutoPivotMode::Always
    };

    Ok(OutputStream::new(async_stream! {
        let (mut input_stream, context) = RunnableContextWithoutInput::convert(context);

        match input_stream.next().await {
            Some(x) => {
                match input_stream.next().await {
                    Some(y) => {
                        let ctrl_c = context.ctrl_c.clone();
                        let stream = async_stream! {
                            yield Ok(x);
                            yield Ok(y);

                            loop {
                                match input_stream.next().await {
                                    Some(z) => {
                                        if ctrl_c.load(Ordering::SeqCst) {
                                            break;
                                        }
                                        yield Ok(z);
                                    }
                                    _ => break,
                                }
                            }
                        };
                        let stream = stream.to_input_stream();

                        if let Some(table) = table {
                            let command_args = create_default_command_args(&context).with_input(stream);
                            let result = table.run(command_args, &context.registry);
                            result.collect::<Vec<_>>().await;
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
                                    stream.push_back(UntaggedValue::string(s).into_value(Tag { anchor, span }));
                                    let command_args = create_default_command_args(&context).with_input(stream);
                                    let result = text.run(command_args, &context.registry);
                                    result.collect::<Vec<_>>().await;
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
                                value: UntaggedValue::Primitive(Primitive::Line(ref s)),
                                tag: Tag { anchor, span },
                            } if anchor.is_some() => {
                                if let Some(text) = text {
                                    let mut stream = VecDeque::new();
                                    stream.push_back(UntaggedValue::string(s).into_value(Tag { anchor, span }));
                                    let command_args = create_default_command_args(&context).with_input(stream);
                                    let result = text.run(command_args, &context.registry);
                                    result.collect::<Vec<_>>().await;
                                } else {
                                    out!("{}\n", s);
                                }
                            }
                            Value {
                                value: UntaggedValue::Primitive(Primitive::Line(s)),
                                ..
                            } => {
                                out!("{}\n", s);
                            }
                            Value {
                                value: UntaggedValue::Primitive(Primitive::Path(s)),
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
                                value: UntaggedValue::Primitive(Primitive::Duration(d)),
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

                            Value { value: UntaggedValue::Primitive(Primitive::Binary(ref b)), .. } => {
                                if let Some(binary) = binary {
                                    let mut stream = VecDeque::new();
                                    stream.push_back(x);
                                    let command_args = create_default_command_args(&context).with_input(stream);
                                    let result = binary.run(command_args, &context.registry);
                                    result.collect::<Vec<_>>().await;
                                } else {
                                    use pretty_hex::*;
                                    out!("{:?}", b.hex_dump());
                                }
                            }

                            Value { value: UntaggedValue::Error(e), .. } => {
                                yield Err(e);
                            }

                            Value { value: UntaggedValue::Row(row), ..}
                                if pivot_mode == AutoPivotMode::Always ||
                                (pivot_mode == AutoPivotMode::Auto &&
                                (row.entries.iter().map(|(k,v)| v.convert_to_string())
                                .collect::<Vec<_>>().iter()
                                .fold(0, |acc, len| acc + len.len())
                                +
                                (row.entries.iter().map(|(k,_)| k.chars()).count() * 2))
                                > textwrap::termwidth()) => {
                                use prettytable::format::{FormatBuilder, LinePosition, LineSeparator};
                                use prettytable::{color, Attr, Cell, Row, Table};
                                use crate::data::value::{format_leaf, style_leaf};
                                use textwrap::fill;

                                let termwidth = std::cmp::max(textwrap::termwidth(), 20);

                                enum TableMode {
                                    Light,
                                    Normal,
                                }

                                let mut table = Table::new();
                                let table_mode = crate::data::config::config(Tag::unknown());

                                let table_mode = if let Some(s) = table_mode?.get("table_mode") {
                                    match s.as_string() {
                                        Ok(typ) if typ == "light" => TableMode::Light,
                                        _ => TableMode::Normal,
                                    }
                                } else {
                                    TableMode::Normal
                                };

                                match table_mode {
                                    TableMode::Light => {
                                        table.set_format(
                                            FormatBuilder::new()
                                                .separator(LinePosition::Title, LineSeparator::new('─', '─', ' ', ' '))
                                                .separator(LinePosition::Bottom, LineSeparator::new(' ', ' ', ' ', ' '))
                                                .padding(1, 1)
                                                .build(),
                                        );
                                    }
                                    _ => {
                                        table.set_format(
                                            FormatBuilder::new()
                                                .column_separator('│')
                                                .separator(LinePosition::Top, LineSeparator::new('─', '┬', ' ', ' '))
                                                .separator(LinePosition::Title, LineSeparator::new('─', '┼', ' ', ' '))
                                                .separator(LinePosition::Bottom, LineSeparator::new('─', '┴', ' ', ' '))
                                                .padding(1, 1)
                                                .build(),
                                        );
                                    }
                                }

                                let mut max_key_len = 0;
                                for (key, _) in row.entries.iter() {
                                    max_key_len = std::cmp::max(max_key_len, key.chars().count());
                                }

                                if max_key_len > (termwidth/2 - 1) {
                                    max_key_len = termwidth/2 - 1;
                                }

                                let max_val_len = termwidth - max_key_len - 5;

                                for (key, value) in row.entries.iter() {
                                    table.add_row(Row::new(vec![Cell::new(&fill(&key, max_key_len)).with_style(Attr::ForegroundColor(color::GREEN)).with_style(Attr::Bold),
                                        Cell::new(&fill(&format_leaf(value).plain_string(100_000), max_val_len))]));
                                }

                                table.printstd();

                                // table.print_term(&mut *context.host.lock().out_terminal().ok_or_else(|| ShellError::untagged_runtime_error("Could not open terminal for output"))?)
                                //     .map_err(|_| ShellError::untagged_runtime_error("Internal error: could not print to terminal (for unix systems check to make sure TERM is set)"))?;
                            }

                            Value { value: ref item, .. } => {
                                if let Some(table) = table {
                                    let mut stream = VecDeque::new();
                                    stream.push_back(x);
                                    let command_args = create_default_command_args(&context).with_input(stream);
                                    let result = table.run(command_args, &context.registry);
                                    result.collect::<Vec<_>>().await;
                                } else {
                                    out!("{:?}", item);
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                //out!("<no results>");
            }
        }

        // Needed for async_stream to type check
        if false {
            yield ReturnSuccess::value(UntaggedValue::nothing().into_untagged_value());
        }
    }))
}

fn create_default_command_args(context: &RunnableContextWithoutInput) -> RawCommandArgs {
    let span = context.name.span;
    RawCommandArgs {
        host: context.host.clone(),
        ctrl_c: context.ctrl_c.clone(),
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
                is_last: true,
            },
            name_tag: context.name.clone(),
            scope: Scope::empty(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::Autoview;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Autoview {})
    }
}
