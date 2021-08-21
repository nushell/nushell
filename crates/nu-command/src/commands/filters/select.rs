use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    ColumnPath, PathMember, Primitive, Signature, SyntaxShape, TaggedDictBuilder,
    UnspannedPathMember, UntaggedValue, Value,
};
use nu_value_ext::{as_string, get_data_by_column_path};

pub struct Command;

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "select"
    }

    fn signature(&self) -> Signature {
        Signature::build("select").rest(
            "rest",
            SyntaxShape::ColumnPath,
            "the columns to select from the table",
        )
    }

    fn usage(&self) -> &str {
        "Down-select table to only these columns."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let columns: Vec<ColumnPath> = args.rest(0)?;
        let input = args.input;
        let name = args.call_info.name_tag;

        select(name, columns, input)
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

fn select(
    name: Tag,
    columns: Vec<ColumnPath>,
    input: InputStream,
) -> Result<OutputStream, ShellError> {
    if columns.is_empty() {
        return Err(ShellError::labeled_error(
            "Select requires columns to select",
            "needs parameter",
            name,
        ));
    }

    let mut bring_back: indexmap::IndexMap<String, Vec<Value>> = indexmap::IndexMap::new();

    for value in input {
        for path in &columns {
            let fetcher = get_data_by_column_path(
                &value,
                path,
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

                    // No value for column 'key' found, insert nothing to make sure all rows contain all keys.
                    bring_back
                        .entry(key.clone())
                        .or_insert(vec![])
                        .push(UntaggedValue::nothing().into());
                }
            }
        }
    }

    let mut max = 0;

    if let Some(max_column) = bring_back.values().max() {
        max = max_column.len();
    }

    let keys = bring_back.keys().cloned().collect::<Vec<String>>();

    Ok(((0..max).map(move |current| {
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

        out.into_value()
    }))
    .into_output_stream())
}

#[cfg(test)]
mod tests {
    use nu_protocol::ColumnPath;
    use nu_source::Span;
    use nu_source::SpannedItem;
    use nu_source::Tag;
    use nu_stream::InputStream;
    use nu_test_support::value::nothing;
    use nu_test_support::value::row;
    use nu_test_support::value::string;

    use super::select;
    use super::Command;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Command {})
    }

    #[test]
    fn select_using_sparse_table() {
        // Create a sparse table with 3 rows:
        //   col_foo | col_bar
        //   -----------------
        //   foo     |
        //           | bar
        //   foo     |
        let input = vec![
            row(indexmap! {"col_foo".into() => string("foo")}),
            row(indexmap! {"col_bar".into() => string("bar")}),
            row(indexmap! {"col_foo".into() => string("foo")}),
        ];

        let expected = vec![
            row(
                indexmap! {"col_none".into() => nothing(), "col_foo".into() => string("foo"), "col_bar".into() => nothing()},
            ),
            row(
                indexmap! {"col_none".into() => nothing(), "col_foo".into() => nothing(), "col_bar".into() => string("bar")},
            ),
            row(
                indexmap! {"col_none".into() => nothing(), "col_foo".into() => string("foo"), "col_bar".into() => nothing()},
            ),
        ];

        let actual = select(
            Tag::unknown(),
            vec![
                ColumnPath::build(&"col_none".to_string().spanned(Span::unknown())),
                ColumnPath::build(&"col_foo".to_string().spanned(Span::unknown())),
                ColumnPath::build(&"col_bar".to_string().spanned(Span::unknown())),
            ],
            input.into(),
        );

        assert_eq!(Ok(expected), actual.map(InputStream::into_vec));
    }
}
