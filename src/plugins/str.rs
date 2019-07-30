use indexmap::IndexMap;
use nu::{
    serve_plugin, CallInfo, CommandConfig, NamedType, Plugin, PositionalType, Primitive,
    ReturnSuccess, ReturnValue, ShellError, Tagged, Value,
};

struct Str {
    field: Option<String>,
    error: Option<String>,
    downcase: bool,
    upcase: bool,
    int: bool,
}

impl Str {
    fn new() -> Str {
        Str {
            field: None,
            error: None,
            downcase: false,
            upcase: false,
            int: false,
        }
    }

    fn actions_desired(&self) -> u8 {
        [self.downcase, self.upcase, self.int].iter().fold(
            0,
            |acc, &field| {
                if field {
                    acc + 1
                } else {
                    acc
                }
            },
        )
    }

    fn is_valid(&self) -> bool {
        self.at_most_one() || self.none()
    }

    fn at_most_one(&self) -> bool {
        self.actions_desired() == 1
    }

    fn none(&self) -> bool {
        self.actions_desired() == 0
    }

    fn log_error(&mut self, message: &str) {
        self.error = Some(message.to_string());
    }

    fn for_input(&mut self, field: String) {
        self.field = Some(field);
    }

    fn for_to_int(&mut self) {
        self.int = true;

        if !self.is_valid() {
            self.log_error("can only apply one")
        }
    }

    fn for_downcase(&mut self) {
        self.downcase = true;

        if !self.is_valid() {
            self.log_error("can only apply one")
        }
    }

    fn for_upcase(&mut self) {
        self.upcase = true;

        if !self.is_valid() {
            self.log_error("can only apply one")
        }
    }

    fn apply(&self, input: &str) -> Value {
        if self.downcase {
            return Value::string(input.to_ascii_lowercase());
        }

        if self.upcase {
            return Value::string(input.to_ascii_uppercase());
        }

        if self.int {
            match input.trim().parse::<i64>() {
                Ok(v) => return Value::int(v),
                Err(_) => return Value::string(input),
            }
        }

        Value::string(input.to_string())
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
            Value::Primitive(Primitive::String(s)) => Ok(Spanned {
                item: self.apply(&s),
                span: value.span,
            }),
            Value::Object(_) => match field {
                Some(f) => {
                    let replacement = match value.item.get_data_by_path(value.span(), f) {
                        Some(result) => self.strutils(result.map(|x| x.clone()), &None)?,
                        None => {
                            return Err(ShellError::string("str could not find field to replace"))
                        }
                    };
                    match value
                        .item
                        .replace_data_at_path(value.span(), f, replacement.item.clone())
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
    fn config(&mut self) -> Result<CommandConfig, ShellError> {
        let mut named = IndexMap::new();
        named.insert("downcase".to_string(), NamedType::Switch);
        named.insert("upcase".to_string(), NamedType::Switch);
        named.insert("to-int".to_string(), NamedType::Switch);

        Ok(CommandConfig {
            name: "str".to_string(),
            positional: vec![PositionalType::optional_any("Field")],
            is_filter: true,
            is_sink: false,
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
        Args, CallInfo, Plugin, ReturnSuccess, SourceMap, Span, Spanned, SpannedDictBuilder,
        SpannedItem, Value,
    };

    struct CallStub {
        positionals: Vec<Spanned<Value>>,
        flags: IndexMap<String, Spanned<Value>>,
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
                Value::boolean(true).spanned(Span::unknown()),
            );
            self
        }

        fn with_parameter(&mut self, name: &str) -> &mut Self {
            self.positionals
                .push(Value::string(name.to_string()).spanned(Span::unknown()));
            self
        }

        fn create(&self) -> CallInfo {
            CallInfo {
                args: Args::new(Some(self.positionals.clone()), Some(self.flags.clone())),
                source_map: SourceMap::new(),
                name_span: None,
            }
        }
    }

    fn sample_record(key: &str, value: &str) -> Spanned<Value> {
        let mut record = SpannedDictBuilder::new(Span::unknown());
        record.insert(key.clone(), Value::string(value));
        record.into_spanned_value()
    }

    #[test]
    fn str_accepts_downcase() {
        let mut strutils = Str::new();

        assert!(strutils
            .begin_filter(CallStub::new().with_long_flag("downcase").create())
            .is_ok());
        assert!(strutils.is_valid());
        assert!(strutils.downcase);
    }

    #[test]
    fn str_accepts_upcase() {
        let mut strutils = Str::new();

        assert!(strutils
            .begin_filter(CallStub::new().with_long_flag("upcase").create())
            .is_ok());
        assert!(strutils.is_valid());
        assert!(strutils.upcase);
    }

    #[test]
    fn str_accepts_to_int() {
        let mut strutils = Str::new();

        assert!(strutils
            .begin_filter(CallStub::new().with_long_flag("to-int").create())
            .is_ok());
        assert!(strutils.is_valid());
        assert!(strutils.int);
    }

    #[test]
    fn str_accepts_only_one_action() {
        let mut strutils = Str::new();

        assert!(strutils
            .begin_filter(
                CallStub::new()
                    .with_long_flag("upcase")
                    .with_long_flag("downcase")
                    .with_long_flag("to-int")
                    .create(),
            )
            .is_err());
        assert!(!strutils.is_valid());
        assert_eq!(Some("can only apply one".to_string()), strutils.error);
    }

    #[test]
    fn str_accepts_field() {
        let mut strutils = Str::new();

        assert!(strutils
            .begin_filter(
                CallStub::new()
                    .with_parameter("package.description")
                    .create()
            )
            .is_ok());

        assert_eq!(Some("package.description".to_string()), strutils.field);
    }

    #[test]
    fn str_downcases() {
        let mut strutils = Str::new();
        strutils.for_downcase();
        assert_eq!(Value::string("andres"), strutils.apply("ANDRES"));
    }

    #[test]
    fn str_upcases() {
        let mut strutils = Str::new();
        strutils.for_upcase();
        assert_eq!(Value::string("ANDRES"), strutils.apply("andres"));
    }

    #[test]
    fn str_to_int() {
        let mut strutils = Str::new();
        strutils.for_to_int();
        assert_eq!(Value::int(9999 as i64), strutils.apply("9999"));
    }

    #[test]
    fn str_plugin_applies_upcase() {
        let mut strutils = Str::new();

        assert!(strutils
            .begin_filter(
                CallStub::new()
                    .with_long_flag("upcase")
                    .with_parameter("name")
                    .create()
            )
            .is_ok());

        let subject = sample_record("name", "jotandrehuda");
        let output = strutils.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Spanned {
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
        let mut strutils = Str::new();

        assert!(strutils
            .begin_filter(
                CallStub::new()
                    .with_long_flag("downcase")
                    .with_parameter("name")
                    .create()
            )
            .is_ok());

        let subject = sample_record("name", "JOTANDREHUDA");
        let output = strutils.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Spanned {
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
        let mut strutils = Str::new();

        assert!(strutils
            .begin_filter(
                CallStub::new()
                    .with_long_flag("to-int")
                    .with_parameter("Nu_birthday")
                    .create()
            )
            .is_ok());

        let subject = sample_record("Nu_birthday", "10");
        let output = strutils.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Spanned {
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
