use super::{EngineState, Stack, StateWorkingSet};
use crate::{ast::Call, Alias, BlockId, Example, OutDest, PipelineData, ShellError, Signature};
use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
    Builtin,
    Custom,
    Keyword,
    External,
    Alias,
    Plugin,
}

impl Display for CommandType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            CommandType::Builtin => "built-in",
            CommandType::Custom => "custom",
            CommandType::Keyword => "keyword",
            CommandType::External => "external",
            CommandType::Alias => "alias",
            CommandType::Plugin => "plugin",
        };
        write!(f, "{str}")
    }
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

    /// Used by the parser to run command at parse time
    ///
    /// If a command has `is_const()` set to true, it must also implement this method.
    #[allow(unused_variables)]
    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Err(ShellError::MissingConstEvalImpl { span: call.head })
    }

    fn examples(&self) -> Vec<Example> {
        Vec::new()
    }

    // Related terms to help with command search
    fn search_terms(&self) -> Vec<&str> {
        vec![]
    }

    // Whether can run in const evaluation in the parser
    fn is_const(&self) -> bool {
        false
    }

    // Is a sub command
    fn is_sub(&self) -> bool {
        self.name().contains(' ')
    }

    // If command is a block i.e. def blah [] { }, get the block id
    fn block_id(&self) -> Option<BlockId> {
        None
    }

    // Return reference to the command as Alias
    fn as_alias(&self) -> Option<&Alias> {
        None
    }

    /// The identity of the plugin, if this is a plugin command
    #[cfg(feature = "plugin")]
    fn plugin_identity(&self) -> Option<&crate::PluginIdentity> {
        None
    }

    fn command_type(&self) -> CommandType {
        CommandType::Builtin
    }

    fn is_builtin(&self) -> bool {
        self.command_type() == CommandType::Builtin
    }

    fn is_custom(&self) -> bool {
        self.command_type() == CommandType::Custom
    }

    fn is_keyword(&self) -> bool {
        self.command_type() == CommandType::Keyword
    }

    fn is_known_external(&self) -> bool {
        self.command_type() == CommandType::External
    }

    fn is_alias(&self) -> bool {
        self.command_type() == CommandType::Alias
    }

    fn is_plugin(&self) -> bool {
        self.command_type() == CommandType::Plugin
    }

    fn pipe_redirection(&self) -> (Option<OutDest>, Option<OutDest>) {
        (None, None)
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
