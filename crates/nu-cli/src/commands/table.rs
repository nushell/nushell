use crate::commands::WholeStreamCommand;
use crate::format::TableView;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use std::time::Instant;

const STREAM_PAGE_SIZE: usize = 1000;
const STREAM_TIMEOUT_CHECK_INTERVAL: usize = 100;

pub struct Table;

impl WholeStreamCommand for Table {
    fn name(&self) -> &str {
        "table"
    }

    fn signature(&self) -> Signature {
        Signature::build("table").named(
            "start_number",
            SyntaxShape::Number,
            "row number to start viewing from",
            Some('n'),
        )
    }

    fn usage(&self) -> &str {
        "View the contents of the pipeline as a table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        table(args, registry)
    }
}

fn table(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let mut args = args.evaluate_once(registry)?;
    let mut finished = false;

    let stream = async_stream! {
        let host = args.host.clone();
        let mut start_number = match args.get("start_number") {
            Some(Value { value: UntaggedValue::Primitive(Primitive::Int(i)), .. }) => {
                if let Some(num) = i.to_usize() {
                    num
                } else {
                    yield Err(ShellError::labeled_error("Expected a row number", "expected a row number", &args.args.call_info.name_tag));
                    0
                }
            }
            _ => {
                0
            }
        };

        let mut delay_slot = None;

        while !finished {
            let mut new_input: VecDeque<Value> = VecDeque::new();

            let start_time = Instant::now();
            for idx in 0..STREAM_PAGE_SIZE {
                if let Some(val) = delay_slot {
                    new_input.push_back(val);
                    delay_slot = None;
                } else {
                    match args.input.next().await {
                        Some(a) => {
                            if !new_input.is_empty() {
                                if let Some(descs) = new_input.get(0) {
                                    let descs = descs.data_descriptors();
                                    let compare = a.data_descriptors();
                                    if descs != compare {
                                        delay_slot = Some(a);
                                        break;
                                    } else {
                                        new_input.push_back(a);
                                    }
                                } else {
                                    new_input.push_back(a);
                                }
                            } else {
                                new_input.push_back(a);
                            }
                        }
                        _ => {
                            finished = true;
                            break;
                        }
                    }

                    // Check if we've gone over our buffering threshold
                    if (idx + 1) % STREAM_TIMEOUT_CHECK_INTERVAL == 0 {
                        let end_time = Instant::now();

                        // If we've been buffering over a second, go ahead and send out what we have so far
                        if (end_time - start_time).as_secs() >= 1 {
                            break;
                        }
                    }
                }
            }

            let input: Vec<Value> = new_input.into();

            if input.len() > 0 {
                let mut host = host.lock();
                let view = TableView::from_list(&input, start_number);

                if let Some(view) = view {
                    handle_unexpected(&mut *host, |host| crate::format::print_view(&view, host));
                }
            }

            start_number += input.len();
        }

        // Needed for async_stream to type check
        if false {
            yield ReturnSuccess::value(UntaggedValue::nothing().into_value(Tag::unknown()));
        }
    };

    Ok(OutputStream::new(stream))
}
