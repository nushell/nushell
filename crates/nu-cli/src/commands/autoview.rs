use crate::commands::UnevaluatedCallInfo;
use crate::commands::WholeStreamCommand;
use crate::data::value::format_leaf;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::hir::{self, Expression, ExternalRedirection, Literal, SpannedExpression};
use nu_protocol::{Primitive, Scope, Signature, UntaggedValue, Value};
use parking_lot::Mutex;
use std::sync::atomic::AtomicBool;

pub struct Autoview;

#[async_trait]
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

    async fn run(
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
            current_errors: args.current_errors,
            name: args.call_info.name_tag,
            raw_input: args.raw_input,
        })
        .await
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
    pub current_errors: Arc<Mutex<Vec<ShellError>>>,
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
            current_errors: context.current_errors,
            registry: context.registry,
            name: context.name,
        };
        (context.input, new_context)
    }
}

pub async fn autoview(context: RunnableContext) -> Result<OutputStream, ShellError> {
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

    let (mut input_stream, context) = RunnableContextWithoutInput::convert(context);
    let term_width = context.host.lock().width();

    if let Some(x) = input_stream.next().await {
        match input_stream.next().await {
            Some(y) => {
                let ctrl_c = context.ctrl_c.clone();
                let xy = vec![x, y];
                let xy_stream = futures::stream::iter(xy)
                    .chain(input_stream)
                    .interruptible(ctrl_c);

                let stream = InputStream::from_stream(xy_stream);

                if let Some(table) = table {
                    let command_args = create_default_command_args(&context).with_input(stream);
                    let result = table.run(command_args, &context.registry).await?;
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
                            stream.push_back(
                                UntaggedValue::string(s).into_value(Tag { anchor, span }),
                            );
                            let command_args =
                                create_default_command_args(&context).with_input(stream);
                            let result = text.run(command_args, &context.registry).await?;
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
                            stream.push_back(
                                UntaggedValue::string(s).into_value(Tag { anchor, span }),
                            );
                            let command_args =
                                create_default_command_args(&context).with_input(stream);
                            let result = text.run(command_args, &context.registry).await?;
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
                        value: UntaggedValue::Primitive(Primitive::Duration(_)),
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
                            let result = binary.run(command_args, &context.registry).await?;
                            result.collect::<Vec<_>>().await;
                        } else {
                            use pretty_hex::*;
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
                        value: UntaggedValue::Row(row),
                        ..
                    } if pivot_mode == AutoPivotMode::Always
                        || (pivot_mode == AutoPivotMode::Auto
                            && (row
                                .entries
                                .iter()
                                .map(|(_, v)| v.convert_to_string())
                                .collect::<Vec<_>>()
                                .iter()
                                .fold(0usize, |acc, len| acc + len.len())
                                + row.entries.iter().count() * 2)
                                > term_width) =>
                    {
                        let mut entries = vec![];
                        for (key, value) in row.entries.iter() {
                            entries.push(vec![
                                nu_table::StyledString::new(
                                    key.to_string(),
                                    nu_table::TextStyle {
                                        alignment: nu_table::Alignment::Left,
                                        color: Some(ansi_term::Color::Green),
                                        is_bold: true,
                                    },
                                ),
                                nu_table::StyledString::new(
                                    format_leaf(value).plain_string(100_000),
                                    nu_table::TextStyle::basic(),
                                ),
                            ]);
                        }

                        let table =
                            nu_table::Table::new(vec![], entries, nu_table::Theme::compact());

                        nu_table::draw_table(&table, term_width);
                    }

                    Value {
                        value: ref item, ..
                    } => {
                        if let Some(table) = table {
                            let mut stream = VecDeque::new();
                            stream.push_back(x);
                            let command_args =
                                create_default_command_args(&context).with_input(stream);
                            let result = table.run(command_args, &context.registry).await?;
                            result.collect::<Vec<_>>().await;
                        } else {
                            out!("{:?}", item);
                        }
                    }
                }
            }
        }
    }

    Ok(OutputStream::empty())
}

fn create_default_command_args(context: &RunnableContextWithoutInput) -> RawCommandArgs {
    let span = context.name.span;
    RawCommandArgs {
        host: context.host.clone(),
        ctrl_c: context.ctrl_c.clone(),
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
            scope: Scope::new(),
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
