use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    merge_descriptors, ColumnPath, ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder,
    UntaggedValue,
};
use nu_source::{SpannedItem, Tagged};
use nu_value_ext::ValueExt;

pub struct Command;

#[derive(Deserialize)]
pub struct Arguments {
    rest: Vec<Tagged<String>>,
}

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "rotate"
    }

    fn signature(&self) -> Signature {
        Signature::build("rotate").rest(
            SyntaxShape::String,
            "the names to give columns once rotated",
        )
    }

    fn usage(&self) -> &str {
        "Rotates the table by 90 degrees clockwise."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        rotate(args).await
    }
}

pub async fn rotate(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let (Arguments { rest }, input) = args.process().await?;

    let input = input.into_vec().await;
    let total_rows = input.len();
    let descs = merge_descriptors(&input);
    let total_descriptors = descs.len();

    let descs = descs.into_iter().rev().collect::<Vec<_>>();

    if total_rows == 0 {
        return Ok(OutputStream::empty());
    }

    let mut headers: Vec<String> = vec![];

    for i in 0..=total_rows {
        headers.push(format!("Column{}", i));
    }

    let first = input[0].clone();

    let name = if first.tag.anchor().is_some() {
        first.tag
    } else {
        name
    };

    let values =
        UntaggedValue::table(&input.into_iter().rev().collect::<Vec<_>>()).into_value(&name);

    let values = nu_data::utils::group(
        &values,
        &Some(Box::new(move |row_number: usize, _| {
            Ok(match headers.get(row_number) {
                Some(name) => name.clone(),
                None => String::new(),
            })
        })),
        &name,
    )?;

    Ok(futures::stream::iter(
        (0..total_descriptors)
            .map(move |row_number| {
                let mut row = TaggedDictBuilder::new(&name);

                for (current_numbered_column, (column_name, _)) in values.row_entries().enumerate()
                {
                    let raw_column_path =
                        format!("{}.0.{}", column_name, descs[row_number]).spanned_unknown();
                    let path = ColumnPath::build(&raw_column_path);

                    match &values.get_data_by_column_path(&path, Box::new(move |_, _, error| error))
                    {
                        Ok(x) => {
                            row.insert_value(
                                rest.get(current_numbered_column)
                                    .map(|c| c.item.clone())
                                    .unwrap_or_else(|| column_name.to_string()),
                                x.clone(),
                            );
                        }
                        Err(_) => {}
                    }
                }

                row.insert_value(
                    rest.get(total_rows)
                        .map(|c| c.item.clone())
                        .unwrap_or_else(|| format!("Column{}", total_rows)),
                    UntaggedValue::string(&descs[row_number]).into_untagged_value(),
                );

                ReturnSuccess::value(row.into_value())
            })
            .rev()
            .collect::<Vec<_>>(),
    )
    .to_output_stream())
}
