use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Value,
};
use sysinfo::{ProcessExt, System, SystemExt};

#[derive(Clone)]
pub struct Ps;

impl Command for Ps {
    fn name(&self) -> &str {
        "ps"
    }

    fn signature(&self) -> Signature {
        Signature::build("ps")
            .desc("View information about system processes.")
            .switch(
                "long",
                "list all available columns for each entry",
                Some('l'),
            )
            .filter()
            .category(Category::System)
    }

    fn usage(&self) -> &str {
        "View information about system processes."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        run_ps(engine_state, call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "List the system processes",
            example: "ps",
            result: None,
        }]
    }
}

fn run_ps(engine_state: &EngineState, call: &Call) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let long = call.has_flag("long");
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut output = vec![];

    let result: Vec<_> = sys.processes().iter().map(|x| *x.0).collect();

    for pid in result {
        if let Some(result) = sys.process(pid) {
            let mut cols = vec![];
            let mut vals = vec![];

            cols.push("pid".into());
            vals.push(Value::Int {
                val: pid as i64,
                span,
            });

            cols.push("name".into());
            vals.push(Value::String {
                val: result.name().into(),
                span,
            });

            cols.push("status".into());
            vals.push(Value::String {
                val: format!("{:?}", result.status()),
                span,
            });

            cols.push("cpu".into());
            vals.push(Value::Float {
                val: result.cpu_usage() as f64,
                span,
            });

            cols.push("mem".into());
            vals.push(Value::Filesize {
                val: result.memory() as i64 * 1000,
                span,
            });

            cols.push("virtual".into());
            vals.push(Value::Filesize {
                val: result.virtual_memory() as i64 * 1000,
                span,
            });

            if long {
                cols.push("parent".into());
                if let Some(parent) = result.parent() {
                    vals.push(Value::Int {
                        val: parent as i64,
                        span,
                    });
                } else {
                    vals.push(Value::Nothing { span });
                }

                cols.push("exe".into());
                vals.push(Value::String {
                    val: result.exe().to_string_lossy().to_string(),
                    span,
                });

                cols.push("command".into());
                vals.push(Value::String {
                    val: result.cmd().join(" "),
                    span,
                });
            }

            output.push(Value::Record { cols, vals, span });
        }
    }

    Ok(output
        .into_iter()
        .into_pipeline_data(engine_state.ctrlc.clone()))
}
