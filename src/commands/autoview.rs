use crate::commands::{RawCommandArgs, WholeStreamCommand};
use crate::errors::ShellError;
use crate::parser::hir::{Expression, NamedArguments};
use crate::prelude::*;
use futures::stream::TryStreamExt;
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
                        if let Some(table) = table {
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

                                let input: Vec<Tagged<Value>> = new_input.into();

                                if input.len() > 0 && input.iter().all(|value| value.is_error()) {
                                    let first = &input[0];

                                    let mut host = context.host.clone();
                                    let mut host = match host.lock() {
                                        Err(err) => {
                                            errln!("Unexpected error acquiring host lock: {:?}", err);
                                            return;
                                        }
                                        Ok(val) => val
                                    };

                                    crate::cli::print_err(first.item.expect_error(), &*host, &context.source);
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
                                Tagged {
                                    item: Value::Primitive(Primitive::String(ref s)),
                                    tag: Tag { anchor, span },
                                } if anchor.is_some() => {
                                    if let Some(text) = text {
                                        let mut stream = VecDeque::new();
                                        stream.push_back(Value::string(s).tagged(Tag { anchor, span }));
                                        let result = text.run(raw.with_input(stream.into()), &context.commands);
                                        result.collect::<Vec<_>>().await;
                                    } else {
                                        outln!("{}", s);
                                    }
                                }
                                Tagged {
                                    item: Value::Primitive(Primitive::String(s)),
                                    ..
                                } => {
                                    outln!("{}", s);
                                }

                                Tagged { item: Value::Primitive(Primitive::Binary(ref b)), .. } => {
                                    if let Some(binary) = binary {
                                        let mut stream = VecDeque::new();
                                        stream.push_back(x.clone());
                                        let result = binary.run(raw.with_input(stream.into()), &context.commands);
                                        result.collect::<Vec<_>>().await;
                                    } else {
                                        use pretty_hex::*;
                                        outln!("{:?}", b.hex_dump());
                                    }
                                }

                                Tagged { item: Value::Error(e), .. } => {
                                    yield Err(e);
                                }
                                Tagged { item: ref item, .. } => {
                                    if let Some(table) = table {
                                        let mut stream = VecDeque::new();
                                        stream.push_back(x.clone());
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
            yield ReturnSuccess::value(Value::nothing().tagged_unknown());
        }
    }))
}
