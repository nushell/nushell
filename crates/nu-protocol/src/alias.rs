use crate::engine::{EngineState, Stack};
use crate::PipelineData;
use crate::{
    ast::{Call, Expression},
    engine::Command,
    BlockId, Example, ShellError, Signature,
};
use std::path::PathBuf;

#[derive(Clone)]
pub struct Alias {
    pub name: String,
    pub command: Option<Box<dyn Command>>, // None if external call
    pub wrapped_call: Expression,
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
        if let Some(cmd) = &self.command {
            cmd.usage()
        } else {
            "This alias wraps an unknown external command."
        }
    }

    fn extra_usage(&self) -> &str {
        if let Some(cmd) = &self.command {
            cmd.extra_usage()
        } else {
            ""
        }
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

    fn examples(&self) -> Vec<Example> {
        if let Some(cmd) = &self.command {
            cmd.examples()
        } else {
            vec![]
        }
    }

    fn is_builtin(&self) -> bool {
        if let Some(cmd) = &self.command {
            cmd.is_builtin()
        } else {
            false
        }
    }

    fn is_known_external(&self) -> bool {
        if let Some(cmd) = &self.command {
            cmd.is_known_external()
        } else {
            false
        }
    }

    fn is_alias(&self) -> bool {
        true
    }

    fn as_alias(&self) -> Option<&Alias> {
        Some(self)
    }

    fn is_custom_command(&self) -> bool {
        if let Some(cmd) = &self.command {
            cmd.is_custom_command()
        } else if self.get_block_id().is_some() {
            true
        } else {
            self.is_known_external()
        }
    }

    fn is_sub(&self) -> bool {
        if let Some(cmd) = &self.command {
            cmd.is_sub()
        } else {
            self.name().contains(' ')
        }
    }

    fn is_parser_keyword(&self) -> bool {
        if let Some(cmd) = &self.command {
            cmd.is_parser_keyword()
        } else {
            false
        }
    }

    fn is_plugin(&self) -> Option<(&PathBuf, &Option<PathBuf>)> {
        if let Some(cmd) = &self.command {
            cmd.is_plugin()
        } else {
            None
        }
    }

    fn get_block_id(&self) -> Option<BlockId> {
        if let Some(cmd) = &self.command {
            cmd.get_block_id()
        } else {
            None
        }
    }

    fn search_terms(&self) -> Vec<&str> {
        if let Some(cmd) = &self.command {
            cmd.search_terms()
        } else {
            vec![]
        }
    }
}
