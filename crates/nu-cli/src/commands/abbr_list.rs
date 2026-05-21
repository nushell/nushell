use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct AbbreviationsList;

impl Command for AbbreviationsList {
    fn name(&self) -> &str {
        "abbr list"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .category(Category::Platform)
    }

    fn description(&self) -> &str {
        "List all defined abbreviations."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "List all abbreviations",
            example: "abbr list",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let config = stack.get_config(engine_state);
        let span = call.head;

        let mut abbreviations: Vec<(&String, &String)> = config.abbreviations.iter().collect();
        abbreviations.sort_by_key(|(k, _)| *k);

        let records = abbreviations
            .into_iter()
            .map(|(name, expansion)| {
                Value::record(
                    record! {
                        "name" => Value::string(name, span),
                        "expansion" => Value::string(expansion, span),
                    },
                    span,
                )
            })
            .collect();

        Ok(Value::list(records, span).into_pipeline_data())
    }
}
