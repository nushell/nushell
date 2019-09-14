use nu::{
    serve_plugin, CallInfo, Plugin, Primitive, ReturnSuccess, ReturnValue, ShellError, Signature,
    SyntaxShape, Tagged, Value,
};

#[derive(Debug, Eq, PartialEq)]
enum Action {
    Downcase,
    Upcase,
    ToInteger,
}

struct Str {
    field: Option<String>,
    params: Option<Vec<String>>,
    error: Option<String>,
    action: Option<Action>,
}

impl Str {
    fn new() -> Str {
        Str {
            field: None,
            params: Some(Vec::<String>::new()),
            error: None,
            action: None,
        }
    }

    fn apply(&self, input: &str) -> Result<Value, ShellError> {
        let applied = match self.action.as_ref() {
            Some(Action::Downcase) => Value::string(input.to_ascii_lowercase()),
            Some(Action::Upcase) => Value::string(input.to_ascii_uppercase()),
            Some(Action::ToInteger) => match input.trim() {
                other => match other.parse::<i64>() {
                    Ok(v) => Value::int(v),
                    Err(_) => Value::string(input),
                },
            },
            None => Value::string(input),
        };

        Ok(applied)
    }

    fn for_field(&mut self, field: &str) {
        self.field = Some(String::from(field));
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

    pub fn usage() -> &'static str {
        "Usage: str field [--downcase|--upcase|--to-int]"
    }
}

impl Str {
    fn strutils(&self, value: Tagged<Value>) -> Result<Tagged<Value>, ShellError> {
        match value.item {
            Value::Primitive(Primitive::String(ref s)) => {
                Ok(Tagged::from_item(self.apply(&s)?, value.tag()))
            }
            Value::Row(_) => match self.field {
                Some(ref f) => {
                    let replacement = match value.item.get_data_by_path(value.tag(), f) {
                        Some(result) => self.strutils(result.map(|x| x.clone()))?,
                        None => return Ok(Tagged::from_item(Value::nothing(), value.tag)),
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
                    "str needs a column when applied to a value in a row",
                    Str::usage()
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
        Ok(Signature::build("str")
            .desc("Apply string function. Optional use the field of a table")
            .switch("downcase")
            .switch("upcase")
            .switch("to-int")
            .rest(SyntaxShape::Member)
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        let args = call_info.args;

        if args.has("downcase") {
            self.for_downcase();
        }
        if args.has("upcase") {
            self.for_upcase();
        }
        if args.has("to-int") {
            self.for_to_int();
        }

        if let Some(possible_field) = args.nth(0) {
            match possible_field {
                Tagged {
                    item: Value::Primitive(Primitive::String(s)),
                    ..
                } => match self.action {
                    Some(Action::Downcase)
                    | Some(Action::Upcase)
                    | Some(Action::ToInteger)
                    | None => {
                        self.for_field(&s);
                    }
                },
                _ => {
                    return Err(ShellError::string(format!(
                        "Unrecognized type in params: {:?}",
                        possible_field
                    )))
                }
            }
        }

        for param in args.positional_iter() {
            match param {
                Tagged {
                    item: Value::Primitive(Primitive::String(s)),
                    ..
                } => self.params.as_mut().unwrap().push(String::from(s)),
                _ => {}
            }
        }

        match &self.error {
            Some(reason) => {
                return Err(ShellError::string(format!("{}: {}", reason, Str::usage())))
            }
            None => Ok(vec![]),
        }
    }

    fn filter(&mut self, input: Tagged<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![ReturnSuccess::value(self.strutils(input)?)])
    }
}

fn main() {
    serve_plugin(&mut Str::new());
}

#[cfg(test)]
mod tests {
    use super::{Action, Str};
    use indexmap::IndexMap;
    use nu::{
        CallInfo, EvaluatedArgs, Plugin, Primitive, ReturnSuccess, SourceMap, Tag, Tagged,
        TaggedDictBuilder, TaggedItem, Value,
    };
    use num_bigint::BigInt;

    struct CallStub {
        origin: uuid::Uuid,
        positionals: Vec<Tagged<Value>>,
        flags: IndexMap<String, Tagged<Value>>,
    }

    impl CallStub {
        fn new() -> CallStub {
            CallStub {
                origin: uuid::Uuid::nil(),
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
            self.positionals
                .push(Value::string(name.to_string()).tagged(Tag::unknown()));
            self
        }

        fn create(&self) -> CallInfo {
            CallInfo {
                args: EvaluatedArgs::new(Some(self.positionals.clone()), Some(self.flags.clone())),
                source_map: SourceMap::new(),
                name_tag: Tag::unknown_span(self.origin),
            }
        }
    }

    fn structured_sample_record(key: &str, value: &str) -> Tagged<Value> {
        let mut record = TaggedDictBuilder::new(Tag::unknown());
        record.insert(key.clone(), Value::string(value));
        record.into_tagged_value()
    }

    fn unstructured_sample_record(value: &str) -> Tagged<Value> {
        Tagged::from_item(Value::string(value), Tag::unknown())
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
            .begin_filter(CallStub::new().with_long_flag("downcase").create())
            .is_ok());
        assert_eq!(plugin.action.unwrap(), Action::Downcase);
    }

    #[test]
    fn str_plugin_accepts_upcase() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(CallStub::new().with_long_flag("upcase").create())
            .is_ok());
        assert_eq!(plugin.action.unwrap(), Action::Upcase);
    }

    #[test]
    fn str_plugin_accepts_to_int() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(CallStub::new().with_long_flag("to-int").create())
            .is_ok());
        assert_eq!(plugin.action.unwrap(), Action::ToInteger);
    }
    #[test]
    fn str_plugin_accepts_field() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_parameter("package.description")
                    .create()
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
                    .create(),
            )
            .is_err());
        assert_eq!(plugin.error, Some("can only apply one".to_string()));
    }

    #[test]
    fn str_downcases() {
        let mut strutils = Str::new();
        strutils.for_downcase();
        assert_eq!(strutils.apply("ANDRES").unwrap(), Value::string("andres"));
    }

    #[test]
    fn str_upcases() {
        let mut strutils = Str::new();
        strutils.for_upcase();
        assert_eq!(strutils.apply("andres").unwrap(), Value::string("ANDRES"));
    }

    #[test]
    fn str_to_int() {
        let mut strutils = Str::new();
        strutils.for_to_int();
        assert_eq!(strutils.apply("9999").unwrap(), Value::int(9999 as i64));
    }

    #[test]
    fn str_plugin_applies_upcase_with_field() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_long_flag("upcase")
                    .with_parameter("name")
                    .create()
            )
            .is_ok());

        let subject = structured_sample_record("name", "jotandrehuda");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Row(o),
                ..
            }) => assert_eq!(
                *o.get_data(&String::from("name")).borrow(),
                Value::string(String::from("JOTANDREHUDA"))
            ),
            _ => {}
        }
    }

    #[test]
    fn str_plugin_applies_upcase_without_field() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(CallStub::new().with_long_flag("upcase").create())
            .is_ok());

        let subject = unstructured_sample_record("jotandrehuda");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Primitive(Primitive::String(s)),
                ..
            }) => assert_eq!(*s, String::from("JOTANDREHUDA")),
            _ => {}
        }
    }

    #[test]
    fn str_plugin_applies_downcase_with_field() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_long_flag("downcase")
                    .with_parameter("name")
                    .create()
            )
            .is_ok());

        let subject = structured_sample_record("name", "JOTANDREHUDA");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Row(o),
                ..
            }) => assert_eq!(
                *o.get_data(&String::from("name")).borrow(),
                Value::string(String::from("jotandrehuda"))
            ),
            _ => {}
        }
    }

    #[test]
    fn str_plugin_applies_downcase_without_field() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(CallStub::new().with_long_flag("downcase").create())
            .is_ok());

        let subject = unstructured_sample_record("JOTANDREHUDA");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Primitive(Primitive::String(s)),
                ..
            }) => assert_eq!(*s, String::from("jotandrehuda")),
            _ => {}
        }
    }

    #[test]
    fn str_plugin_applies_to_int_with_field() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_long_flag("to-int")
                    .with_parameter("Nu_birthday")
                    .create()
            )
            .is_ok());

        let subject = structured_sample_record("Nu_birthday", "10");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Row(o),
                ..
            }) => assert_eq!(
                *o.get_data(&String::from("Nu_birthday")).borrow(),
                Value::int(10)
            ),
            _ => {}
        }
    }

    #[test]
    fn str_plugin_applies_to_int_without_field() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(CallStub::new().with_long_flag("to-int").create())
            .is_ok());

        let subject = unstructured_sample_record("10");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Primitive(Primitive::Int(i)),
                ..
            }) => assert_eq!(*i, BigInt::from(10)),
            _ => {}
        }
    }
}
