use crate::value::Value;
use nu_errors::ShellError;
use nu_source::{b, DebugDocBuilder, PrettyDebug};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandAction {
    ChangePath(String),
    Exit,
    Error(ShellError),
    EnterShell(String),
    AutoConvert(Value, String),
    EnterValueShell(Value),
    EnterHelpShell(Value),
    PreviousShell,
    NextShell,
    LeaveShell,
}

impl PrettyDebug for CommandAction {
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
            CommandAction::PreviousShell => b::description("previous shell"),
            CommandAction::NextShell => b::description("next shell"),
            CommandAction::LeaveShell => b::description("leave shell"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReturnSuccess {
    Value(Value),
    DebugValue(Value),
    Action(CommandAction),
}

impl PrettyDebug for ReturnSuccess {
    fn pretty(&self) -> DebugDocBuilder {
        match self {
            ReturnSuccess::Value(value) => b::typed("value", value.pretty()),
            ReturnSuccess::DebugValue(value) => b::typed("debug value", value.pretty()),
            ReturnSuccess::Action(action) => b::typed("action", action.pretty()),
        }
    }
}

pub type ReturnValue = Result<ReturnSuccess, ShellError>;

impl Into<ReturnValue> for Value {
    fn into(self) -> ReturnValue {
        Ok(ReturnSuccess::Value(self))
    }
}

impl ReturnSuccess {
    pub fn raw_value(&self) -> Option<Value> {
        match self {
            ReturnSuccess::Value(raw) => Some(raw.clone()),
            ReturnSuccess::DebugValue(raw) => Some(raw.clone()),
            ReturnSuccess::Action(_) => None,
        }
    }

    pub fn change_cwd(path: String) -> ReturnValue {
        Ok(ReturnSuccess::Action(CommandAction::ChangePath(path)))
    }

    pub fn value(input: impl Into<Value>) -> ReturnValue {
        Ok(ReturnSuccess::Value(input.into()))
    }

    pub fn debug_value(input: impl Into<Value>) -> ReturnValue {
        Ok(ReturnSuccess::DebugValue(input.into()))
    }

    pub fn action(input: CommandAction) -> ReturnValue {
        Ok(ReturnSuccess::Action(input))
    }
}
