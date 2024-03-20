use crate::{
    ast::{Call, Expression},
    engine::{Command, EngineState, Stack},
    PipelineData, ShellError, Signature,
};

#[derive(Clone)]
pub struct Alias {
    pub name: String,
    pub command: Option<Box<dyn Command>>, // None if external call
    pub wrapped_call: Expression,
    pub usage: String,
    pub extra_usage: String,
}

impl Command for Alias {
    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> Signature {
        if let Some(cmd) = &self.command {
            cmd.signature()
        } else {
            Signature::new(&self.name).allows_unknown_args()
        }
    }

    fn usage(&self) -> &str {
        &self.usage
    }

    fn extra_usage(&self) -> &str {
        &self.extra_usage
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Err(ShellError::NushellFailedSpanned {
            msg: "Can't run alias directly. Unwrap it first".to_string(),
            label: "originates from here".to_string(),
            span: call.head,
        })
    }

    fn is_alias(&self) -> bool {
        true
    }

    fn as_alias(&self) -> Option<&Alias> {
        Some(self)
    }
}
