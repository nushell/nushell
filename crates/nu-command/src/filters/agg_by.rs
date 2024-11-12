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
            .switch(
                "count",
                "Add a count column to count the items grouped and aggregated",
                Some('c'),
            )
            .optional(
                "grouper",
                SyntaxShape::OneOf(vec![
                    SyntaxShape::CellPath,
                    SyntaxShape::Closure(None),
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                ]),
                "The path to the column to group on.",
            )
            .named(
                "sum",
                SyntaxShape::CellPath,
                "Column name to calculate the sum from",
                Some('s'),
            )
            .named(
                "avg",
                SyntaxShape::CellPath,
                "Column name to calculate the average from",
                Some('a'),
            )
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
        vec![
            Example {
                description: "Aggregate the data by the Lead_Studio column, summing the Worldwide_Gross column",
                example: r#"open ~/sample_data/movies.csv | agg-by Lead_Studio --sum Worldwide_Gross"#,
                result: None,
            },
            Example {
                description: "Aggregate the data by the Lead_Studio column, averaging the Worldwide_Gross column",
                example: r#"open ~/sample_data/movies.csv | agg-by Lead_Studio --avg Worldwide_Gross"#,
                result: None,
            },
            Example {
                description: "Aggregate the data by the Lead_Studio column, summing, counting, and averaging the Worldwide_Gross column",
                example: r#"open ~/sample_data/movies.csv | agg-by Lead_Studio --sum Worldwide_Gross --avg Worldwide_Gross --count"#,
                result: None,
            },
        ]
    }
}

pub fn group_by(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head: Span = call.head;
    let grouper: Option<Value> = call.opt(engine_state, stack, 0)?;
    let has_count: bool = call.has_flag(engine_state, stack, "count")?;
    let maybe_sum_column: Option<Value> = call.get_flag(engine_state, stack, "sum")?;
    let maybe_avg_column: Option<Value> = call.get_flag(engine_state, stack, "avg")?;
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

    let value = groups_to_table(
        groups,
        has_count,
        maybe_sum_column,
        maybe_avg_column,
        group_name,
        head,
    );

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
    has_count: bool,
    maybe_sum_column: Option<Value>,
    maybe_avg_column: Option<Value>,
    group_name: String,
    span: Span,
) -> Value {
    // using the groups indexmap, create a record! that contains the group, the count, the sum, and the average as a Value::list
    Value::list(
        groups
            .into_iter()
            .map(|(group, items)| {
                let mut record_map = Record::new();
                record_map.insert(group_name.clone(), Value::string(group.clone(), span));

                if has_count {
                    record_map.insert("count".to_string(), Value::int(items.len() as i64, span));
                }

                if let Some(sum_col) = maybe_sum_column.clone() {
                    let (sum_col_name, sum) = sum_celllpath(sum_col, &items, span, true);
                    record_map.insert(sum_col_name + "_sum", Value::float(sum, span));
                }

                if let Some(avg_col) = maybe_avg_column.clone() {
                    let (avg_col_name, sum) = sum_celllpath(avg_col, &items, span, false);
                    let avg = if !items.is_empty() {
                        sum / items.len() as f64
                    } else {
                        0.0
                    };

                    record_map.insert(avg_col_name + "_avg", Value::float(avg, span));
                }

                Value::record(record_map, span)
            })
            .collect(),
        span,
    )
}

fn sum_celllpath(column: Value, items: &[Value], span: Span, is_sum: bool) -> (String, f64) {
    if let Value::CellPath { val, .. } = column {
        let sum: f64 = items
            .iter()
            .map(|v| {
                v.clone()
                    .follow_cell_path(&val.members, false)
                    .unwrap_or_else(|_| Value::float(0.0, span))
                    .as_float()
                    .unwrap_or(0.0)
            })
            .sum();
        (val.to_column_name(), sum)
    } else {
        eprintln!("sum_col type: {:#?}", column.get_type());
        if is_sum {
            ("sum".to_string(), 0.0f64)
        } else {
            ("avg".to_string(), 0.0f64)
        }
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
