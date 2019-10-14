use nu::{
    serve_plugin, CallInfo, Plugin, Primitive, ReturnSuccess, ReturnValue, ShellError, Signature,
    SyntaxShape, Tagged, TaggedItem, Value,
};

enum Action {
    SemVerAction(SemVerAction),
    Default,
}

pub enum SemVerAction {
    Major,
    Minor,
    Patch,
}

pub type ColumnPath = Vec<Tagged<String>>;

struct Inc {
    field: Option<ColumnPath>,
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

    fn apply(&self, input: &str) -> Result<Value, ShellError> {
        let applied = match &self.action {
            Some(Action::SemVerAction(act_on)) => {
                let mut ver = match semver::Version::parse(&input) {
                    Ok(parsed_ver) => parsed_ver,
                    Err(_) => return Ok(Value::string(input.to_string())),
                };

                match act_on {
                    SemVerAction::Major => ver.increment_major(),
                    SemVerAction::Minor => ver.increment_minor(),
                    SemVerAction::Patch => ver.increment_patch(),
                }

                Value::string(ver.to_string())
            }
            Some(Action::Default) | None => match input.parse::<u64>() {
                Ok(v) => Value::string(format!("{}", v + 1)),
                Err(_) => Value::string(input),
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

    fn inc(&self, value: Tagged<Value>) -> Result<Tagged<Value>, ShellError> {
        match value.item() {
            Value::Primitive(Primitive::Int(i)) => Ok(Value::int(i + 1).tagged(value.tag())),
            Value::Primitive(Primitive::Bytes(b)) => {
                Ok(Value::bytes(b + 1 as u64).tagged(value.tag()))
            }
            Value::Primitive(Primitive::String(ref s)) => {
                Ok(Tagged::from_item(self.apply(&s)?, value.tag()))
            }
            Value::Row(_) => match self.field {
                Some(ref f) => {
                    let replacement = match value.item.get_data_by_column_path(value.tag(), f) {
                        Some(result) => self.inc(result.map(|x| x.clone()))?,
                        None => {
                            return Err(ShellError::string("inc could not find field to replace"))
                        }
                    };
                    match value.item.replace_data_at_column_path(
                        value.tag(),
                        f,
                        replacement.item.clone(),
                    ) {
                        Some(v) => return Ok(v),
                        None => {
                            return Err(ShellError::string("inc could not find field to replace"))
                        }
                    }
                }
                None => Err(ShellError::string(
                    "inc needs a field when incrementing a column in a table",
                )),
            },
            x => Err(ShellError::string(format!(
                "Unrecognized type in stream: {:?}",
                x
            ))),
        }
    }
}

impl Plugin for Inc {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("inc")
            .desc("Increment a value or version. Optionally use the column of a table.")
            .switch("major")
            .switch("minor")
            .switch("patch")
            .rest(SyntaxShape::ColumnPath)
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
                    table @ Tagged {
                        item: Value::Table(_),
                        ..
                    } => {
                        self.field = Some(table.as_column_path()?.item);
                    }
                    _ => {
                        return Err(ShellError::string(format!(
                            "Unrecognized type in params: {:?}",
                            arg
                        )))
                    }
                }
            }
        }

        if self.action.is_none() {
            self.action = Some(Action::Default);
        }

        match &self.error {
            Some(reason) => {
                return Err(ShellError::string(format!("{}: {}", reason, Inc::usage())))
            }
            None => Ok(vec![]),
        }
    }

    fn filter(&mut self, input: Tagged<Value>) -> Result<Vec<ReturnValue>, ShellError> {
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
    use nu::{
        CallInfo, EvaluatedArgs, Plugin, ReturnSuccess, SourceMap, Tag, Tagged, TaggedDictBuilder,
        TaggedItem, Value,
    };

    struct CallStub {
        anchor: uuid::Uuid,
        positionals: Vec<Tagged<Value>>,
        flags: IndexMap<String, Tagged<Value>>,
    }

    impl CallStub {
        fn new(anchor: uuid::Uuid) -> CallStub {
            CallStub {
                anchor,
                positionals: vec![],
                flags: indexmap::IndexMap::new(),
            }
        }

        fn with_long_flag(&mut self, name: &str) -> &mut Self {
            self.flags.insert(
                name.to_string(),
                Value::boolean(true).tagged(Tag::unknown()),
            );
            self
        }

        fn with_parameter(&mut self, name: &str) -> &mut Self {
            let fields: Vec<Tagged<Value>> = name
                .split(".")
                .map(|s| Value::string(s.to_string()).tagged(Tag::unknown_span(self.anchor)))
                .collect();

            self.positionals
                .push(Value::Table(fields).tagged(Tag::unknown_span(self.anchor)));
            self
        }

        fn create(&self) -> CallInfo {
            CallInfo {
                args: EvaluatedArgs::new(Some(self.positionals.clone()), Some(self.flags.clone())),
                source_map: SourceMap::new(),
                name_tag: Tag::unknown_span(self.anchor),
            }
        }
    }

    fn cargo_sample_record(with_version: &str) -> Tagged<Value> {
        let mut package = TaggedDictBuilder::new(Tag::unknown());
        package.insert("version", Value::string(with_version));
        package.into_tagged_value()
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
            .begin_filter(CallStub::new(test_uuid()).with_long_flag("major").create())
            .is_ok());
        assert!(plugin.action.is_some());
    }

    #[test]
    fn inc_plugin_accepts_minor() {
        let mut plugin = Inc::new();

        assert!(plugin
            .begin_filter(CallStub::new(test_uuid()).with_long_flag("minor").create())
            .is_ok());
        assert!(plugin.action.is_some());
    }

    #[test]
    fn inc_plugin_accepts_patch() {
        let mut plugin = Inc::new();

        assert!(plugin
            .begin_filter(CallStub::new(test_uuid()).with_long_flag("patch").create())
            .is_ok());
        assert!(plugin.action.is_some());
    }

    #[test]
    fn inc_plugin_accepts_only_one_action() {
        let mut plugin = Inc::new();

        assert!(plugin
            .begin_filter(
                CallStub::new(test_uuid())
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
            .begin_filter(
                CallStub::new(test_uuid())
                    .with_parameter("package.version")
                    .create()
            )
            .is_ok());

        assert_eq!(
            plugin
                .field
                .map(|f| f.into_iter().map(|f| f.item).collect()),
            Some(vec!["package".to_string(), "version".to_string()])
        );
    }

    #[test]
    fn incs_major() {
        let mut inc = Inc::new();
        inc.for_semver(SemVerAction::Major);
        assert_eq!(inc.apply("0.1.3").unwrap(), Value::string("1.0.0"));
    }

    #[test]
    fn incs_minor() {
        let mut inc = Inc::new();
        inc.for_semver(SemVerAction::Minor);
        assert_eq!(inc.apply("0.1.3").unwrap(), Value::string("0.2.0"));
    }

    #[test]
    fn incs_patch() {
        let mut inc = Inc::new();
        inc.for_semver(SemVerAction::Patch);
        assert_eq!(inc.apply("0.1.3").unwrap(), Value::string("0.1.4"));
    }

    #[test]
    fn inc_plugin_applies_major() {
        let mut plugin = Inc::new();

        assert!(plugin
            .begin_filter(
                CallStub::new(test_uuid())
                    .with_long_flag("major")
                    .with_parameter("version")
                    .create()
            )
            .is_ok());

        let subject = cargo_sample_record("0.1.3");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Row(o),
                ..
            }) => assert_eq!(
                *o.get_data(&String::from("version")).borrow(),
                Value::string(String::from("1.0.0"))
            ),
            _ => {}
        }
    }

    #[test]
    fn inc_plugin_applies_minor() {
        let mut plugin = Inc::new();

        assert!(plugin
            .begin_filter(
                CallStub::new(test_uuid())
                    .with_long_flag("minor")
                    .with_parameter("version")
                    .create()
            )
            .is_ok());

        let subject = cargo_sample_record("0.1.3");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Row(o),
                ..
            }) => assert_eq!(
                *o.get_data(&String::from("version")).borrow(),
                Value::string(String::from("0.2.0"))
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
                CallStub::new(test_uuid())
                    .with_long_flag("patch")
                    .with_parameter(&field)
                    .create()
            )
            .is_ok());

        let subject = cargo_sample_record("0.1.3");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Row(o),
                ..
            }) => assert_eq!(
                *o.get_data(&field).borrow(),
                Value::string(String::from("0.1.4"))
            ),
            _ => {}
        }
    }

    fn test_uuid() -> uuid::Uuid {
        uuid::Uuid::nil()
    }
}
