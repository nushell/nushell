use nu_engine::command_prelude::*;
use nu_protocol::engine::{FrozenJob, Job};

#[derive(Clone)]
pub struct JobList;

impl Command for JobList {
    fn name(&self) -> &str {
        "job list"
    }

    fn description(&self) -> &str {
        "List background jobs."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("job list")
            .category(Category::Experimental)
            .input_output_types(vec![(Type::Nothing, Type::table())])
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

        let jobs = engine_state.jobs.lock().expect("jobs lock is poisoned!");

        let values = jobs
            .iter()
            .map(|(id, job)| {
                let mut record = record! {
                    "id" => Value::int(id.get() as i64, head),
                    "type" => match job {
                        Job::Thread(_) => Value::string("thread", head),
                        Job::Frozen(_) => Value::string("frozen", head),
                    },
                    "pids" => match job {
                        Job::Thread(job) => Value::list(
                            job.collect_pids()
                                .into_iter()
                                .map(|it| Value::int(it as i64, head))
                                .collect::<Vec<Value>>(),
                            head,
                        ),

                        Job::Frozen(FrozenJob { unfreeze, .. }) => {
                            Value::list(vec![ Value::int(unfreeze.pid() as i64, head) ], head)
                        }
                    },
                };

                if let Some(tag) = job.tag() {
                    record.push("tag", Value::string(tag, head));
                }

                Value::record(record, head)
            })
            .collect::<Vec<Value>>();

        Ok(Value::list(values, head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "job list",
            description: "List all background jobs",
            result: None,
        }]
    }
}
