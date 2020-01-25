use crate::commands::{RawCommandArgs, WholeStreamCommand};
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue, Value};
use std::sync::atomic::Ordering;

pub struct Autoview;

#[derive(Deserialize)]
pub struct AutoviewArgs {}

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
        Ok(args.process_raw(registry, autoview)?.run())
    }
}

pub fn autoview(
    AutoviewArgs {}: AutoviewArgs,
    context: RunnableContext,
    raw: RawCommandArgs,
) -> Result<OutputStream, ShellError> {
    let binary = context.get_command("binaryview");
    let text = context.get_command("textview");
    let table = context.get_command("table");

    Ok(OutputStream::new(async_stream! {
        let mut input_stream = context.input;

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
                            let mut command_args = raw.with_input(stream);
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
                                    let result = text.run(raw.with_input(stream), &context.commands);
                                    result.collect::<Vec<_>>().await;
                                } else {
                                    outln!("{}", s);
                                }
                            }
                            Value {
                                value: UntaggedValue::Primitive(Primitive::String(s)),
                                ..
                            } => {
                                outln!("{}", s);
                            }
                            Value {
                                value: UntaggedValue::Primitive(Primitive::Line(ref s)),
                                tag: Tag { anchor, span },
                            } if anchor.is_some() => {
                                if let Some(text) = text {
                                    let mut stream = VecDeque::new();
                                    stream.push_back(UntaggedValue::string(s).into_value(Tag { anchor, span }));
                                    let result = text.run(raw.with_input(stream), &context.commands);
                                    result.collect::<Vec<_>>().await;
                                } else {
                                    outln!("{}\n", s);
                                }
                            }
                            Value {
                                value: UntaggedValue::Primitive(Primitive::Line(s)),
                                ..
                            } => {
                                outln!("{}\n", s);
                            }
                            Value {
                                value: UntaggedValue::Primitive(Primitive::Path(s)),
                                ..
                            } => {
                                outln!("{}", s.display());
                            }
                            Value {
                                value: UntaggedValue::Primitive(Primitive::Int(n)),
                                ..
                            } => {
                                outln!("{}", n);
                            }
                            Value {
                                value: UntaggedValue::Primitive(Primitive::Decimal(n)),
                                ..
                            } => {
                                outln!("{}", n);
                            }

                            Value { value: UntaggedValue::Primitive(Primitive::Binary(ref b)), .. } => {
                                if let Some(binary) = binary {
                                    let mut stream = VecDeque::new();
                                    stream.push_back(x);
                                    let result = binary.run(raw.with_input(stream), &context.commands);
                                    result.collect::<Vec<_>>().await;
                                } else {
                                    use pretty_hex::*;
                                    outln!("{:?}", b.hex_dump());
                                }
                            }

                            Value { value: UntaggedValue::Error(e), .. } => {
                                yield Err(e);
                            }
                            Value { value: ref item, .. } => {
                                if let Some(table) = table {
                                    let mut stream = VecDeque::new();
                                    stream.push_back(x);
                                    let result = table.run(raw.with_input(stream), &context.commands);
                                    result.collect::<Vec<_>>().await;
                                } else {
                                    outln!("{:?}", item);
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                //outln!("<no results>");
            }
        }

        // Needed for async_stream to type check
        if false {
            yield ReturnSuccess::value(UntaggedValue::nothing().into_untagged_value());
        }
    }))
}
