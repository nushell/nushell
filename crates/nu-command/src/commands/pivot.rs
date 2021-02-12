use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    merge_descriptors, ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue,
};
use nu_source::{SpannedItem, Tagged};
use nu_value_ext::ValueExt;

pub struct Pivot;

#[derive(Deserialize)]
pub struct PivotArgs {
    rest: Vec<Tagged<String>>,
    #[serde(rename(deserialize = "header-row"))]
    header_row: bool,
    #[serde(rename(deserialize = "ignore-titles"))]
    ignore_titles: bool,
}

#[async_trait]
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        pivot(args).await
    }
}

pub async fn pivot(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let (args, input): (PivotArgs, _) = args.process().await?;
    let input = input.into_vec().await;

    let descs = merge_descriptors(&input);

    let mut headers: Vec<String> = vec![];

    if !args.rest.is_empty() && args.header_row {
        return Err(ShellError::labeled_error(
            "Can not provide header names and use header row",
            "using header row",
            name,
        ));
    }

    if args.header_row {
        for i in input.clone() {
            if let Some(desc) = descs.get(0) {
                match &i.get_data_by_key(desc[..].spanned_unknown()) {
                    Some(x) => {
                        if let Ok(s) = x.as_string() {
                            headers.push(s.to_string());
                        } else {
                            return Err(ShellError::labeled_error(
                                "Header row needs string headers",
                                "used non-string headers",
                                name,
                            ));
                        }
                    }
                    _ => {
                        return Err(ShellError::labeled_error(
                            "Header row is incomplete and can't be used",
                            "using incomplete header row",
                            name,
                        ));
                    }
                }
            } else {
                return Err(ShellError::labeled_error(
                    "Header row is incomplete and can't be used",
                    "using incomplete header row",
                    name,
                ));
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
        descs.into_iter().skip(1).collect()
    } else {
        descs
    };

    Ok(futures::stream::iter(descs.into_iter().map(move |desc| {
        let mut column_num: usize = 0;
        let mut dict = TaggedDictBuilder::new(&name);

        if !args.ignore_titles && !args.header_row {
            dict.insert_untagged(
                headers[column_num].clone(),
                UntaggedValue::string(desc.clone()),
            );
            column_num += 1
        }

        for i in input.clone() {
            match &i.get_data_by_key(desc[..].spanned_unknown()) {
                Some(x) => {
                    dict.insert_value(headers[column_num].clone(), x.clone());
                }
                _ => {
                    dict.insert_untagged(headers[column_num].clone(), UntaggedValue::nothing());
                }
            }
            column_num += 1;
        }

        ReturnSuccess::value(dict.into_value())
    }))
    .to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Pivot;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Pivot {})
    }
}
