use nu::{
    did_you_mean, serve_plugin, span_for_spanned_list, CallInfo, ColumnPath, Plugin, Primitive,
    ReturnSuccess, ReturnValue, ShellError, ShellTypeName, Signature, SyntaxShape, Tagged,
    TaggedItem, Value,
};
use std::cmp;

#[derive(Debug, Eq, PartialEq)]
enum Action {
    Downcase,
    Upcase,
    ToInteger,
    Substring(usize, usize),
}

struct Str {
    field: Option<Tagged<ColumnPath>>,
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
            Some(Action::Substring(s, e)) => {
                let end: usize = cmp::min(*e, input.len());
                let start: usize = *s;
                if start > input.len() - 1 {
                    Value::string("")
                } else {
                    Value::string(
                        &input
                            .chars()
                            .skip(start)
                            .take(end - start)
                            .collect::<String>(),
                    )
                }
            }
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

    fn for_field(&mut self, column_path: Tagged<ColumnPath>) {
        self.field = Some(column_path);
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

    fn for_substring(&mut self, s: String) {
        let v: Vec<&str> = s.split(',').collect();
        let start: usize = match v[0] {
            "" => 0,
            _ => v[0].trim().parse().unwrap(),
        };
        let end: usize = match v[1] {
            "" => usize::max_value().clone(),
            _ => v[1].trim().parse().unwrap(),
        };
        if start > end {
            self.log_error("End must be greater than or equal to Start");
        } else if self.permit() {
            self.action = Some(Action::Substring(start, end));
        } else {
            self.log_error("can only apply one");
        }
    }

    pub fn usage() -> &'static str {
        "Usage: str field [--downcase|--upcase|--to-int|--substring \"start,end\"]"
    }
}

impl Str {
    fn strutils(&self, value: Tagged<Value>) -> Result<Tagged<Value>, ShellError> {
        match value.item {
            Value::Primitive(Primitive::String(ref s)) => Ok(self.apply(&s)?.tagged(value.tag())),
            Value::Row(_) => match self.field {
                Some(ref f) => {
                    let fields = f.clone();

                    let replace_for =
                        value.get_data_by_column_path(
                            &f,
                            Box::new(move |(obj_source, column_path_tried, error)| {
                                match did_you_mean(&obj_source, &column_path_tried) {
                                    Some(suggestions) => {
                                        return ShellError::labeled_error(
                                            "Unknown column",
                                            format!("did you mean '{}'?", suggestions[0].1),
                                            span_for_spanned_list(fields.iter().map(|p| p.span)),
                                        )
                                    }
                                    None => return error,
                                }
                            }),
                        );

                    let got = replace_for?;
                    let replacement = self.strutils(got.map(|x| x.clone()))?;

                    match value.replace_data_at_column_path(&f, replacement.item.clone()) {
                        Some(v) => return Ok(v),
                        None => Err(ShellError::labeled_error(
                            "str could not find field to replace",
                            "column name",
                            value.tag(),
                        )),
                    }
                }
                None => Err(ShellError::untagged_runtime_error(format!(
                    "{}: {}",
                    "str needs a column when applied to a value in a row",
                    Str::usage()
                ))),
            },
            _ => Err(ShellError::labeled_error(
                "Unrecognized type in stream",
                value.item.type_name(),
                value.tag,
            )),
        }
    }
}

impl Plugin for Str {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("str")
            .desc("Apply string function. Optional use the column of a table")
            .switch("downcase", "convert string to lowercase")
            .switch("upcase", "convert string to uppercase")
            .switch("to-int", "convert string to integer")
            .named(
                "substring",
                SyntaxShape::String,
                "convert string to portion of original, requires \"start,end\"",
            )
            .rest(SyntaxShape::ColumnPath, "the column(s) to convert")
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
        if args.has("substring") {
            if let Some(start_end) = args.get("substring") {
                match start_end {
                    Tagged {
                        item: Value::Primitive(Primitive::String(s)),
                        ..
                    } => {
                        self.for_substring(s.to_string());
                    }
                    _ => {
                        return Err(ShellError::labeled_error(
                            "Unrecognized type in params",
                            start_end.type_name(),
                            &start_end.tag,
                        ))
                    }
                }
            }
        }

        if let Some(possible_field) = args.nth(0) {
            let possible_field = possible_field.as_column_path()?;

            self.for_field(possible_field);
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
                return Err(ShellError::untagged_runtime_error(format!(
                    "{}: {}",
                    reason,
                    Str::usage()
                )))
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
        CallInfo, EvaluatedArgs, Plugin, Primitive, RawPathMember, ReturnSuccess, Tag, Tagged,
        TaggedDictBuilder, TaggedItem, Value,
    };
    use num_bigint::BigInt;

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

        fn with_named_parameter(&mut self, name: &str, value: &str) -> &mut Self {
            self.flags.insert(
                name.to_string(),
                Value::string(value).tagged(Tag::unknown()),
            );
            self
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
                .map(|s| Value::string(s.to_string()).tagged(Tag::unknown()))
                .collect();

            self.positionals
                .push(Value::Table(fields).tagged(Tag::unknown()));
            self
        }

        fn create(&self) -> CallInfo {
            CallInfo {
                args: EvaluatedArgs::new(Some(self.positionals.clone()), Some(self.flags.clone())),
                name_tag: Tag::unknown(),
            }
        }
    }

    fn structured_sample_record(key: &str, value: &str) -> Tagged<Value> {
        let mut record = TaggedDictBuilder::new(Tag::unknown());
        record.insert(key.clone(), Value::string(value));
        record.into_tagged_value()
    }

    fn unstructured_sample_record(value: &str) -> Tagged<Value> {
        Value::string(value).tagged(Tag::unknown())
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

        assert_eq!(
            plugin
                .field
                .map(|f| f.iter().cloned().map(|f| f.item).collect()),
            Some(vec![
                RawPathMember::String("package".to_string()),
                RawPathMember::String("description".to_string())
            ])
        )
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
                    .with_long_flag("substring")
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

    #[test]
    fn str_plugin_applies_substring_without_field() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_named_parameter("substring", "0,1")
                    .create()
            )
            .is_ok());

        let subject = unstructured_sample_record("0123456789");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Primitive(Primitive::String(s)),
                ..
            }) => assert_eq!(*s, String::from("0")),
            _ => {}
        }
    }

    #[test]
    fn str_plugin_applies_substring_exceeding_string_length() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_named_parameter("substring", "0,11")
                    .create()
            )
            .is_ok());

        let subject = unstructured_sample_record("0123456789");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Primitive(Primitive::String(s)),
                ..
            }) => assert_eq!(*s, String::from("0123456789")),
            _ => {}
        }
    }

    #[test]
    fn str_plugin_applies_substring_returns_blank_if_start_exceeds_length() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_named_parameter("substring", "20,30")
                    .create()
            )
            .is_ok());

        let subject = unstructured_sample_record("0123456789");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Primitive(Primitive::String(s)),
                ..
            }) => assert_eq!(*s, String::from("")),
            _ => {}
        }
    }

    #[test]
    fn str_plugin_applies_substring_treats_blank_start_as_zero() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_named_parameter("substring", ",5")
                    .create()
            )
            .is_ok());

        let subject = unstructured_sample_record("0123456789");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Primitive(Primitive::String(s)),
                ..
            }) => assert_eq!(*s, String::from("01234")),
            _ => {}
        }
    }

    #[test]
    fn str_plugin_applies_substring_treats_blank_end_as_length() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_named_parameter("substring", "2,")
                    .create()
            )
            .is_ok());

        let subject = unstructured_sample_record("0123456789");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Primitive(Primitive::String(s)),
                ..
            }) => assert_eq!(*s, String::from("23456789")),
            _ => {}
        }
    }

    #[test]
    fn str_plugin_applies_substring_returns_error_if_start_exceeds_end() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_named_parameter("substring", "3,1")
                    .create()
            )
            .is_err());
        assert_eq!(
            plugin.error,
            Some("End must be greater than or equal to Start".to_string())
        );
    }
}
