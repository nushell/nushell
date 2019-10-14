use crate::commands::WholeStreamCommand;
use crate::data::meta::tag_for_tagged_list;
use crate::data::Value;
use crate::errors::ShellError;
use crate::prelude::*;
use log::trace;

pub struct Get;

#[derive(Deserialize)]
pub struct GetArgs {
    member: ColumnPath,
    rest: Vec<ColumnPath>,
}

impl WholeStreamCommand for Get {
    fn name(&self) -> &str {
        "get"
    }

    fn signature(&self) -> Signature {
        Signature::build("get")
            .required("member", SyntaxShape::ColumnPath)
            .rest(SyntaxShape::ColumnPath)
    }

    fn usage(&self) -> &str {
        "Open given cells as text."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, get)?.run()
    }
}

pub type ColumnPath = Vec<Tagged<String>>;

pub fn get_column_path(
    path: &ColumnPath,
    obj: &Tagged<Value>,
) -> Result<Tagged<Value>, ShellError> {
    let mut current = Some(obj);
    for p in path.iter() {
        if let Some(obj) = current {
            current = match obj.get_data_by_key(&p) {
                Some(v) => Some(v),
                None =>
                // Before we give up, see if they gave us a path that matches a field name by itself
                {
                    let possibilities = obj.data_descriptors();

                    let mut possible_matches: Vec<_> = possibilities
                        .iter()
                        .map(|x| (natural::distance::levenshtein_distance(x, &p), x))
                        .collect();

                    possible_matches.sort();

                    if possible_matches.len() > 0 {
                        return Err(ShellError::labeled_error(
                            "Unknown column",
                            format!("did you mean '{}'?", possible_matches[0].1),
                            tag_for_tagged_list(path.iter().map(|p| p.tag())),
                        ));
                    } else {
                        return Err(ShellError::labeled_error(
                            "Unknown column",
                            "row does not contain this column",
                            tag_for_tagged_list(path.iter().map(|p| p.tag())),
                        ));
                    }
                }
            }
        }
    }

    match current {
        Some(v) => Ok(v.clone()),
        None => match obj {
            // If its None check for certain values.
            Tagged {
                item: Value::Primitive(Primitive::String(_)),
                ..
            } => Ok(obj.clone()),
            Tagged {
                item: Value::Primitive(Primitive::Path(_)),
                ..
            } => Ok(obj.clone()),
            _ => Ok(Value::nothing().tagged(&obj.tag)),
        },
    }
}

pub fn get(
    GetArgs {
        member,
        rest: fields,
    }: GetArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    trace!("get {:?} {:?}", member, fields);

    let stream = input
        .values
        .map(move |item| {
            let mut result = VecDeque::new();

            let member = vec![member.clone()];

            let fields = vec![&member, &fields]
                .into_iter()
                .flatten()
                .collect::<Vec<&ColumnPath>>();

            for column_path in &fields {
                match get_column_path(column_path, &item) {
                    Ok(Tagged {
                        item: Value::Table(l),
                        ..
                    }) => {
                        for item in l {
                            result.push_back(ReturnSuccess::value(item.clone()));
                        }
                    }
                    Ok(x) => result.push_back(ReturnSuccess::value(x.clone())),
                    Err(x) => result.push_back(Err(x)),
                }
            }

            result
        })
        .flatten();

    Ok(stream.to_output_stream())
}
