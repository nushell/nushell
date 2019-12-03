use nu::{did_you_mean, serve_plugin, value, Plugin, ValueExt};
use nu_errors::ShellError;
use nu_protocol::{
    CallInfo, ColumnPath, Primitive, ReturnSuccess, ReturnValue, ShellTypeName, Signature,
    SyntaxShape, UntaggedValue, Value,
};
use nu_source::{span_for_spanned_list, HasSpan, SpannedItem, Tagged};

enum Action {
    SemVerAction(SemVerAction),
    Default,
}

pub enum SemVerAction {
    Major,
    Minor,
    Patch,
}

struct Inc {
    field: Option<Tagged<ColumnPath>>,
    error: Option<String>,
    action: Option<Action>,
}

impl Inc {
    fn new() -> Inc {
        Inc {
            field: None,
            error: None,
            action: None,
        }
    }

    fn apply(&self, input: &str) -> Result<UntaggedValue, ShellError> {
        let applied = match &self.action {
            Some(Action::SemVerAction(act_on)) => {
                let mut ver = match semver::Version::parse(&input) {
                    Ok(parsed_ver) => parsed_ver,
                    Err(_) => return Ok(value::string(input.to_string())),
                };

                match act_on {
                    SemVerAction::Major => ver.increment_major(),
                    SemVerAction::Minor => ver.increment_minor(),
                    SemVerAction::Patch => ver.increment_patch(),
                }

                value::string(ver.to_string())
            }
            Some(Action::Default) | None => match input.parse::<u64>() {
                Ok(v) => value::string(format!("{}", v + 1)),
                Err(_) => value::string(input),
            },
        };

        Ok(applied)
    }

    fn for_semver(&mut self, part: SemVerAction) {
        if self.permit() {
            self.action = Some(Action::SemVerAction(part));
        } else {
            self.log_error("can only apply one");
        }
    }

    fn permit(&mut self) -> bool {
        self.action.is_none()
    }

    fn log_error(&mut self, message: &str) {
        self.error = Some(message.to_string());
    }

    pub fn usage() -> &'static str {
        "Usage: inc field [--major|--minor|--patch]"
    }

    fn inc(&self, value: Value) -> Result<Value, ShellError> {
        match &value.value {
            UntaggedValue::Primitive(Primitive::Int(i)) => {
                Ok(value::int(i + 1).into_value(value.tag()))
            }
            UntaggedValue::Primitive(Primitive::Bytes(b)) => {
                Ok(value::bytes(b + 1 as u64).into_value(value.tag()))
            }
            UntaggedValue::Primitive(Primitive::String(ref s)) => {
                Ok(self.apply(&s)?.into_value(value.tag()))
            }
            UntaggedValue::Table(values) => {
                if values.len() == 1 {
                    return Ok(UntaggedValue::Table(vec![self.inc(values[0].clone())?])
                        .into_value(value.tag()));
                } else {
                    return Err(ShellError::type_error(
                        "incrementable value",
                        value.type_name().spanned(value.span()),
                    ));
                }
            }

            UntaggedValue::Row(_) => match self.field {
                Some(ref f) => {
                    let fields = f.clone();

                    let replace_for = value.get_data_by_column_path(
                        &f,
                        Box::new(move |(obj_source, column_path_tried, _)| {
                            match did_you_mean(&obj_source, &column_path_tried) {
                                Some(suggestions) => {
                                    return ShellError::labeled_error(
                                        "Unknown column",
                                        format!("did you mean '{}'?", suggestions[0].1),
                                        span_for_spanned_list(fields.iter().map(|p| p.span)),
                                    )
                                }
                                None => {
                                    return ShellError::labeled_error(
                                        "Unknown column",
                                        "row does not contain this column",
                                        span_for_spanned_list(fields.iter().map(|p| p.span)),
                                    )
                                }
                            }
                        }),
                    );

                    let got = replace_for?;
                    let replacement = self.inc(got.clone())?;

                    match value.replace_data_at_column_path(
                        &f,
                        replacement.value.clone().into_untagged_value(),
                    ) {
                        Some(v) => return Ok(v),
                        None => {
                            return Err(ShellError::labeled_error(
                                "inc could not find field to replace",
                                "column name",
                                value.tag(),
                            ))
                        }
                    }
                }
                None => Err(ShellError::untagged_runtime_error(
                    "inc needs a field when incrementing a column in a table",
                )),
            },
            _ => Err(ShellError::type_error(
                "incrementable value",
                value.type_name().spanned(value.span()),
            )),
        }
    }
}

impl Plugin for Inc {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("inc")
            .desc("Increment a value or version. Optionally use the column of a table.")
            .switch("major", "increment the major version (eg 1.2.1 -> 2.0.0)")
            .switch("minor", "increment the minor version (eg 1.2.1 -> 1.3.0)")
            .switch("patch", "increment the patch version (eg 1.2.1 -> 1.2.2)")
            .rest(SyntaxShape::ColumnPath, "the column(s) to update")
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        if call_info.args.has("major") {
            self.for_semver(SemVerAction::Major);
        }
        if call_info.args.has("minor") {
            self.for_semver(SemVerAction::Minor);
        }
        if call_info.args.has("patch") {
            self.for_semver(SemVerAction::Patch);
        }

        if let Some(args) = call_info.args.positional {
            for arg in args {
                match arg {
                    table @ Value {
                        value: UntaggedValue::Primitive(Primitive::ColumnPath(_)),
                        ..
                    } => {
                        self.field = Some(table.as_column_path()?);
                    }
                    value => {
                        return Err(ShellError::type_error(
                            "table",
                            value.type_name().spanned(value.span()),
                        ))
                    }
                }
            }
        }

        if self.action.is_none() {
            self.action = Some(Action::Default);
        }

        match &self.error {
            Some(reason) => {
                return Err(ShellError::untagged_runtime_error(format!(
                    "{}: {}",
                    reason,
                    Inc::usage()
                )))
            }
            None => Ok(vec![]),
        }
    }

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![ReturnSuccess::value(self.inc(input)?)])
    }
}

fn main() {
    serve_plugin(&mut Inc::new());
}

#[cfg(test)]
mod tests {

    use super::{Inc, SemVerAction};
    use indexmap::IndexMap;
    use nu::{value, Plugin, TaggedDictBuilder};
    use nu_protocol::{
        CallInfo, EvaluatedArgs, PathMember, ReturnSuccess, UnspannedPathMember, UntaggedValue,
        Value,
    };
    use nu_source::{Span, Tag};

    struct CallStub {
        positionals: Vec<Value>,
        flags: IndexMap<String, Value>,
    }

    impl CallStub {
        fn new() -> CallStub {
            CallStub {
                positionals: vec![],
                flags: indexmap::IndexMap::new(),
            }
        }

        fn with_long_flag(&mut self, name: &str) -> &mut Self {
            self.flags.insert(
                name.to_string(),
                value::boolean(true).into_value(Tag::unknown()),
            );
            self
        }

        fn with_parameter(&mut self, name: &str) -> &mut Self {
            let fields: Vec<PathMember> = name
                .split(".")
                .map(|s| {
                    UnspannedPathMember::String(s.to_string()).into_path_member(Span::unknown())
                })
                .collect();

            self.positionals
                .push(value::column_path(fields).into_untagged_value());
            self
        }

        fn create(&self) -> CallInfo {
            CallInfo {
                args: EvaluatedArgs::new(Some(self.positionals.clone()), Some(self.flags.clone())),
                name_tag: Tag::unknown(),
            }
        }
    }

    fn cargo_sample_record(with_version: &str) -> Value {
        let mut package = TaggedDictBuilder::new(Tag::unknown());
        package.insert_untagged("version", value::string(with_version));
        package.into_value()
    }

    #[test]
    fn inc_plugin_configuration_flags_wired() {
        let mut plugin = Inc::new();

        let configured = plugin.config().expect("Can not configure plugin");

        for action_flag in &["major", "minor", "patch"] {
            assert!(configured.named.get(*action_flag).is_some());
        }
    }

    #[test]
    fn inc_plugin_accepts_major() {
        let mut plugin = Inc::new();

        assert!(plugin
            .begin_filter(CallStub::new().with_long_flag("major").create())
            .is_ok());
        assert!(plugin.action.is_some());
    }

    #[test]
    fn inc_plugin_accepts_minor() {
        let mut plugin = Inc::new();

        assert!(plugin
            .begin_filter(CallStub::new().with_long_flag("minor").create())
            .is_ok());
        assert!(plugin.action.is_some());
    }

    #[test]
    fn inc_plugin_accepts_patch() {
        let mut plugin = Inc::new();

        assert!(plugin
            .begin_filter(CallStub::new().with_long_flag("patch").create())
            .is_ok());
        assert!(plugin.action.is_some());
    }

    #[test]
    fn inc_plugin_accepts_only_one_action() {
        let mut plugin = Inc::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_long_flag("major")
                    .with_long_flag("minor")
                    .create(),
            )
            .is_err());
        assert_eq!(plugin.error, Some("can only apply one".to_string()));
    }

    #[test]
    fn inc_plugin_accepts_field() {
        let mut plugin = Inc::new();

        assert!(plugin
            .begin_filter(CallStub::new().with_parameter("package.version").create())
            .is_ok());

        assert_eq!(
            plugin
                .field
                .map(|f| f.iter().map(|f| f.unspanned.clone()).collect()),
            Some(vec![
                UnspannedPathMember::String("package".to_string()),
                UnspannedPathMember::String("version".to_string())
            ])
        );
    }

    #[test]
    fn incs_major() {
        let mut inc = Inc::new();
        inc.for_semver(SemVerAction::Major);
        assert_eq!(inc.apply("0.1.3").unwrap(), value::string("1.0.0"));
    }

    #[test]
    fn incs_minor() {
        let mut inc = Inc::new();
        inc.for_semver(SemVerAction::Minor);
        assert_eq!(inc.apply("0.1.3").unwrap(), value::string("0.2.0"));
    }

    #[test]
    fn incs_patch() {
        let mut inc = Inc::new();
        inc.for_semver(SemVerAction::Patch);
        assert_eq!(inc.apply("0.1.3").unwrap(), value::string("0.1.4"));
    }

    #[test]
    fn inc_plugin_applies_major() {
        let mut plugin = Inc::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_long_flag("major")
                    .with_parameter("version")
                    .create()
            )
            .is_ok());

        let subject = cargo_sample_record("0.1.3");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Value {
                value: UntaggedValue::Row(o),
                ..
            }) => assert_eq!(
                *o.get_data(&String::from("version")).borrow(),
                value::string(String::from("1.0.0")).into_untagged_value()
            ),
            _ => {}
        }
    }

    #[test]
    fn inc_plugin_applies_minor() {
        let mut plugin = Inc::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_long_flag("minor")
                    .with_parameter("version")
                    .create()
            )
            .is_ok());

        let subject = cargo_sample_record("0.1.3");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Value {
                value: UntaggedValue::Row(o),
                ..
            }) => assert_eq!(
                *o.get_data(&String::from("version")).borrow(),
                value::string(String::from("0.2.0")).into_untagged_value()
            ),
            _ => {}
        }
    }

    #[test]
    fn inc_plugin_applies_patch() {
        let field = String::from("version");
        let mut plugin = Inc::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_long_flag("patch")
                    .with_parameter(&field)
                    .create()
            )
            .is_ok());

        let subject = cargo_sample_record("0.1.3");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Value {
                value: UntaggedValue::Row(o),
                ..
            }) => assert_eq!(
                *o.get_data(&field).borrow(),
                value::string(String::from("0.1.4")).into_untagged_value()
            ),
            _ => {}
        }
    }
}
