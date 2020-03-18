use crate::commands::UnevaluatedCallInfo;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_parser::{hir, hir::Expression, hir::Literal, hir::SpannedExpression};
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue, Value};
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
            commands: registry.clone(),
            shell_manager: args.shell_manager,
            host: args.host,
            source: args.call_info.source,
            ctrl_c: args.ctrl_c,
            name: args.call_info.name_tag,
        })
    }
}

pub struct RunnableContextWithoutInput {
    pub shell_manager: ShellManager,
    pub host: Arc<parking_lot::Mutex<Box<dyn Host>>>,
    pub source: Text,
    pub ctrl_c: Arc<AtomicBool>,
    pub commands: CommandRegistry,
    pub name: Tag,
}

impl RunnableContextWithoutInput {
    pub fn convert(context: RunnableContext) -> (InputStream, RunnableContextWithoutInput) {
        let new_context = RunnableContextWithoutInput {
            shell_manager: context.shell_manager,
            host: context.host,
            source: context.source,
            ctrl_c: context.ctrl_c,
            commands: context.commands,
            name: context.name,
        };
        (context.input, new_context)
    }
}

pub fn autoview(context: RunnableContext) -> Result<OutputStream, ShellError> {
    let binary = context.get_command("binaryview");
    let text = context.get_command("textview");
    let table = context.get_command("table");

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
                            let result = table.run(command_args, &context.commands);
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
                                    let result = text.run(command_args, &context.commands);
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
                                    let result = text.run(command_args, &context.commands);
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
                                out!("{}", n);
                            }

                            Value { value: UntaggedValue::Primitive(Primitive::Binary(ref b)), .. } => {
                                if let Some(binary) = binary {
                                    let mut stream = VecDeque::new();
                                    stream.push_back(x);
                                    let command_args = create_default_command_args(&context).with_input(stream);
                                    let result = binary.run(command_args, &context.commands);
                                    result.collect::<Vec<_>>().await;
                                } else {
                                    use pretty_hex::*;
                                    out!("{:?}", b.hex_dump());
                                }
                            }

                            Value { value: UntaggedValue::Error(e), .. } => {
                                yield Err(e);
                            }

                            Value { value: UntaggedValue::Row(row), ..} => {
                                use prettytable::format::{FormatBuilder, LinePosition, LineSeparator};
                                use prettytable::{color, Attr, Cell, Row, Table};
                                use crate::data::value::{format_leaf, style_leaf};

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

                                for (key, value) in row.entries.iter() {
                                    table.add_row(Row::new(vec![Cell::new(key).with_style(Attr::ForegroundColor(color::GREEN)).with_style(Attr::Bold),
                                        Cell::new(&format_leaf(value).plain_string(100_000))]));
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
                                    let result = table.run(command_args, &context.commands);
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
                    Expression::Literal(Literal::String(span)),
                    span,
                )),
                positional: None,
                named: None,
                span,
            },
            source: context.source.clone(),
            name_tag: context.name.clone(),
        },
    }
}
