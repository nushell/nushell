use indexmap::IndexMap;
use nu::{
    serve_plugin, CallInfo, NamedType, Plugin, PositionalType, Primitive, ReturnSuccess,
    ReturnValue, ShellError, Signature, Tagged, Value,
};

enum Action {
    Downcase,
    Upcase,
    ToInteger,
}

struct Str {
    field: Option<String>,
    error: Option<String>,
    action: Option<Action>,
}

impl Str {
    fn new() -> Str {
        Str {
            field: None,
            error: None,
            action: None,
        }
    }

    fn apply(&self, input: &str) -> Value {
        match self.action {
            Some(Action::Downcase) => Value::string(input.to_ascii_lowercase()),
            Some(Action::Upcase) => Value::string(input.to_ascii_uppercase()),
            Some(Action::ToInteger) => match input.trim().parse::<i64>() {
                Ok(v) => Value::int(v),
                Err(_) => Value::string(input),
            },
            None => Value::string(input.to_string()),
        }
    }

    fn for_input(&mut self, field: String) {
        self.field = Some(field);
    }

    fn permit(&mut self) -> bool {
        self.action.is_none()
    }

    fn log_error(&mut self, message: &str) {
        self.error = Some(message.to_string());
    }

    fn for_to_int(&mut self) {
        if self.permit() {
            self.action = Some(Action::ToInteger);
        } else {
            self.log_error("can only apply one");
        }
    }

    fn for_downcase(&mut self) {
        if self.permit() {
            self.action = Some(Action::Downcase);
        } else {
            self.log_error("can only apply one");
        }
    }

    fn for_upcase(&mut self) {
        if self.permit() {
            self.action = Some(Action::Upcase);
        } else {
            self.log_error("can only apply one");
        }
    }

    fn usage(&self) -> &'static str {
        "Usage: str field [--downcase|--upcase|--to-int]"
    }
}

impl Str {
    fn strutils(
        &self,
        value: Tagged<Value>,
        field: &Option<String>,
    ) -> Result<Tagged<Value>, ShellError> {
        match value.item {
            Value::Primitive(Primitive::String(ref s)) => {
                Ok(Tagged::from_item(self.apply(&s), value.tag()))
            }
            Value::Object(_) => match field {
                Some(f) => {
                    let replacement = match value.item.get_data_by_path(value.tag(), f) {
                        Some(result) => self.strutils(result.map(|x| x.clone()), &None)?,
                        None => {
                            return Err(ShellError::string("str could not find field to replace"))
                        }
                    };
                    match value
                        .item
                        .replace_data_at_path(value.tag(), f, replacement.item.clone())
                    {
                        Some(v) => return Ok(v),
                        None => {
                            return Err(ShellError::string("str could not find field to replace"))
                        }
                    }
                }
                None => Err(ShellError::string(format!(
                    "{}: {}",
                    "str needs a field when applying it to a value in an object",
                    self.usage()
                ))),
            },
            x => Err(ShellError::string(format!(
                "Unrecognized type in stream: {:?}",
                x
            ))),
        }
    }
}

impl Plugin for Str {
    fn config(&mut self) -> Result<Signature, ShellError> {
        let mut named = IndexMap::new();
        named.insert("downcase".to_string(), NamedType::Switch);
        named.insert("upcase".to_string(), NamedType::Switch);
        named.insert("to-int".to_string(), NamedType::Switch);

        Ok(Signature {
            name: "str".to_string(),
            positional: vec![PositionalType::optional_any("Field")],
            is_filter: true,
            named,
            rest_positional: true,
        })
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        if call_info.args.has("downcase") {
            self.for_downcase();
        }

        if call_info.args.has("upcase") {
            self.for_upcase();
        }

        if call_info.args.has("to-int") {
            self.for_to_int();
        }

        if let Some(args) = call_info.args.positional {
            for arg in args {
                match arg {
                    Tagged {
                        item: Value::Primitive(Primitive::String(s)),
                        ..
                    } => {
                        self.for_input(s);
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

        match &self.error {
            Some(reason) => {
                return Err(ShellError::string(format!("{}: {}", reason, self.usage())))
            }
            None => Ok(vec![]),
        }
    }

    fn filter(&mut self, input: Tagged<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![ReturnSuccess::value(
            self.strutils(input, &self.field)?,
        )])
    }
}

fn main() {
    serve_plugin(&mut Str::new());
}

#[cfg(test)]
mod tests {

    use super::Str;
    use indexmap::IndexMap;
    use nu::{
        CallInfo, EvaluatedArgs, Plugin, ReturnSuccess, SourceMap, Span, Tag, Tagged,
        TaggedDictBuilder, TaggedItem, Value,
    };

    struct CallStub {
        positionals: Vec<Tagged<Value>>,
        flags: IndexMap<String, Tagged<Value>>,
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
                Value::boolean(true).simple_spanned(Span::unknown()),
            );
            self
        }

        fn with_parameter(&mut self, name: &str) -> &mut Self {
            self.positionals
                .push(Value::string(name.to_string()).simple_spanned(Span::unknown()));
            self
        }

        fn create(&self, name_span: Span) -> CallInfo {
            CallInfo {
                args: EvaluatedArgs::new(Some(self.positionals.clone()), Some(self.flags.clone())),
                source_map: SourceMap::new(),
                name_span,
            }
        }
    }

    fn sample_record(key: &str, value: &str) -> Tagged<Value> {
        let mut record = TaggedDictBuilder::new(Tag::unknown());
        record.insert(key.clone(), Value::string(value));
        record.into_tagged_value()
    }

    #[test]
    fn str_plugin_configuration_flags_wired() {
        let mut plugin = Str::new();

        let configured = plugin.config().unwrap();

        for action_flag in &["downcase", "upcase", "to-int"] {
            assert!(configured.named.get(*action_flag).is_some());
        }
    }

    #[test]
    fn str_plugin_accepts_downcase() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_long_flag("downcase")
                    .create(Span::unknown())
            )
            .is_ok());
        assert!(plugin.action.is_some());
    }

    #[test]
    fn str_plugin_accepts_upcase() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_long_flag("upcase")
                    .create(Span::unknown())
            )
            .is_ok());
        assert!(plugin.action.is_some());
    }

    #[test]
    fn str_plugin_accepts_to_int() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_long_flag("to-int")
                    .create(Span::unknown())
            )
            .is_ok());
        assert!(plugin.action.is_some());
    }

    #[test]
    fn str_plugin_accepts_field() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_parameter("package.description")
                    .create(Span::unknown())
            )
            .is_ok());

        assert_eq!(plugin.field, Some("package.description".to_string()));
    }

    #[test]
    fn str_plugin_accepts_only_one_action() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_long_flag("upcase")
                    .with_long_flag("downcase")
                    .with_long_flag("to-int")
                    .create(Span::unknown()),
            )
            .is_err());
        assert_eq!(plugin.error, Some("can only apply one".to_string()));
    }

    #[test]
    fn str_downcases() {
        let mut strutils = Str::new();
        strutils.for_downcase();
        assert_eq!(strutils.apply("ANDRES"), Value::string("andres"));
    }

    #[test]
    fn str_upcases() {
        let mut strutils = Str::new();
        strutils.for_upcase();
        assert_eq!(strutils.apply("andres"), Value::string("ANDRES"));
    }

    #[test]
    fn str_to_int() {
        let mut strutils = Str::new();
        strutils.for_to_int();
        assert_eq!(strutils.apply("9999"), Value::int(9999 as i64));
    }

    #[test]
    fn str_plugin_applies_upcase() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_long_flag("upcase")
                    .with_parameter("name")
                    .create(Span::unknown())
            )
            .is_ok());

        let subject = sample_record("name", "jotandrehuda");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Object(o),
                ..
            }) => assert_eq!(
                *o.get_data(&String::from("name")).borrow(),
                Value::string(String::from("JOTANDREHUDA"))
            ),
            _ => {}
        }
    }

    #[test]
    fn str_plugin_applies_downcase() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_long_flag("downcase")
                    .with_parameter("name")
                    .create(Span::unknown())
            )
            .is_ok());

        let subject = sample_record("name", "JOTANDREHUDA");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Object(o),
                ..
            }) => assert_eq!(
                *o.get_data(&String::from("name")).borrow(),
                Value::string(String::from("jotandrehuda"))
            ),
            _ => {}
        }
    }

    #[test]
    fn str_plugin_applies_to_int() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_long_flag("to-int")
                    .with_parameter("Nu_birthday")
                    .create(Span::unknown())
            )
            .is_ok());

        let subject = sample_record("Nu_birthday", "10");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Object(o),
                ..
            }) => assert_eq!(
                *o.get_data(&String::from("Nu_birthday")).borrow(),
                Value::int(10)
            ),
            _ => {}
        }
    }
}
