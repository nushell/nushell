use ahash::{HashMap, HashMapExt};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    record, Category, Example, IntoPipelineData, LazyRecord, PipelineData, Record, ShellError,
    Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into hash-record"
    }

    fn signature(&self) -> Signature {
        Signature::build("into hash-record")
            .input_output_types(vec![(Type::Record(vec![]), Type::Record(vec![]))])
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert a record into a hashmap"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "record", "hash", "map", "lookup", "table"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        into_record(engine_state, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Create a hash map from a record",
            example: r#"{shell: "Nushell", best: true, uwu: "owo"} | into hash-record"#,
            result: Some(Value::Record {
                val: record!(
                  "shell" => Value::string("Nushell".to_string(), Span::unknown()),
                  "best" => Value::bool(true, Span::unknown()),
                  "uwu" => Value::string("owo".to_string(), Span::unknown()),
                ),
                internal_span: Span::unknown(),
            }),
        }]
    }
}

fn into_record(
    _engine_state: &EngineState,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let input = input.into_value(call.head);
    let span = input.span();

    let map = match input {
        Value::Record { val, .. } => {
            Value::lazy_record(Box::new(NuHashMapRecord::from_record(val, span)), span)
        }
        _ => Value::error(
            ShellError::CantConvert {
                from_type: input.get_type().to_string(),
                to_type: "record".to_string(),
                span,
                help: None,
            },
            span,
        ),
    };
    Ok(map.into_pipeline_data())
}

#[derive(Clone)]
pub struct NuHashMapRecord {
    hash_map: HashMap<String, Value>,
    cols: Vec<String>,
    span: Span,
}

impl std::fmt::Debug for NuHashMapRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NuLazyRecord").finish()
    }
}

impl NuHashMapRecord {
    fn from_record(record: Record, span: Span) -> Self {
        let len = record.cols.len();
        let mut hash_map: HashMap<String, Value> = HashMap::with_capacity(len);
        for i in 0..len {
            hash_map.insert(record.cols[i].clone(), record.vals[i].clone());
        }

        Self {
            hash_map,
            cols: record.cols,
            span,
        }
    }
}

impl<'a> LazyRecord<'a> for NuHashMapRecord {
    fn column_names(&'a self) -> Vec<&'a str> {
        self.cols.iter().map(|col| col.as_str()).collect()
    }

    fn get_column_value(&self, column: &str) -> Result<Value, ShellError> {
        match self.hash_map.get(column) {
            Some(value) => Ok(value.clone()),
            None => Err(ShellError::CantFindColumn {
                col_name: column.to_string(),
                span: self.span,
                src_span: self.span,
            }),
        }
    }

    fn get_column_value_opt(&self, column: &str) -> Option<Result<Value, ShellError>> {
        self.hash_map.get(column).map(|value| Ok(value.clone()))
    }

    fn span(&self) -> Span {
        self.span
    }

    fn clone_value(&self, span: Span) -> Value {
        Value::lazy_record(Box::new(self.clone()), span)
    }

    fn collect(&'a self) -> Result<Value, ShellError> {
        let len = self.hash_map.len();

        let mut cols: Vec<String> = Vec::with_capacity(len);
        let mut vals: Vec<Value> = Vec::with_capacity(len);

        for (col, val) in self.hash_map.iter() {
            cols.push(col.to_string());
            vals.push(val.clone());
        }

        Ok(Value::record(Record { cols, vals }, self.span))
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
