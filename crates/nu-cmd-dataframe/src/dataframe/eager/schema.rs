use crate::dataframe::values::NuDataFrame;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SchemaDF;

impl Command for SchemaDF {
    fn name(&self) -> &str {
        "dfr schema"
    }

    fn usage(&self) -> &str {
        "Show schema for a dataframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .switch("datatype-list", "creates a lazy dataframe", Some('l'))
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Dataframe schema",
            example: r#"[[a b]; [1 "foo"] [3 "bar"]] | dfr into-df | dfr schema"#,
            result: Some(Value::record(
                record! {
                    "a" => Value::string("i64", Span::test_data()),
                    "b" => Value::string("str", Span::test_data()),
                },
                Span::test_data(),
            )),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        if call.has_flag(engine_state, stack, "datatype-list")? {
            Ok(PipelineData::Value(datatype_list(Span::unknown()), None))
        } else {
            command(engine_state, stack, call, input)
        }
    }
}

fn command(
    _engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline(input, call.head)?;
    let schema = df.schema();
    let value: Value = schema.into();
    Ok(PipelineData::Value(value, None))
}

fn datatype_list(span: Span) -> Value {
    let types: Vec<Value> = [
        ("null", ""),
        ("bool", ""),
        ("u8", ""),
        ("u16", ""),
        ("u32", ""),
        ("u64", ""),
        ("i8", ""),
        ("i16", ""),
        ("i32", ""),
        ("i64", ""),
        ("f32", ""),
        ("f64", ""),
        ("str", ""),
        ("binary", ""),
        ("date", ""),
        ("datetime<time_unit: (ms, us, ns) timezone (optional)>", "Time Unit can be: milliseconds: ms, microseconds: us, nanoseconds: ns. Timezone wildcard is *. Other Timezone examples: UTC, America/Los_Angeles."),
        ("duration<time_unit: (ms, us, ns)>", "Time Unit can be: milliseconds: ms, microseconds: us, nanoseconds: ns."),
        ("time", ""),
        ("object", ""),
        ("unknown", ""),
        ("list<dtype>", ""),
    ]
    .iter()
    .map(|(dtype, note)| {
        Value::record(record! {
            "dtype" => Value::string(*dtype, span),
            "note" => Value::string(*note, span),
        },
        span)
    })
    .collect();
    Value::list(types, span)
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(SchemaDF {})])
    }
}
