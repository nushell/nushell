use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use futures_util::pin_mut;
use nu_errors::ShellError;
use nu_protocol::{
    ColumnPath, PathMember, Primitive, ReturnSuccess, ReturnValue, Signature, SyntaxShape,
    TaggedDictBuilder, UnspannedPathMember, UntaggedValue, Value,
};
use nu_source::span_for_spanned_list;
use nu_value_ext::{as_string, get_data_by_column_path};

#[derive(Deserialize)]
struct PickArgs {
    rest: Vec<ColumnPath>,
}

pub struct Pick;

impl WholeStreamCommand for Pick {
    fn name(&self) -> &str {
        "pick"
    }

    fn signature(&self) -> Signature {
        Signature::build("pick").rest(
            SyntaxShape::ColumnPath,
            "the columns to select from the table",
        )
    }

    fn usage(&self) -> &str {
        "Down-select table to only these columns."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, pick)?.run()
    }
}

fn pick(
    PickArgs { rest: mut fields }: PickArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    if fields.is_empty() {
        return Err(ShellError::labeled_error(
            "Pick requires columns to pick",
            "needs parameter",
            name,
        ));
    }

    let member = fields.remove(0);
    let member = vec![member];

    let column_paths = vec![&member, &fields]
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<ColumnPath>>();

    let stream = async_stream! {
        let values = input.values;
        pin_mut!(values);

        let mut empty = true;
        let mut bring_back: indexmap::IndexMap<String, Vec<Value>> = indexmap::IndexMap::new();

        while let Some(value) = values.next().await {
            for path in &column_paths {
                let path_members_span = span_for_spanned_list(path.members().iter().map(|p| p.span));

                let fetcher = get_data_by_column_path(&value, &path, Box::new(move |(obj_source, path_member_tried, error)| {
                    if let PathMember { unspanned: UnspannedPathMember::String(column), .. } = path_member_tried {
                        return ShellError::labeled_error_with_secondary(
                        "No data to fetch.",
                        format!("Couldn't pick column \"{}\"", column),
                        path_member_tried.span,
                        format!("How about exploring it with \"get\"? Check the input is appropiate originating from here"),
                        obj_source.tag.span)
                    }

                    error
                }));


                let field = path.clone();
                let key = as_string(&UntaggedValue::Primitive(Primitive::ColumnPath(field.clone())).into_untagged_value())?;

                match fetcher {
                    Ok(results) => {
                        match results.value {
                            UntaggedValue::Table(records) => {
                                for x in records {
                                    let mut out = TaggedDictBuilder::new(name.clone());
                                    out.insert_untagged(&key, x.value.clone());
                                    let group = bring_back.entry(key.clone()).or_insert(vec![]);
                                    group.push(out.into_value());
                                }
                            },
                            x => {
                                let mut out = TaggedDictBuilder::new(name.clone());
                                out.insert_untagged(&key, x.clone());
                                let group = bring_back.entry(key.clone()).or_insert(vec![]);
                                group.push(out.into_value());
                            }

                        }
                    }
                    Err(reason) => {
                        // At the moment, we can't add switches, named flags
                        // and the like while already using .rest since it
                        // breaks the parser.
                        //
                        // We allow flexibility for now and skip the error
                        // if a given column isn't present.
                        let strict: Option<bool> = None;

                        if strict.is_some() {
                            yield Err(reason);
                            return;
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

        let keys = bring_back.keys().map(|x| x.clone()).collect::<Vec<String>>();

        for mut current in 0..max  {
            let mut out = TaggedDictBuilder::new(name.clone());

            for k in &keys {
                let nothing = UntaggedValue::Primitive(Primitive::Nothing).into_untagged_value();
                let subsets = bring_back.get(k);

                match subsets {
                    Some(set) => {
                        match set.get(current) {
                            Some(row) => out.insert_untagged(k, row.get_data(k).borrow().clone()),
                            None => out.insert_untagged(k, nothing.clone()),
                        }
                    }
                    None => out.insert_untagged(k, nothing.clone()),
                }
            }

            yield ReturnSuccess::value(out.into_value());
        }
    };

    let stream: BoxStream<'static, ReturnValue> = stream.boxed();

    Ok(stream.to_output_stream())
}
