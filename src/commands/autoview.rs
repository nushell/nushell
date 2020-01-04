use crate::commands::{RawCommandArgs, WholeStreamCommand};
use crate::prelude::*;
use futures::stream::TryStreamExt;
use nu_errors::ShellError;
use nu_parser::hir::{Expression, NamedArguments};
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue, Value};
use std::sync::atomic::Ordering;

pub struct Autoview;

const STREAM_PAGE_SIZE: u64 = 50;

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
        let mut output_stream: OutputStream = context.input.into();

        let next = output_stream.try_next().await;

        match next {
            Ok(Some(x)) => {
                match output_stream.try_next().await {
                    Ok(Some(y)) => {
                        let ctrl_c = context.ctrl_c.clone();
                        let stream = async_stream! {
                            yield Ok(x);
                            yield Ok(y);

                            loop {
                                match output_stream.try_next().await {
                                    Ok(Some(z)) => {
                                        if ctrl_c.load(Ordering::SeqCst) {
                                            break;
                                        }
                                        yield Ok(z);
                                    }
                                    _ => break,
                                }
                            }
                        };
                        if let Some(table) = table? {
                            let mut new_output_stream: OutputStream = stream.to_output_stream();
                            let mut finished = false;
                            let mut current_idx = 0;
                            loop {
                                let mut new_input = VecDeque::new();

                                for _ in 0..STREAM_PAGE_SIZE {
                                    match new_output_stream.try_next().await {

                                        Ok(Some(a)) => {
                                            if let ReturnSuccess::Value(v) = a {
                                                new_input.push_back(v);
                                            }
                                        }
                                        _ => {
                                            finished = true;
                                            break;
                                        }
                                    }
                                }

                                let raw = raw.clone();

                                let input: Vec<Value> = new_input.into();

                                if input.len() > 0 && input.iter().all(|value| value.value.is_error()) {
                                    let first = &input[0];

                                    let mut host = context.host.clone();
                                    let mut host = host.lock();

                                    crate::cli::print_err(first.value.expect_error(), &*host, &context.source);
                                    return;
                                }

                                let mut command_args = raw.with_input(input);
                                let mut named_args = NamedArguments::new();
                                named_args.insert_optional("start_number", Some(Expression::number(current_idx, Tag::unknown())));
                                command_args.call_info.args.named = Some(named_args);

                                let result = table.run(command_args, &context.commands);
                                result.collect::<Vec<_>>().await;


                                if finished {
                                    break;
                                } else {
                                    current_idx += STREAM_PAGE_SIZE;
                                }
                            }
                        }
                    }
                    _ => {
                        if let ReturnSuccess::Value(x) = x {
                            match x {
                                Value {
                                    value: UntaggedValue::Primitive(Primitive::String(ref s)),
                                    tag: Tag { anchor, span },
                                } if anchor.is_some() => {
                                    if let Some(text) = text? {
                                        let mut stream = VecDeque::new();
                                        stream.push_back(UntaggedValue::string(s).into_value(Tag { anchor, span }));
                                        let result = text.run(raw.with_input(stream.into()), &context.commands);
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
                                    if let Some(text) = text? {
                                        let mut stream = VecDeque::new();
                                        stream.push_back(UntaggedValue::string(s).into_value(Tag { anchor, span }));
                                        let result = text.run(raw.with_input(stream.into()), &context.commands);
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
                                    if let Some(binary) = binary? {
                                        let mut stream = VecDeque::new();
                                        stream.push_back(x);
                                        let result = binary.run(raw.with_input(stream.into()), &context.commands);
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
                                    if let Some(table) = table? {
                                        let mut stream = VecDeque::new();
                                        stream.push_back(x);
                                        let result = table.run(raw.with_input(stream.into()), &context.commands);
                                        result.collect::<Vec<_>>().await;
                                    } else {
                                        outln!("{:?}", item);
                                    }
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
