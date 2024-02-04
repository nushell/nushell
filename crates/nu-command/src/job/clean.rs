use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};
use nu_system::JobId;

#[derive(Clone)]
pub struct JobClean;

impl Command for JobClean {
    fn name(&self) -> &str {
        "job clean"
    }

    fn signature(&self) -> Signature {
        Signature::build("job clean")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .rest(
                "job ids",
                SyntaxShape::Int,
                "remove jobs with these ids if they have completed",
            )
            .category(Category::Job)
    }

    fn usage(&self) -> &str {
        "Remove completed jobs from the job list."
    }

    fn extra_usage(&self) -> &str {
        "All completed jobs are removed if no job ids are provided."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let ids = call
            .rest(engine_state, stack, 0)?
            .into_iter()
            .map(|v| {
                if let Value::Int { val, .. } = v {
                    if val <= 0 {
                        Err(ShellError::IncorrectValue {
                            msg: "job ids must be positive integers not equal to zero".into(),
                            val_span: v.span(),
                            call_span: call.head,
                        })
                    } else {
                        Ok(val as JobId)
                    }
                } else {
                    Err(ShellError::CantConvert {
                        to_type: "int".into(),
                        from_type: v.get_type().to_string(),
                        span: v.span(),
                        help: None,
                    })
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        if ids.is_empty() {
            engine_state.jobs.clean()
        } else {
            engine_state.jobs.clean_ids(&ids)
        }

        Ok(PipelineData::Empty)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Remove all completed jobs from the job list",
                example: "job clean",
                result: None,
            },
            Example {
                description: "Remove the job with id 1 from the job list if it has completed",
                example: "job clean 1",
                result: None,
            },
        ]
    }
}
