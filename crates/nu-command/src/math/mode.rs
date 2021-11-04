use crate::math::utils::run_with_function;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, Value};
use std::cmp::Ordering;

#[derive(Clone)]
pub struct SubCommand;

#[derive(Hash, Eq, PartialEq, Debug)]
enum NumberTypes {
    Float,
    Int,
    Duration,
    Filesize,
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct HashableType {
    bytes: [u8; 8],
    original_type: NumberTypes,
}

impl HashableType {
    fn new(bytes: [u8; 8], original_type: NumberTypes) -> HashableType {
        HashableType {
            bytes,
            original_type,
        }
    }
}

impl Command for SubCommand {
    fn name(&self) -> &str {
        "math mode"
    }

    fn signature(&self) -> Signature {
        Signature::build("math mode")
    }

    fn usage(&self) -> &str {
        "Gets the most frequent element(s) from a list of numbers or tables"
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        run_with_function(call, input, mode)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get the mode(s) of a list of numbers",
            example: "[3 3 9 12 12 15] | math mode",
            result: Some(Value::List {
                vals: vec![Value::test_int(3), Value::test_int(12)],
                span: Span::unknown(),
            }),
        }]
    }
}

pub fn mode(values: &[Value], head: &Span) -> Result<Value, ShellError> {
    if let Some(Err(values)) = values
        .windows(2)
        .map(|elem| {
            if elem[0].partial_cmp(&elem[1]).is_none() {
                return Err(ShellError::OperatorMismatch {
                    op_span: *head,
                    lhs_ty: elem[0].get_type(),
                    lhs_span: elem[0].span()?,
                    rhs_ty: elem[1].get_type(),
                    rhs_span: elem[1].span()?,
                });
            }
            Ok(elem[0].partial_cmp(&elem[1]).unwrap())
        })
        .find(|elem| elem.is_err())
    {
        return Err(values);
    }
    //In e-q, Value doesn't implement Hash or Eq, so we have to get the values inside
    // But f64 doesn't implement Hash, so we get the binary representation to use as
    // key in the HashMap
    let hashable_values: Result<Vec<HashableType>, ShellError> = values
        .iter()
        .map(|val| match val {
            Value::Int { val, .. } => Ok(HashableType::new(val.to_ne_bytes(), NumberTypes::Int)),
            Value::Duration { val, .. } => {
                Ok(HashableType::new(val.to_ne_bytes(), NumberTypes::Duration))
            }
            Value::Float { val, .. } => {
                Ok(HashableType::new(val.to_ne_bytes(), NumberTypes::Float))
            }
            Value::Filesize { val, .. } => {
                Ok(HashableType::new(val.to_ne_bytes(), NumberTypes::Filesize))
            }
            other => Err(ShellError::UnsupportedInput(
                "Unable to give a result with this input".to_string(),
                other.span().unwrap(),
            )),
        })
        .collect::<Result<Vec<HashableType>, ShellError>>();
    if let Err(not_hashable) = hashable_values {
        return Err(not_hashable);
    }

    let mut frequency_map = std::collections::HashMap::new();
    for v in hashable_values.unwrap() {
        let counter = frequency_map.entry(v).or_insert(0);
        *counter += 1;
    }

    let mut max_freq = -1;
    let mut modes = Vec::<Value>::new();
    for (value, frequency) in &frequency_map {
        match max_freq.cmp(frequency) {
            Ordering::Less => {
                max_freq = *frequency;
                modes.clear();
                modes.push(recreate_value(value, *head));
            }
            Ordering::Equal => {
                modes.push(recreate_value(value, *head));
            }
            Ordering::Greater => (),
        }
    }

    modes.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Ok(Value::List {
        vals: modes,
        span: *head,
    })
}

fn recreate_value(hashable_value: &HashableType, head: Span) -> Value {
    let bytes = hashable_value.bytes;
    match &hashable_value.original_type {
        NumberTypes::Int => Value::Int {
            val: i64::from_ne_bytes(bytes),
            span: head,
        },
        NumberTypes::Float => Value::Float {
            val: f64::from_ne_bytes(bytes),
            span: head,
        },
        NumberTypes::Duration => Value::Duration {
            val: i64::from_ne_bytes(bytes),
            span: head,
        },
        NumberTypes::Filesize => Value::Filesize {
            val: i64::from_ne_bytes(bytes),
            span: head,
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
