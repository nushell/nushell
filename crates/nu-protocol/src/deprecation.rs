use crate::{FromValue, ParseWarning, ShellError, Type, Value, ast::Call};

// Make nu_protocol available in this namespace, consumers of this crate will
// have this without such an export.
// The `FromValue` derive macro fully qualifies paths to "nu_protocol".
use crate::{self as nu_protocol, ReportMode, Span};

/// A entry which indicates that some part of, or all of, a command is deprecated
///
/// Commands can implement [`Command::deprecation_info`](crate::engine::Command::deprecation_info)
/// to return deprecation entries, which will cause a parse-time warning.
/// Additionally, custom commands can use the `@deprecated` attribute to add a
/// `DeprecationEntry`.
#[derive(FromValue)]
pub struct DeprecationEntry {
    /// The type of deprecation
    // might need to revisit this if we added additional DeprecationTypes
    #[nu_value(rename = "flag", default)]
    pub ty: DeprecationType,
    /// How this deprecation should be reported
    #[nu_value(rename = "report")]
    pub report_mode: ReportMode,
    /// When this deprecation started
    pub since: Option<String>,
    /// When this item is expected to be removed
    pub expected_removal: Option<String>,
    /// Help text, possibly including a suggestion for what to use instead
    pub help: Option<String>,
}

/// What this deprecation affects
#[derive(Default)]
pub enum DeprecationType {
    /// Deprecation of whole command
    #[default]
    Command,
    /// Deprecation of a flag/switch
    Flag(String),
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
                span,
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
            DeprecationType::Flag(flag) => {
                // Make sure we don't accidentally have dashes in the flag
                debug_assert!(
                    !flag.starts_with('-'),
                    "DeprecationEntry for {flag} should not include dashes in the flag name!"
                );

                call.get_named_arg(flag).is_some()
            }
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
            None => "is deprecated".to_string(),
        };
        let removal = match &self.expected_removal {
            Some(expected) => format!("and will be removed in {expected}"),
            None => "and will be removed in a future release".to_string(),
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
        Some(ParseWarning::Deprecated {
            dep_type,
            label,
            span,
            report_mode,
            help: self.help,
        })
    }
}
