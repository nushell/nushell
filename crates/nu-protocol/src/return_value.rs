use crate::hir::Block;
use crate::{SyntaxShape, value::Value};
use nu_errors::ShellError;
use nu_source::{b, DebugDocBuilder, PrettyDebug, Span};
use serde::{Deserialize, Serialize};


//TODO where to move this?
#[derive(Eq, Debug, Clone, Serialize, Deserialize, Hash)]
pub struct VarDeclaration{
    pub name: String,
    // type_decl: Option<UntaggedValue>,
    pub is_var_arg: bool,
    // scope: ?
    // pub tag: Tag, ?
    pub span: Span,
}

impl VarDeclaration{
    pub fn new(name: &str, span: Span) -> VarDeclaration{
        VarDeclaration{
            name: name.to_string(),
            is_var_arg: false,
            span,
        }
    }
}

impl PartialEq<VarDeclaration> for VarDeclaration{
    // When searching through the expressions, only the name of the
    // Variable is available. (TODO And their scope). Their full definition is not available.
    // Therefore the equals relationship is relaxed
    fn eq(&self, other: &VarDeclaration) -> bool {
        // TODO when scripting is available scope has to be respected
        self.name == other.name
            // && self.scope == other.scope
    }
}

//TODO where to move this
//TODO implement iterator for this to iterate on it like a list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarShapeDeduction{
    var_decl: VarDeclaration,
    deduction: SyntaxShape,
    /// Spans pointing to the source of the deduction.
    /// The spans locate positions within the tag of var_decl
    deducted_from: Vec<Span>,
    /// For a command with a signature of:
    /// cmd [optional1] [optional2] <required>
    /// the resulting inference must be:
    /// optional1Shape or optional2Shape or requiredShape
    /// Thats a list of alternative shapes.
    /// This field stores a pointer to the possible next deduction
    alternative: Option<Box<VarShapeDeduction>>,
    /// Whether the variable can be substituted with the SyntaxShape deduction
    /// multiple times.
    /// For example a Var-Arg-Variable must be substituted when used in a cmd with
    /// a signature of:
    /// cmd [optionalPaths...] [integers...]
    /// with 2 SpannedVarShapeDeductions, where each can substitute multiple arguments
    many_of_shapes: bool
}
impl VarShapeDeduction{
    //TODO better naming
    pub fn from_usage(var_name: &str, deduced_from: &Span, deduced_shape: &SyntaxShape) -> VarShapeDeduction{
        VarShapeDeduction{
            var_decl: VarDeclaration{
                name: var_name.to_string(),
                is_var_arg: false,
                span: Span::unknown(),
            },
            deduction: deduced_shape.clone(),
            deducted_from: vec![deduced_from.clone()],
            alternative: None,
            many_of_shapes: false,
        }
    }
}


/// The inner set of actions for the command processor. Each denotes a way to change state in the processor without changing it directly from the command itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandAction {
    /// Change to a new directory or path (in non-filesystem situations)
    ChangePath(String),
    /// Exit out of Nu
    Exit,
    /// Display an error
    Error(ShellError),
    /// Enter a new shell at the given path
    EnterShell(String),
    /// Convert the value given from one type to another
    AutoConvert(Value, String),
    /// Enter a value shell, one that allows exploring inside of a Value
    EnterValueShell(Value),
    /// Enter the help shell, which allows exploring the help system
    EnterHelpShell(Value),
    /// Enter the help shell, which allows exploring the help system
    //TODO Vec<VarShapeDeduction> can be Signature!?
    AddAlias(String, Vec<VarShapeDeduction>, Block),
    /// Go to the previous shell in the shell ring buffer
    PreviousShell,
    /// Go to the next shell in the shell ring buffer
    NextShell,
    /// Leave the current shell. If it's the last shell, exit out of Nu
    LeaveShell,
}

impl PrettyDebug for CommandAction {
    /// Get a command action ready to be pretty-printed
    fn pretty(&self) -> DebugDocBuilder {
        match self {
            CommandAction::ChangePath(path) => b::typed("change path", b::description(path)),
            CommandAction::Exit => b::description("exit"),
            CommandAction::Error(_) => b::error("error"),
            CommandAction::AutoConvert(_, extension) => {
                b::typed("auto convert", b::description(extension))
            }
            CommandAction::EnterShell(s) => b::typed("enter shell", b::description(s)),
            CommandAction::EnterValueShell(v) => b::typed("enter value shell", v.pretty()),
            CommandAction::EnterHelpShell(v) => b::typed("enter help shell", v.pretty()),
            CommandAction::AddAlias(..) => b::description("add alias"),
            CommandAction::PreviousShell => b::description("previous shell"),
            CommandAction::NextShell => b::description("next shell"),
            CommandAction::LeaveShell => b::description("leave shell"),
        }
    }
}

/// The fundamental success type in the pipeline. Commands return these values as their main responsibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReturnSuccess {
    /// A value to be used or shown to the user
    Value(Value),
    /// A debug-enabled value to be used or shown to the user
    DebugValue(Value),
    /// An action to be performed as values pass out of the command. These are performed rather than passed to the next command in the pipeline
    Action(CommandAction),
}

impl PrettyDebug for ReturnSuccess {
    /// Get a return success ready to be pretty-printed
    fn pretty(&self) -> DebugDocBuilder {
        match self {
            ReturnSuccess::Value(value) => b::typed("value", value.pretty()),
            ReturnSuccess::DebugValue(value) => b::typed("debug value", value.pretty()),
            ReturnSuccess::Action(action) => b::typed("action", action.pretty()),
        }
    }
}

/// The core Result type for pipelines
pub type ReturnValue = Result<ReturnSuccess, ShellError>;

impl From<Value> for ReturnValue {
    fn from(v: Value) -> Self {
        Ok(ReturnSuccess::Value(v))
    }
}

impl ReturnSuccess {
    /// Get to the contained Value, if possible
    pub fn raw_value(&self) -> Option<Value> {
        match self {
            ReturnSuccess::Value(raw) => Some(raw.clone()),
            ReturnSuccess::DebugValue(raw) => Some(raw.clone()),
            ReturnSuccess::Action(_) => None,
        }
    }

    /// Helper function for an action to change the the path
    pub fn change_cwd(path: String) -> ReturnValue {
        Ok(ReturnSuccess::Action(CommandAction::ChangePath(path)))
    }

    /// Helper function to create simple values for returning
    pub fn value(input: impl Into<Value>) -> ReturnValue {
        Ok(ReturnSuccess::Value(input.into()))
    }

    /// Helper function to create simple debug-enabled values for returning
    pub fn debug_value(input: impl Into<Value>) -> ReturnValue {
        Ok(ReturnSuccess::DebugValue(input.into()))
    }

    /// Helper function for creating actions
    pub fn action(input: CommandAction) -> ReturnValue {
        Ok(ReturnSuccess::Action(input))
    }
}
