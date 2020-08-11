use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct Flatten;

#[derive(Deserialize)]
pub struct FlattenArgs {
    column_name: Option<Tagged<String>>,
}

#[async_trait]
impl WholeStreamCommand for Flatten {
    fn name(&self) -> &str {
        "flatten"
    }

    fn signature(&self) -> Signature {
        Signature::build("flatten").required(
            "structure",
            SyntaxShape::String,
            "structure to be flattened",
        )
    }

    fn usage(&self) -> &str {
        "Bring nested data into the current table"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        flatten(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Flatten nested rows",
            example: r#"echo "{"dog_names":{"dog_1":"susan","dog_2":"frank"}}" | from json | flatten dog_names"#,
            result: None,
        }]
    }
}

async fn flatten(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let (FlattenArgs { column_name }, mut input) = args.process(&registry).await?;
    let input_vec = input.drain_vec().await;
    let mut values_vec_deque: VecDeque<Value> = VecDeque::new();
    let column = column_name.unwrap();

    if input_vec.is_empty() {
        return Err(ShellError::labeled_error(
            "Cannot flatten structure in an empty table",
            "cannot flatten structure in an empty table",
            column.tag,
        ));
    }

    let outermost_tag = input_vec[0].tag.clone();

    for outer_value in input_vec.iter() {
        let mut indexmap: IndexMap<String, Value> = IndexMap::new();

        match outer_value.clone() {
            Value {
                value: UntaggedValue::Row(outer_dict),
                ..
            } => {
                if !outer_dict.contains_key(column.as_str()) {
                    return Err(ShellError::labeled_error(
                        "Column name is not in table",
                        "column name is not in table",
                        column.tag,
                    ));
                }
                for outer_dict_key in outer_dict.keys() {
                    let maybe_owned_value_from_innered_value_from_outer =
                        outer_dict.get_data(outer_dict_key.as_str());
                    let value_from_outer = maybe_owned_value_from_innered_value_from_outer.borrow();

                    if column.to_string() == *outer_dict_key {
                        match value_from_outer {
                            Value {
                                value: UntaggedValue::Primitive(_),
                                ..
                            } => {
                                // Maybe this should not be an error, maybe it should pass the table through?
                                // Since flattening a Primitive would be the same as the original primitive...
                                // Same question for the other cases that emit errors...
                                return Err(ShellError::labeled_error(
                                    "Primitives don't require flattening",
                                    "primitives don't require flattening",
                                    column.tag,
                                ));
                            }
                            Value {
                                value: UntaggedValue::Row(inner_dict),
                                ..
                            } => {
                                for inner_dict_key in inner_dict.keys() {
                                    let maybe_owned_value_from_inner =
                                        inner_dict.get_data(inner_dict_key);
                                    let value_from_inner = maybe_owned_value_from_inner.borrow();

                                    indexmap
                                        .insert(inner_dict_key.clone(), value_from_inner.clone());
                                }
                            }
                            Value {
                                value: UntaggedValue::Table(_),
                                ..
                            } => {
                                return Err(ShellError::labeled_error(
                                    "Flatten is not yet implemented for nested tables",
                                    "flatten is not yet implemented for nested tables",
                                    column.tag,
                                ))
                            }
                            Value {
                                value: UntaggedValue::Error(_),
                                ..
                            } => {
                                return Err(ShellError::labeled_error(
                                    "Errors cannot be flattened",
                                    "errors cannot be flattened",
                                    column.tag,
                                ))
                            }
                            Value {
                                value: UntaggedValue::Block(_),
                                ..
                            } => {
                                return Err(ShellError::labeled_error(
                                    "Blocks cannot be flattened",
                                    "blocks cannot be flattened",
                                    column.tag,
                                ))
                            }
                        }
                    } else {
                        indexmap.insert(outer_dict_key.clone(), value_from_outer.clone());
                    }
                }
            }
            _ => {
                return Err(ShellError::labeled_error(
                    "When would this occur?",
                    "when would this occur?",
                    column.tag,
                ))
            }
        }

        values_vec_deque.push_back(
            UntaggedValue::Row(Dictionary::from(indexmap)).into_value(outermost_tag.clone()),
        );
    }

    Ok(futures::stream::iter(values_vec_deque).to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Flatten;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Flatten {})
    }
}

// Examples
// Remove clones
// Get it to work with tables / other nested structures
// Tests
// Rename variables (should I be referring to them as nested "columns" or nested "rows"?)
