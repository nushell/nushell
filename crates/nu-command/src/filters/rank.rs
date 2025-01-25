use crate::sort_utils::{compare_by, compare_values};
use crate::Comparator;
use nu_engine::command_prelude::*;
use nu_protocol::ast::PathMember;
use std::cmp::Ordering;

#[derive(Clone)]
pub struct Rank;

impl Command for Rank {
    fn name(&self) -> &str {
        "rank"
    }

    fn description(&self) -> &str {
        "Compute the rank of a list of numbers."
    }

    fn extra_description(&self) -> &str {
        r#"Rank starts at 1.
        The following are available methods for breaking ties:
        `dense`: Tied elements all take their lowest rank, and the next highest element takes the next rank.
        `average`: The average of the ranks that would have been assigned to all the tied values is assigned to each value.
        `min`: The minimum of the ranks that would have been assigned to all the tied values is assigned to each value. (This is also referred to as “competition” ranking.)
        `max`: The maximum of the ranks that would have been assigned to all the tied values is assigned to each value.
        "#
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Float)),
                ),
                (Type::record(), Type::List(Box::new(Type::Float))),
            ])
            .named(
                "method",
                SyntaxShape::String,
                "Method for breaking ties: dense, average, min, max (default average)",
                Some('m'),
            )
            .switch("reverse", "Rank in reverse order", Some('r'))
            .switch(
                "ignore-case",
                "Rank string-based data case-insensitively",
                Some('i'),
            )
            .switch(
                "values",
                "If input is a single record, rank the record by values; ignored if input is not a single record",
                Some('v'),
            )
            .switch(
                "natural",
                "Rank alphanumeric string-based values naturally (1, 9, 10, 99, 100, ...)",
                Some('n'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Math)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return a rank of a list of values",
                example: "[3 8 9 12 12 12 15] | rank",
                result: Some(Value::test_list(vec![
                    Value::test_float(1f64),
                    Value::test_float(2f64),
                    Value::test_float(3f64),
                    Value::test_float(5f64),
                    Value::test_float(5f64),
                    Value::test_float(5f64),
                    Value::test_float(7f64),
                ])),
            },
            Example {
                description: "Rank in reverse",
                example: "[3 8 9 12 12 15] | rank --reverse",
                result: Some(Value::test_list(vec![
                    Value::test_float(6f64),
                    Value::test_float(5f64),
                    Value::test_float(4f64),
                    Value::test_float(2.5f64),
                    Value::test_float(2.5f64),
                    Value::test_float(1f64),
                ])),
            },
            Example {
                description: "Rank using dense method for breaking ties",
                example: "[3 8 9 12 12 15] | rank --method dense",
                result: Some(Value::test_list(vec![
                    Value::test_float(1f64),
                    Value::test_float(2f64),
                    Value::test_float(3f64),
                    Value::test_float(4f64),
                    Value::test_float(4f64),
                    Value::test_float(5f64),
                ])),
            },
            Example {
                description: "Rank using dense method for breaking ties with case-insensitivity",
                example: "[a A b] | rank --method dense --ignore-case",
                result: Some(Value::test_list(vec![
                    Value::test_float(1f64),
                    Value::test_float(1f64),
                    Value::test_float(2f64),
                ])),
            },
            Example {
                description: "Rank using min method for breaking ties",
                example: "[3 8 9 12 12 15] | rank --method min",
                result: Some(Value::test_list(vec![
                    Value::test_float(1f64),
                    Value::test_float(2f64),
                    Value::test_float(3f64),
                    Value::test_float(4f64),
                    Value::test_float(4f64),
                    Value::test_float(6f64),
                ])),
            },
            Example {
                description: "Rank using max method for breaking ties",
                example: "[3 8 9 12 12 15] | rank --method max",
                result: Some(Value::test_list(vec![
                    Value::test_float(1f64),
                    Value::test_float(2f64),
                    Value::test_float(3f64),
                    Value::test_float(5f64),
                    Value::test_float(5f64),
                    Value::test_float(6f64),
                ])),
            },
            Example {
                description: "Rank arbitrary values",
                example: "[5day 5Kib Five five 5] | rank",
                result: Some(Value::test_list(vec![
                    Value::test_float(5f64),
                    Value::test_float(4f64),
                    Value::test_float(2f64),
                    Value::test_float(3f64),
                    Value::test_float(1f64),
                ])),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        if matches!(input, PipelineData::Empty) {
            return Ok(PipelineData::Empty);
        }

        let method = match call.get_flag::<String>(engine_state, stack, "method")? {
            Some(method) => match method.as_str() {
                "dense" => Ok(RankMethod::Dense),
                "average" => Ok(RankMethod::Average),
                "min" => Ok(RankMethod::Min),
                "max" => Ok(RankMethod::Max),
                val => Err(ShellError::InvalidValue {
                    valid: "dense, average, min, max".into(),
                    actual: val.into(),
                    span: call.head,
                }),
            },
            None => Ok(RankMethod::Average), // default
        }?;

        let reverse = call.has_flag(engine_state, stack, "reverse")?;
        let insensitive = call.has_flag(engine_state, stack, "ignore-case")?;
        let natural = call.has_flag(engine_state, stack, "natural")?;
        let rank_by_value = call.has_flag(engine_state, stack, "values")?;

        let span = input.span().unwrap_or(call.head);

        let ranks = rank(
            input,
            span,
            method,
            reverse,
            insensitive,
            natural,
            rank_by_value,
        )?;

        Ok(ranks.into_pipeline_data())
    }
}

#[derive(Debug, Clone, Copy)]
enum RankMethod {
    Dense,
    Average,
    Min,
    Max,
}

fn rank(
    input: PipelineData,
    span: Span,
    method: RankMethod,
    reverse: bool,
    insensitive: bool,
    natural: bool,
    rank_by_value: bool,
) -> Result<Value, ShellError> {
    let mut indices: Vec<usize>;
    let sort_key: Vec<Value>;

    match input.into_value(span)? {
        Value::Record { val, .. } => {
            let input_pairs: Vec<(String, Value)> = val.into_owned().into_iter().collect();
            indices = (0..input_pairs.len()).collect();

            if rank_by_value {
                sort_key = input_pairs.into_iter().map(|(_, value)| value).collect();
            } else {
                // cast record key as Value::String for consistency
                sort_key = input_pairs
                    .into_iter()
                    .map(|(key, _)| Value::string(key, span))
                    .collect();
            }

            indices.sort_by(compare_by_sort_key(
                &sort_key,
                reverse,
                insensitive,
                natural,
            ));
        }
        val @ Value::List { .. } => {
            let r#type = val.get_type();
            sort_key = val.into_list()?;
            indices = (0..sort_key.len()).collect();

            if let Type::Table(cols) = r#type {
                let mut columns: Vec<Comparator> = cols
                    .iter()
                    .map(|col| vec![PathMember::string(col.0.clone(), false, Span::unknown())])
                    .map(|members| CellPath { members })
                    .map(Comparator::CellPath)
                    .collect();

                // allow the comparator function to indicate error
                // by mutating this option captured by the closure,
                // since sort_by closure must be infallible
                let mut compare_err: Option<ShellError> = None;

                indices.sort_by(|&a, &b| {
                    let ordering = compare_by(
                        &sort_key[a],
                        &sort_key[b],
                        &mut columns,
                        span,
                        insensitive,
                        natural,
                        &mut compare_err,
                    );

                    if reverse {
                        ordering.reverse()
                    } else {
                        ordering
                    }
                });

                if let Some(err) = compare_err {
                    return Err(err);
                }
            } else {
                indices.sort_by(compare_by_sort_key(
                    &sort_key,
                    reverse,
                    insensitive,
                    natural,
                ));
            }
        }
        val @ Value::Nothing { .. } => {
            return Err(ShellError::PipelineEmpty {
                dst_span: val.span(),
            })
        }
        val => {
            return Err(ShellError::PipelineMismatch {
                exp_input_type: "record or list".to_string(),
                dst_span: span,
                src_span: val.span(),
            })
        }
    }

    let mut ranks = vec![0f64; sort_key.len()];

    // resolve tie-breaks
    match method {
        RankMethod::Dense => {
            let mut current_rank = 1;
            let mut prev_value = &sort_key[indices[0]];
            ranks[indices[0]] = current_rank as f64;

            for i in 1..indices.len() {
                if compare_values(&sort_key[indices[i]], prev_value, insensitive, natural)?.is_ne()
                {
                    current_rank += 1;
                }
                ranks[indices[i]] = current_rank as f64;
                prev_value = &sort_key[indices[i]];
            }
        }
        RankMethod::Average => {
            let mut i = 0;
            while i < indices.len() {
                let mut j = i;
                while j < indices.len()
                    && compare_values(
                        &sort_key[indices[j]],
                        &sort_key[indices[i]],
                        insensitive,
                        natural,
                    )?
                    .is_eq()
                {
                    j += 1;
                }
                let rank = (i + j + 1) as f64 / 2f64;
                for k in i..j {
                    ranks[indices[k]] = rank;
                }
                i = j;
            }
        }
        RankMethod::Min => {
            let mut i = 0;
            while i < indices.len() {
                let mut j = i;
                while j < indices.len()
                    && compare_values(
                        &sort_key[indices[j]],
                        &sort_key[indices[i]],
                        insensitive,
                        natural,
                    )?
                    .is_eq()
                {
                    j += 1;
                }
                for k in i..j {
                    // i starts at 0 but rank starts at 1
                    ranks[indices[k]] = i as f64 + 1f64;
                }
                i = j;
            }
        }
        RankMethod::Max => {
            let mut i = 0;
            while i < indices.len() {
                let mut j = i;
                while j < indices.len()
                    && compare_values(
                        &sort_key[indices[j]],
                        &sort_key[indices[i]],
                        insensitive,
                        natural,
                    )?
                    .is_eq()
                {
                    j += 1;
                }
                for k in i..j {
                    ranks[indices[k]] = j as f64;
                }
                i = j;
            }
        }
    }

    Ok(Value::list(
        ranks
            .into_iter()
            .map(|val| Value::float(val, span))
            .collect(),
        span,
    ))
}

fn compare_by_sort_key(
    sort_key: &[Value],
    reverse: bool,
    insensitive: bool,
    natural: bool,
) -> impl FnMut(&usize, &usize) -> Ordering + '_ {
    move |&a, &b| {
        let ordering = compare_values(&sort_key[a], &sort_key[b], insensitive, natural)
            .unwrap_or(Ordering::Equal);

        if reverse {
            ordering.reverse()
        } else {
            ordering
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Rank {})
    }
}
