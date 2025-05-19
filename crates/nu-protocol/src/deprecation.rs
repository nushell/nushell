use crate::{ParseWarning, ast::Call};

#[derive(Clone)]
pub enum DeprecationStatus {
    Undeprecated,
    Deprecated(Option<String>),
}

impl DeprecationStatus {
    pub fn into_warning(self, command_name: &str, call: &Call) -> Option<ParseWarning> {
        match self {
            DeprecationStatus::Undeprecated => None,
            DeprecationStatus::Deprecated(None) => Some(ParseWarning::DeprecatedWarning {
                old_command: command_name.to_string(),
                span: call.head,
                decl_id: call.decl_id,
            }),
            DeprecationStatus::Deprecated(Some(message)) => {
                Some(ParseWarning::DeprecatedWarningWithMessage {
                    old_command: command_name.to_string(),
                    span: call.head,
                    help: message,
                    decl_id: call.decl_id,
                })
            }
        }
    }
}
