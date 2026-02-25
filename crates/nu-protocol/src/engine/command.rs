use serde::{Deserialize, Serialize};

use super::{EngineState, Stack, StateWorkingSet};
use crate::{
    Alias, BlockId, DeprecationEntry, DynamicCompletionCallRef, DynamicSuggestion, Example,
    OutDest, PipelineData, ShellError, Signature, Span, Value, engine::Call,
};
use std::{borrow::Cow, fmt::Display};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgType<'a> {
    Flag(Cow<'a, str>),
    Positional(usize),
}

impl<'a> Display for ArgType<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgType::Flag(flag_name) => match flag_name {
                Cow::Borrowed(v) => write!(f, "{v}"),
                Cow::Owned(v) => write!(f, "{v}"),
            },
            ArgType::Positional(idx) => write!(f, "{idx}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

    /// Short preferably single sentence description for the command.
    ///
    /// Will be shown with the completions etc.
    fn description(&self) -> &str;

    /// Longer documentation description, if necessary.
    ///
    /// Will be shown below `description`
    fn extra_description(&self) -> &str {
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

    fn examples(&self) -> Vec<Example<'_>> {
        Vec::new()
    }

    // Related terms to help with command search
    fn search_terms(&self) -> Vec<&str> {
        vec![]
    }

    fn attributes(&self) -> Vec<(String, Value)> {
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

    /// The span of this command's declaration, if available.
    /// Used to look up the source file where the command was declared.
    /// Applicable to any command type that knows its declaration site.
    fn decl_span(&self) -> Option<Span> {
        None
    }

    fn is_alias(&self) -> bool {
        self.command_type() == CommandType::Alias
    }

    fn is_plugin(&self) -> bool {
        self.command_type() == CommandType::Plugin
    }

    fn deprecation_info(&self) -> Vec<DeprecationEntry> {
        vec![]
    }

    fn pipe_redirection(&self) -> (Option<OutDest>, Option<OutDest>) {
        (None, None)
    }

    // engine_state and stack are required to get completion from plugin.
    /// Get completion items for `arg_type`.
    ///
    /// It's useful when you want to get auto completion items of a flag or positional argument
    /// dynamically.
    ///
    /// The implementation can returns 3 types of return values:
    /// - None: I couldn't find any suggestions, please fall back to default completions
    /// - Some(vec![]): there are no suggestions
    /// - Some(vec![item1, item2]): item1 and item2 are available
    #[allow(unused_variables)]
    #[expect(deprecated)]
    fn get_dynamic_completion(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: DynamicCompletionCallRef,
        arg_type: &ArgType,
        _experimental: ExperimentalMarker,
    ) -> Result<Option<Vec<DynamicSuggestion>>, ShellError> {
        Ok(None)
    }

    /// Return true if the AST nodes for the arguments are required for IR evaluation. This is
    /// currently inefficient so is not generally done.
    fn requires_ast_for_arguments(&self) -> bool {
        false
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

/// Marker type for tagging [`Command`] methods as experimental.
///
/// Add this marker as a parameter to a method to make implementors see a deprecation warning when
/// they implement it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[deprecated(note = "this method is very experimental, likely to change")]
pub struct ExperimentalMarker;
