use crate::commands::WholeStreamCommand;
use crate::data::Value;
use crate::errors::ShellError;
use crate::prelude::*;

pub struct Get;

#[derive(Deserialize)]
pub struct GetArgs {
    member: Tagged<String>,
    rest: Vec<Tagged<String>>,
}

impl WholeStreamCommand for Get {
    fn name(&self) -> &str {
        "get"
    }

    fn signature(&self) -> Signature {
        Signature::build("get")
            .required("member", SyntaxShape::Member)
            .rest(SyntaxShape::Member)
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

fn get_member(path: &Tagged<String>, obj: &Tagged<Value>) -> Result<Tagged<Value>, ShellError> {
    let mut current = Some(obj);
    for p in path.split(".") {
        if let Some(obj) = current {
            current = match obj.get_data_by_key(p) {
                Some(v) => Some(v),
                None =>
                // Before we give up, see if they gave us a path that matches a field name by itself
                {
                    match obj.get_data_by_key(&path.item) {
                        Some(v) => return Ok(v.clone()),
                        None => {
                            let possibilities = obj.data_descriptors();

                            let mut possible_matches: Vec<_> = possibilities
                                .iter()
                                .map(|x| {
                                    (natural::distance::levenshtein_distance(x, &path.item), x)
                                })
                                .collect();

                            possible_matches.sort();

                            return Err(ShellError::labeled_error(
                                "Unknown column",
                                format!("did you mean '{}'?", possible_matches[0].1),
                                path.tag(),
                            ));
                        }
                    }
                }
            }
        }
    }

    match current {
        Some(v) => Ok(v.clone()),
        None => Ok(Value::nothing().tagged(obj.tag)),
    }
}

pub fn get(
    GetArgs {
        member,
        rest: fields,
    }: GetArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = input
        .values
        .map(move |item| {
            let mut result = VecDeque::new();

            let member = vec![member.clone()];

            let fields = vec![&member, &fields]
                .into_iter()
                .flatten()
                .collect::<Vec<&Tagged<String>>>();

            for field in &fields {
                match get_member(field, &item) {
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
