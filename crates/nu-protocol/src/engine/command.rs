use std::path::PathBuf;

use crate::{ast::Call, BlockId, Example, PipelineData, ShellError, Signature};

use super::{EngineState, Stack};

#[derive(Debug)]
pub enum CommandType {
    Builtin,
    Custom,
    Keyword,
    External,
    Plugin,
    Other,
}

pub trait Command: Send + Sync + CommandClone {
    fn name(&self) -> &str;

    fn signature(&self) -> Signature;

    fn usage(&self) -> &str;

    fn extra_usage(&self) -> &str {
        ""
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError>;

    fn examples(&self) -> Vec<Example> {
        Vec::new()
    }

    // This is a built-in command
    fn is_builtin(&self) -> bool {
        true
    }

    // This is a signature for a known external command
    fn is_known_external(&self) -> bool {
        false
    }

    // This is an enhanced method to determine if a command is custom command or not
    // since extern "foo" [] and def "foo" [] behaves differently
    fn is_custom_command(&self) -> bool {
        if self.get_block_id().is_some() {
            true
        } else {
            self.is_known_external()
        }
    }

    // Is a sub command
    fn is_sub(&self) -> bool {
        self.name().contains(' ')
    }

    // Is a parser keyword (source, def, etc.)
    fn is_parser_keyword(&self) -> bool {
        false
    }

    // Is a plugin command (returns plugin's path, type of shell if the declaration is a plugin)
    fn is_plugin(&self) -> Option<(&PathBuf, &Option<PathBuf>)> {
        None
    }

    // If command is a block i.e. def blah [] { }, get the block id
    fn get_block_id(&self) -> Option<BlockId> {
        None
    }

    // Related terms to help with command search
    fn search_terms(&self) -> Vec<&str> {
        vec![]
    }

    fn command_type(&self) -> CommandType {
        match (
            self.is_builtin(),
            self.is_custom_command(),
            self.is_parser_keyword(),
            self.is_known_external(),
            self.is_plugin().is_some(),
        ) {
            (true, false, false, false, false) => CommandType::Builtin,
            (true, true, false, false, false) => CommandType::Custom,
            (true, false, true, false, false) => CommandType::Keyword,
            (false, true, false, true, false) => CommandType::External,
            (true, false, false, false, true) => CommandType::Plugin,
            _ => CommandType::Other,
        }
    }
}

pub trait CommandClone {
    fn clone_box(&self) -> Box<dyn Command>;
}

impl<T> CommandClone for T
where
    T: 'static + Command + Clone,
{
    fn clone_box(&self) -> Box<dyn Command> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Command> {
    fn clone(&self) -> Box<dyn Command> {
        self.clone_box()
    }
}
