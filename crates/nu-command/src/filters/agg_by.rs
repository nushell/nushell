use indexmap::IndexMap;
use nu_engine::{command_prelude::*, ClosureEval};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct AggBy;

impl Command for AggBy {
    fn name(&self) -> &str {
        "agg-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("agg-by")
            .input_output_types(vec![(Type::List(Box::new(Type::Any)), Type::Any)])
            .required(
                "grouper",
                SyntaxShape::OneOf(vec![
                    SyntaxShape::CellPath,
                    SyntaxShape::Closure(None),
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                ]),
                "The path to the column to group on.",
            )
            .required_named(
                "agg-column",
                SyntaxShape::CellPath,
                "Column name to calculate the sum from",
                Some('a'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Splits a list or table into groups, and returns a record containing those groups."
    }

    fn extra_description(&self) -> &str {
        r#"the agg-by command makes some assumptions:
    - if the input data is not a string, the grouper will convert the key to string but the values will remain in their original format. e.g. with bools, "true" and true would be in the same group (see example).
    - datetime is formatted based on your configuration setting. use `format date` to change the format.
    - filesize is formatted based on your configuration setting. use `format filesize` to change the format.
    - some nushell values are not supported, such as closures.
    - agg-by will append _sum and _avg to --sum and --avg column names"#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        group_by(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description:
                "Aggregate the data by the Lead_Studio column, summing the Worldwide_Gross column",
            example: r#"open ~/sample_data/movies.csv | agg-by Lead_Studio --agg-column Worldwide_Gross"#,
            result: None,
        }]
    }
}

pub fn group_by(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head: Span = call.head;
    let grouper: Option<Value> = call.req(engine_state, stack, 0)?;
    let maybe_agg_column: Option<Value> = call.get_flag(engine_state, stack, "agg-column")?;
    let config = engine_state.get_config();

    let values: Vec<Value> = input.into_iter().collect();
    if values.is_empty() {
        return Ok(Value::record(Record::new(), head).into_pipeline_data());
    }

    let groups = match grouper {
        Some(ref grouper) => {
            let span = grouper.span();
            match grouper {
                Value::CellPath { val, .. } => group_cell_path(val.clone(), values, config)?,
                Value::Closure { val, .. } => {
                    group_closure(values, span, *val.clone(), engine_state, stack)?
                }
                _ => {
                    return Err(ShellError::TypeMismatch {
                        err_message: "unsupported grouper type".to_string(),
                        span,
                    })
                }
            }
        }
        None => group_no_grouper(values, config)?,
    };

    let group_name = match grouper {
        Some(Value::CellPath { val, .. }) => val.to_column_name(),
        _ => "group".to_string(),
    };

    let value = groups_to_table(groups, maybe_agg_column, group_name, head);

    Ok(value.into_pipeline_data())
}

fn group_cell_path(
    column_name: CellPath,
    values: Vec<Value>,
    config: &nu_protocol::Config,
) -> Result<IndexMap<String, Vec<Value>>, ShellError> {
    let mut groups = IndexMap::<_, Vec<_>>::new();

    for value in values.into_iter() {
        let key = value
            .clone()
            .follow_cell_path(&column_name.members, false)?;

        if matches!(key, Value::Nothing { .. }) {
            continue; // likely the result of a failed optional access, ignore this value
        }

        let key = key.to_abbreviated_string(config);
        groups.entry(key).or_default().push(value);
    }

    Ok(groups)
}

fn group_no_grouper(
    values: Vec<Value>,
    config: &nu_protocol::Config,
) -> Result<IndexMap<String, Vec<Value>>, ShellError> {
    let mut groups = IndexMap::<_, Vec<_>>::new();

    for value in values.into_iter() {
        let key = value.to_abbreviated_string(config);
        groups.entry(key).or_default().push(value);
    }

    Ok(groups)
}

fn group_closure(
    values: Vec<Value>,
    span: Span,
    closure: Closure,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Result<IndexMap<String, Vec<Value>>, ShellError> {
    let mut groups = IndexMap::<_, Vec<_>>::new();
    let mut closure = ClosureEval::new(engine_state, stack, closure);
    let config = engine_state.get_config();

    for value in values {
        let key = closure
            .run_with_value(value.clone())?
            .into_value(span)?
            .to_abbreviated_string(config);

        groups.entry(key).or_default().push(value);
    }

    Ok(groups)
}

fn groups_to_table(
    groups: IndexMap<String, Vec<Value>>,
    maybe_sum_column: Option<Value>,
    group_name: String,
    span: Span,
) -> Value {
    // using the groups indexmap, create a record! that contains the group, the count, the sum, and the average as a Value::list
    Value::list(
        groups
            .into_iter()
            .map(|(group, items)| {
                let mut record_map = Record::new();
                // add group
                record_map.insert(group_name.clone(), Value::string(group.clone(), span));
                // add count
                record_map.insert("count".to_string(), Value::int(items.len() as i64, span));

                if let Some(sum_col) = maybe_sum_column.clone() {
                    match sum_celllpath(sum_col.clone(), &items, span) {
                        Ok((sum_col_name, sum)) => {
                            // add sum
                            record_map
                                .insert(sum_col_name.clone() + "_sum", Value::float(sum, span));
                            let avg = if !items.is_empty() {
                                sum / items.len() as f64
                            } else {
                                0.0
                            };
                            // add avg
                            record_map.insert(sum_col_name + "_avg", Value::float(avg, span));
                        }
                        Err(err) => {
                            // It seems a little odd to be adding an error to the record
                            record_map.insert("error".to_string(), Value::error(err, span));
                        }
                    }

                    match minmax_celllpath(sum_col, &items, span) {
                        Ok((min_col_name, min, max)) => {
                            // add min
                            record_map
                                .insert(min_col_name.clone() + "_min", Value::float(min, span));
                            // add max
                            record_map.insert(min_col_name + "_max", Value::float(max, span));
                        }
                        Err(err) => {
                            // It seems a little odd to be adding an error to the record
                            record_map.insert("error".to_string(), Value::error(err, span));
                        }
                    }
                }

                Value::record(record_map, span)
            })
            .collect(),
        span,
    )
}

fn sum_celllpath(column: Value, items: &[Value], span: Span) -> Result<(String, f64), ShellError> {
    if let Value::CellPath { val, .. } = column {
        let sum: f64 = items
            .iter()
            .map(|v| {
                v.clone()
                    .follow_cell_path(&val.members, false)
                    .unwrap_or_else(|_| Value::float(0.0, span))
                    .coerce_float()
                    .unwrap_or(0.0)
            })
            .sum();
        Ok((val.to_column_name(), sum))
    } else {
        Err(ShellError::TypeMismatch {
            err_message: format!("Only CellPath's are allowed. Found {}.", column.get_type()),
            span,
        })
    }
}

fn minmax_celllpath(
    column: Value,
    items: &[Value],
    span: Span,
) -> Result<(String, f64, f64), ShellError> {
    if let Value::CellPath { val, .. } = column {
        let collection = items
            .iter()
            .map(|v| {
                v.clone()
                    .follow_cell_path(&val.members, false)
                    .unwrap_or_else(|_| Value::float(0.0, span))
                    .coerce_float()
                    .unwrap_or(0.0)
            })
            .collect::<Vec<f64>>();

        Ok((
            val.to_column_name(),
            *collection
                .iter()
                .min_by(|a, b| a.total_cmp(b))
                .unwrap_or(&0.0),
            *collection
                .iter()
                .max_by(|a, b| a.total_cmp(b))
                .unwrap_or(&0.0),
        ))
    } else {
        Err(ShellError::TypeMismatch {
            err_message: format!("Only CellPath's are allowed. Found {}.", column.get_type()),
            span,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(AggBy {})
    }
}
