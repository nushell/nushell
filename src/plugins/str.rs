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
}

impl Str {
    fn new() -> Str {
        Str {
            field: None,
            error: None,
            downcase: false,
            upcase: false,
        }
    }

    fn is_valid(&self) -> bool {
        self.at_least_one() || self.none()
    }

    fn at_least_one(&self) -> bool {
        (self.downcase && !self.upcase) || (!self.downcase && self.upcase)
    }

    fn none(&self) -> bool {
        (!self.downcase && !self.upcase)
    }

    fn log_error(&mut self, message: &str) {
        self.error = Some(message.to_string());
    }

    fn for_input(&mut self, field: String) {
        self.field = Some(field);
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

    fn apply(&self, input: &str) -> String {
        if self.downcase {
            return input.to_ascii_lowercase();
        }

        if self.upcase {
            return input.to_ascii_uppercase();
        }

        input.to_string()
    }

    fn usage(&self) -> &'static str {
        "Usage: str field [--downcase|--upcase]"
    }
}

impl Str {
    fn strutils(
        &self,
        value: Tagged<Value>,
        field: &Option<String>,
    ) -> Result<Tagged<Value>, ShellError> {
        match value.item {
            Value::Primitive(Primitive::String(ref s)) => Ok(Tagged::from_item(
                Value::string(self.apply(&s)),
                value.span(),
            )),
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

    fn sample_record(value: &str) -> Spanned<Value> {
        let mut record = SpannedDictBuilder::new(Span::unknown());
        record.insert_spanned(
            "name",
            Value::string(value.to_string()).spanned(Span::unknown()),
        );
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
    fn str_accepts_only_one_flag() {
        let mut strutils = Str::new();

        assert!(strutils
            .begin_filter(
                CallStub::new()
                    .with_long_flag("upcase")
                    .with_long_flag("downcase")
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
    fn str_reports_error_if_no_field_given_for_object() {
        let mut strutils = Str::new();
        let subject = sample_record("jotandrehuda");

        assert!(strutils.begin_filter(CallStub::new().create()).is_ok());
        assert!(strutils.filter(subject).is_err());
    }

    #[test]
    fn str_downcases() {
        let mut strutils = Str::new();
        strutils.for_downcase();
        assert_eq!("andres", strutils.apply("ANDRES"));
    }

    #[test]
    fn str_upcases() {
        let mut strutils = Str::new();
        strutils.for_upcase();
        assert_eq!("ANDRES", strutils.apply("andres"));
    }

    #[test]
    fn str_applies_upcase() {
        let mut strutils = Str::new();

        assert!(strutils
            .begin_filter(
                CallStub::new()
                    .with_long_flag("upcase")
                    .with_parameter("name")
                    .create()
            )
            .is_ok());

        let subject = sample_record("jotandrehuda");
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
    fn str_applies_downcase() {
        let mut strutils = Str::new();

        assert!(strutils
            .begin_filter(
                CallStub::new()
                    .with_long_flag("downcase")
                    .with_parameter("name")
                    .create()
            )
            .is_ok());

        let subject = sample_record("JOTANDREHUDA");
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
}
