use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct JobList;

impl Command for JobList {
    fn name(&self) -> &str {
        "job list"
    }

    fn description(&self) -> &str {
        "List background jobs"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("job list")
            .category(Category::Experimental)
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["background", "jobs"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        // TODO: proper mutex error handling.
        let jobs = engine_state.jobs.lock().unwrap();

        let values = jobs
            .iter()
            .map(|(id, _job)| {
                let mut record = Record::new();
                record.push("id", Value::int(id as i64, head));

                Value::record(record, head)
            })
            .collect::<Vec<Value>>();

        Ok(Value::list(values, head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}
