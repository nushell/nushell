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

        for row in values {
            let key = row.get_data_by_key(&column_name.item).unwrap().as_string()?;
            let mut group = groups.entry(key).or_insert(vec![]);
            group.push(row);
        }

        let mut out = TaggedDictBuilder::new(name.clone());

        for (k,v) in groups.iter() {
            out.insert(k, Value::table(v));
        }

        yield ReturnSuccess::value(out)
    };

    Ok(stream.to_output_stream())
}
