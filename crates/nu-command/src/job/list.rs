use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    record, Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Type, Value,
};

#[derive(Clone)]
pub struct JobList;

impl Command for JobList {
    fn name(&self) -> &str {
        "job list"
    }

    fn signature(&self) -> Signature {
        Signature::build("job list")
            .input_output_types(vec![(Type::Nothing, Type::Table(vec![]))])
            .category(Category::Job)
    }

    fn usage(&self) -> &str {
        "View information about background jobs."
    }

    fn extra_usage(&self) -> &str {
        ""
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let jobs = engine_state
            .jobs
            .info()
            .into_iter()
            .map(|job| {
                // how should `job.exit_status` be displayed?
                let status = job.status();
                Value::record(
                    record! {
                        "id" => Value::int(job.id as i64, span),
                        "command" => Value::string(job.command, span),
                        "status" => Value::string(status.to_string(), span),
                        "pid" => Value::int(job.pid.into(), span),
                    },
                    span,
                )
            })
            .collect();

        Ok(Value::list(jobs, span).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "List all background jobs",
            example: "job list",
            result: None,
        }]
    }
}
