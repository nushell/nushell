use crate::prelude::*;
use crate::utils::arguments::arguments;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    PathMember, Primitive, ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder,
    UnspannedPathMember, UntaggedValue, Value,
};
use nu_value_ext::{as_string, get_data_by_column_path};

#[derive(Deserialize)]
struct Arguments {
    rest: Vec<Value>,
}

pub struct Command;

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "select"
    }

    fn signature(&self) -> Signature {
        Signature::build("select").rest(SyntaxShape::Any, "the columns to select from the table")
    }

    fn usage(&self) -> &str {
        "Down-select table to only these columns."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        select(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Select just the name column",
                example: "ls | select name",
                result: None,
            },
            Example {
                description: "Select the name and size columns",
                example: "ls | select name size",
                result: None,
            },
        ]
    }
}

async fn select(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let (Arguments { mut rest }, mut input) = args.process().await?;
    let (columns, _) = arguments(&mut rest)?;

    if columns.is_empty() {
        return Err(ShellError::labeled_error(
            "Select requires columns to select",
            "needs parameter",
            name,
        ));
    }

    let mut bring_back: indexmap::IndexMap<String, Vec<Value>> = indexmap::IndexMap::new();

    while let Some(value) = input.next().await {
        for path in &columns {
            let fetcher = get_data_by_column_path(
                &value,
                &path,
                move |obj_source, path_member_tried, error| {
                    if let PathMember {
                        unspanned: UnspannedPathMember::String(column),
                        ..
                    } = path_member_tried
                    {
                        return ShellError::labeled_error_with_secondary(
                        "No data to fetch.",
                        format!("Couldn't select column \"{}\"", column),
                        path_member_tried.span,
                        "How about exploring it with \"get\"? Check the input is appropriate originating from here",
                        obj_source.tag.span);
                    }

                    error
                },
            );

            let field = path.clone();
            let key = as_string(
                &UntaggedValue::Primitive(Primitive::ColumnPath(field.clone()))
                    .into_untagged_value(),
            )?;

            match fetcher {
                Ok(results) => match results.value {
                    UntaggedValue::Table(records) => {
                        for x in records {
                            let mut out = TaggedDictBuilder::new(name.clone());
                            out.insert_untagged(&key, x.value.clone());
                            let group = bring_back.entry(key.clone()).or_insert(vec![]);
                            group.push(out.into_value());
                        }
                    }
                    x => {
                        let mut out = TaggedDictBuilder::new(name.clone());
                        out.insert_untagged(&key, x.clone());
                        let group = bring_back.entry(key.clone()).or_insert(vec![]);
                        group.push(out.into_value());
                    }
                },
                Err(reason) => {
                    // At the moment, we can't add switches, named flags
                    // and the like while already using .rest since it
                    // breaks the parser.
                    //
                    // We allow flexibility for now and skip the error
                    // if a given column isn't present.
                    let strict: Option<bool> = None;

                    if strict.is_some() {
                        return Err(reason);
                    }

                    bring_back.entry(key.clone()).or_insert(vec![]);
                }
            }
        }
    }

    let mut max = 0;

    if let Some(max_column) = bring_back.values().max() {
        max = max_column.len();
    }

    let keys = bring_back.keys().cloned().collect::<Vec<String>>();

    Ok(futures::stream::iter((0..max).map(move |current| {
        let mut out = TaggedDictBuilder::new(name.clone());

        for k in &keys {
            let new_key = k.replace(".", "_");
            let nothing = UntaggedValue::Primitive(Primitive::Nothing).into_untagged_value();
            let subsets = bring_back.get(k);

            match subsets {
                Some(set) => match set.get(current) {
                    Some(row) => out.insert_untagged(new_key, row.get_data(k).borrow().clone()),
                    None => out.insert_untagged(new_key, nothing.clone()),
                },
                None => out.insert_untagged(new_key, nothing.clone()),
            }
        }

        ReturnSuccess::value(out.into_value())
    }))
    .to_output_stream())
}
