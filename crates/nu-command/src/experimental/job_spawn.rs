use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU32},
        mpsc,
    },
    thread,
};

use nu_engine::{ClosureEvalOnce, command_prelude::*};
use nu_protocol::{
    OutDest, Signals,
    engine::{Closure, CurrentJob, Job, Mailbox, Redirection, ThreadJob},
    report_shell_error,
};

#[derive(Clone)]
pub struct JobSpawn;

impl Command for JobSpawn {
    fn name(&self) -> &str {
        "job spawn"
    }

    fn description(&self) -> &str {
        "Spawn a background job and retrieve its ID."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("job spawn")
            .category(Category::Experimental)
            .input_output_types(vec![(Type::Nothing, Type::Int)])
            .named(
                "tag",
                SyntaxShape::String,
                "An optional description tag for this job",
                Some('t'),
            )
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "The closure to run in another thread.",
            )
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["background", "bg", "&"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        let closure: Closure = call.req(engine_state, stack, 0)?;

        let tag: Option<String> = call.get_flag(engine_state, stack, "tag")?;
        let job_stack = stack.clone();

        let mut job_state = engine_state.clone();
        job_state.is_interactive = false;

        // the new job should have its ctrl-c independent of foreground
        let job_signals = Signals::new(Arc::new(AtomicBool::new(false)));
        job_state.set_signals(job_signals.clone());

        // the new job has a separate process group state for its processes
        job_state.pipeline_externals_state = Arc::new((AtomicU32::new(0), AtomicU32::new(0)));

        job_state.exit_warning_given = Arc::new(AtomicBool::new(false));

        let jobs = job_state.jobs.clone();
        let mut jobs = jobs.lock().expect("jobs lock is poisoned!");

        let (send, recv) = mpsc::channel();

        let id = {
            let thread_job = ThreadJob::new(job_signals, tag, send);

            let id = jobs.add_job(Job::Thread(thread_job.clone()));

            job_state.current_job = CurrentJob {
                id,
                background_thread_job: Some(thread_job),
                mailbox: Arc::new(Mutex::new(Mailbox::new(recv))),
            };

            id
        };

        let result = thread::Builder::new()
            .name(format!("background job {}", id.get()))
            .spawn(move || {
                let mut stack = job_stack.reset_pipes();
                let stack = stack.push_redirection(
                    Some(Redirection::Pipe(OutDest::Null)),
                    Some(Redirection::Pipe(OutDest::Null)),
                );
                ClosureEvalOnce::new_preserve_out_dest(&job_state, &stack, closure)
                    .run_with_input(Value::nothing(head).into_pipeline_data())
                    .and_then(|data| data.drain())
                    .unwrap_or_else(|err| {
                        if !job_state.signals().interrupted() {
                            report_shell_error(&job_state, &err);
                        }
                    });

                {
                    let mut jobs = job_state.jobs.lock().expect("jobs lock is poisoned!");

                    jobs.remove_job(id);
                }
            });

        match result {
            Ok(_) => Ok(Value::int(id.get() as i64, head).into_pipeline_data()),
            Err(err) => {
                jobs.remove_job(id);
                Err(ShellError::Io(IoError::new_with_additional_context(
                    err,
                    call.head,
                    None,
                    "Failed to spawn thread for job",
                )))
            }
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "job spawn { sleep 5sec; rm evidence.pdf }",
            description: "Spawn a background job to do some time consuming work",
            result: None,
        }]
    }

    fn extra_description(&self) -> &str {
        r#"Executes the provided closure in a background thread
and registers this task in the background job table, which can be retrieved with `job list`.

This command returns the job id of the newly created job.
            "#
    }
}
