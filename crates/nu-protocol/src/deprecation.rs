use crate::{FromValue, ParseWarning, ShellError, Type, Value, ast::Call};

// Make nu_protocol available in this namespace, consumers of this crate will
// have this without such an export.
// The `FromValue` derive macro fully qualifies paths to "nu_protocol".
use crate::{self as nu_protocol, ReportMode, Span};

#[derive(FromValue)]
pub struct DeprecationEntry {
    // might need to revisit this if we added additional DeprecationTypes
    #[nu_value(rename = "flag", default)]
    ty: DeprecationType,
    #[nu_value(rename = "report")]
    pub report_mode: ReportMode,
    since: Option<String>,
    expected_removal: Option<String>,
    help: Option<String>,
}

/// What this deprecation affects
enum DeprecationType {
    /// Deprecation of whole command
    Command,
    /// Deprecation of a flag/switch
    Flag(String),
}

impl Default for DeprecationType {
    fn default() -> Self {
        DeprecationType::Command
    }
}

impl FromValue for DeprecationType {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::String { val, .. } => Ok(DeprecationType::Flag(val)),
            Value::Nothing { .. } => Ok(DeprecationType::Command),
            v => Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }

    fn expected_type() -> Type {
        Type::String
    }
}

impl FromValue for ReportMode {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        let span = v.span();
        let Value::String { val, .. } = v else {
            return Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            });
        };
        match val.as_str() {
            "first" => Ok(ReportMode::FirstUse),
            "every" => Ok(ReportMode::EveryUse),
            _ => Err(ShellError::InvalidValue {
                valid: "first or every".into(),
                actual: val,
                span: span,
            }),
        }
    }

    fn expected_type() -> Type {
        Type::String
    }
}

impl DeprecationEntry {
    fn check(&self, call: &Call) -> bool {
        match &self.ty {
            DeprecationType::Command => true,
            DeprecationType::Flag(flag) => call.get_flag_expr(flag).is_some(),
        }
    }

    fn type_name(&self) -> String {
        match &self.ty {
            DeprecationType::Command => "Command".to_string(),
            DeprecationType::Flag(_) => "Flag".to_string(),
        }
    }

    fn label(&self, command_name: &str) -> String {
        let name = match &self.ty {
            DeprecationType::Command => command_name,
            DeprecationType::Flag(flag) => &format!("{command_name} --{flag}"),
        };
        let since = match &self.since {
            Some(since) => format!("was deprecated in {since}"),
            None => format!("is deprecated"),
        };
        let removal = match &self.expected_removal {
            Some(expected) => format!("and will be removed in {expected}"),
            None => format!("and will be removed in a future release"),
        };
        format!("{name} {since} {removal}.")
    }

    fn span(&self, call: &Call) -> Span {
        match &self.ty {
            DeprecationType::Command => call.span(),
            DeprecationType::Flag(flag) => call
                .get_named_arg(flag)
                .map(|arg| arg.span)
                .unwrap_or(Span::unknown()),
        }
    }

    pub fn parse_warning(self, command_name: &str, call: &Call) -> Option<ParseWarning> {
        if !self.check(call) {
            return None;
        }

        let dep_type = self.type_name();
        let label = self.label(command_name);
        let span = self.span(call);
        let report_mode = self.report_mode;
        match self.help {
            Some(help) => Some(ParseWarning::DeprecationWarningWithHelp {
                dep_type,
                label,
                span,
                report_mode,
                help,
            }),
            None => Some(ParseWarning::DeprecationWarning {
                dep_type,
                label,
                span,
                report_mode,
            }),
        }
    }
}
