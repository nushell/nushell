use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    merge_descriptors, ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue,
};
use nu_source::{SpannedItem, Tagged};
use nu_value_ext::get_data_by_key;

pub struct Pivot;

#[derive(Deserialize)]
pub struct PivotArgs {
    rest: Vec<Tagged<String>>,
    #[serde(rename(deserialize = "header-row"))]
    header_row: bool,
    #[serde(rename(deserialize = "ignore-titles"))]
    ignore_titles: bool,
}

impl WholeStreamCommand for Pivot {
    fn name(&self) -> &str {
        "pivot"
    }

    fn signature(&self) -> Signature {
        Signature::build("pivot")
            .switch(
                "header-row",
                "treat the first row as column names",
                Some('r'),
            )
            .switch(
                "ignore-titles",
                "don't pivot the column names into values",
                Some('i'),
            )
            .rest(
                SyntaxShape::String,
                "the names to give columns once pivoted",
            )
    }

    fn usage(&self) -> &str {
        "Pivots the table contents so rows become columns and columns become rows."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, pivot)?.run()
    }
}

pub fn pivot(args: PivotArgs, context: RunnableContext) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let input = context.input.into_vec().await;

        let descs = merge_descriptors(&input);

        let mut headers: Vec<String> = vec![];

        if args.rest.len() > 0 && args.header_row {
            yield Err(ShellError::labeled_error("Can not provide header names and use header row", "using header row", context.name));
            return;
        }

        if args.header_row {
            for i in input.clone() {
                if let Some(desc) = descs.get(0) {
                    match get_data_by_key(&i, desc[..].spanned_unknown()) {
                        Some(x) => {
                            if let Ok(s) = x.as_string() {
                                headers.push(s.to_string());
                            } else {
                                yield Err(ShellError::labeled_error("Header row needs string headers", "used non-string headers", context.name));
                                return;
                            }
                        }
                        _ => {
                            yield Err(ShellError::labeled_error("Header row is incomplete and can't be used", "using incomplete header row", context.name));
                            return;
                        }
                    }
                } else {
                    yield Err(ShellError::labeled_error("Header row is incomplete and can't be used", "using incomplete header row", context.name));
                    return;
                }
            }
        } else {
            for i in 0..=input.len() {
                if let Some(name) = args.rest.get(i) {
                    headers.push(name.to_string())
                } else {
                    headers.push(format!("Column{}", i));
                }
            }
        }

        let descs: Vec<_> = if args.header_row {
            descs.iter().skip(1).collect()
        } else {
            descs.iter().collect()
        };

        for desc in descs {
            let mut column_num: usize = 0;
            let mut dict = TaggedDictBuilder::new(&context.name);

            if !args.ignore_titles && !args.header_row {
                dict.insert_untagged(headers[column_num].clone(), UntaggedValue::string(desc.clone()));
                column_num += 1
            }

            for i in input.clone() {
                match get_data_by_key(&i, desc[..].spanned_unknown()) {
                    Some(x) => {
                        dict.insert_value(headers[column_num].clone(), x.clone());
                    }
                    _ => {
                        dict.insert_untagged(headers[column_num].clone(), UntaggedValue::nothing());
                    }
                }
                column_num += 1;
            }

            yield ReturnSuccess::value(dict.into_value());
        }


    };

    Ok(OutputStream::new(stream))
}
