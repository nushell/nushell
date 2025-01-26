use nu_engine::command_prelude::*;
use nu_protocol::engine::{Job, JobId};

#[derive(Clone)]
pub struct JobKill;

impl Command for JobKill {
    fn name(&self) -> &str {
        "job kill"
    }

    fn description(&self) -> &str {
        "Kill a background job."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("job kill")
            .category(Category::Experimental)
            .required("id", SyntaxShape::Int, "The process id to kill")
            .allow_variants_without_examples(true)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["halt", "stop", "end", "close"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        let id: i64 = call.req(engine_state, stack, 0)?;

        let id: JobId = id as JobId;

        let jobs = engine_state.jobs.lock().unwrap();

        match jobs.lookup(id) {
            None => return Err(ShellError::NotFound { span: head }),
            Some(job) => kill_job(job),
        };

        Ok(Value::nothing(head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}

fn kill_job(job: &Job) -> () {
    match job {
        Job::ThreadJob { signals } => {
            signals.trigger();
        }

        Job::FrozenJob { .. } => {
            todo!("implement killing frozen jobs");
        }
    }
}
