use crate::commands::WholeStreamCommand;
use crate::data::TaggedDictBuilder;
use crate::errors::ShellError;
use crate::prelude::*;

pub struct GroupBy;

#[derive(Deserialize)]
pub struct GroupByArgs {
    column_name: Tagged<String>,
}

impl WholeStreamCommand for GroupBy {
    fn name(&self) -> &str {
        "group-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("group-by").required("column_name", SyntaxShape::String)
    }

    fn usage(&self) -> &str {
        "Creates a new table with the data from the table rows grouped by the column given."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, group_by)?.run()
    }
}

fn group_by(
    GroupByArgs { column_name }: GroupByArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let values: Vec<Tagged<Value>> = input.values.collect().await;
        let mut groups = indexmap::IndexMap::new();

        for value in values {
            let group_key = value.get_data_by_key(&column_name.item);

            if group_key.is_none() {

                let possibilities = value.data_descriptors();

                let mut possible_matches: Vec<_> = possibilities
                    .iter()
                    .map(|x| (natural::distance::levenshtein_distance(x, &column_name.item), x))
                    .collect();

                possible_matches.sort();

                let err = {
                    if possible_matches.len() > 0 {
                        ShellError::labeled_error(
                            "Unknown column",
                            format!("did you mean '{}'?", possible_matches[0].1),
                            &column_name.tag,)
                    } else {
                        ShellError::labeled_error(
                            "Unknown column",
                            "row does not contain this column",
                            &column_name.tag,
                        )
                    }
                };

                yield Err(err)
            } else {
                let group_key = group_key.unwrap().as_string()?;
                let mut group = groups.entry(group_key).or_insert(vec![]);
                group.push(value);
            }
        }

        let mut out = TaggedDictBuilder::new(name.clone());

        for (k,v) in groups.iter() {
            out.insert(k, Value::table(v));
        }

        yield ReturnSuccess::value(out)
    };

    Ok(stream.to_output_stream())
}
