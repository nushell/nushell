use nu::{did_you_mean, serve_plugin, value, Plugin, ValueExt};
use nu_errors::ShellError;
use nu_protocol::{
    CallInfo, ColumnPath, Primitive, ReturnSuccess, ReturnValue, ShellTypeName, Signature,
    SyntaxShape, UntaggedValue, Value,
};
use nu_source::{span_for_spanned_list, Tagged};

use regex::Regex;
use std::cmp;

#[derive(Debug, Eq, PartialEq)]
enum Action {
    Downcase,
    Upcase,
    ToInteger,
    Substring(usize, usize),
    Replace(ReplaceAction),
}

#[derive(Debug, Eq, PartialEq)]
enum ReplaceAction {
    Direct(String),
    FindAndReplace(String, String),
}

struct Str {
    field: Option<Tagged<ColumnPath>>,
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

    fn apply(&self, input: &str) -> Result<UntaggedValue, ShellError> {
        let applied = match self.action.as_ref() {
            Some(Action::Downcase) => value::string(input.to_ascii_lowercase()),
            Some(Action::Upcase) => value::string(input.to_ascii_uppercase()),
            Some(Action::Substring(s, e)) => {
                let end: usize = cmp::min(*e, input.len());
                let start: usize = *s;
                if start > input.len() - 1 {
                    value::string("")
                } else {
                    value::string(
                        &input
                            .chars()
                            .skip(start)
                            .take(end - start)
                            .collect::<String>(),
                    )
                }
            }
            Some(Action::Replace(mode)) => match mode {
                ReplaceAction::Direct(replacement) => value::string(replacement.as_str()),
                ReplaceAction::FindAndReplace(find, replacement) => {
                    let regex = Regex::new(find.as_str());

                    match regex {
                        Ok(re) => value::string(re.replace(input, replacement.as_str()).to_owned()),
                        Err(_) => value::string(input),
                    }
                }
            },
            Some(Action::ToInteger) => match input.trim() {
                other => match other.parse::<i64>() {
                    Ok(v) => value::int(v),
                    Err(_) => value::string(input),
                },
            },
            None => value::string(input),
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

    fn for_replace(&mut self, mode: ReplaceAction) {
        if self.permit() {
            self.action = Some(Action::Replace(mode));
        } else {
            self.log_error("can only apply one");
        }
    }

    pub fn usage() -> &'static str {
        "Usage: str field [--downcase|--upcase|--to-int|--substring \"start,end\"|--replace|--find-replace [pattern replacement]]]"
    }
}

impl Str {
    fn strutils(&self, value: Value) -> Result<Value, ShellError> {
        match &value.value {
            UntaggedValue::Primitive(Primitive::String(ref s)) => {
                Ok(self.apply(&s)?.into_value(value.tag()))
            }
            UntaggedValue::Row(_) => match self.field {
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
                    let replacement = self.strutils(got.clone())?;

                    match value.replace_data_at_column_path(
                        &f,
                        replacement.value.clone().into_untagged_value(),
                    ) {
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
                value.type_name(),
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
            .named("replace", SyntaxShape::String, "replaces the string")
            .named(
                "find-replace",
                SyntaxShape::Any,
                "finds and replaces [pattern replacement]",
            )
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
                    Value {
                        value: UntaggedValue::Primitive(Primitive::String(s)),
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
        if args.has("replace") {
            if let Some(Value {
                value: UntaggedValue::Primitive(Primitive::String(replacement)),
                ..
            }) = args.get("replace")
            {
                self.for_replace(ReplaceAction::Direct(replacement.clone()));
            }
        }

        if args.has("find-replace") {
            if let Some(Value {
                value: UntaggedValue::Table(arguments),
                ..
            }) = args.get("find-replace")
            {
                self.for_replace(ReplaceAction::FindAndReplace(
                    arguments.get(0).unwrap().as_string()?.to_string(),
                    arguments.get(1).unwrap().as_string()?.to_string(),
                ));
            }
        }

        if let Some(possible_field) = args.nth(0) {
            let possible_field = possible_field.as_column_path()?;
            self.for_field(possible_field);
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

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![ReturnSuccess::value(self.strutils(input)?)])
    }
}

fn main() {
    serve_plugin(&mut Str::new());
}

#[cfg(test)]
mod tests {
    use super::{Action, ReplaceAction, Str};
    use indexmap::IndexMap;
    use nu::{value, Plugin, TaggedDictBuilder, ValueExt};
    use nu_protocol::{CallInfo, EvaluatedArgs, Primitive, ReturnSuccess, UntaggedValue, Value};
    use nu_source::Tag;
    use num_bigint::BigInt;

    fn string(input: impl Into<String>) -> Value {
        value::string(input.into()).into_untagged_value()
    }

    fn table(list: &Vec<Value>) -> Value {
        value::table(list).into_untagged_value()
    }

    fn column_path(paths: &Vec<Value>) -> Value {
        UntaggedValue::Primitive(Primitive::ColumnPath(
            table(&paths.iter().cloned().collect())
                .as_column_path()
                .unwrap()
                .item,
        ))
        .into_untagged_value()
    }
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

        fn with_named_parameter(&mut self, name: &str, value: Value) -> &mut Self {
            self.flags.insert(name.to_string(), value);
            self
        }

        fn with_long_flag(&mut self, name: &str) -> &mut Self {
            self.flags.insert(
                name.to_string(),
                value::boolean(true).into_value(Tag::unknown()),
            );
            self
        }

        fn with_parameter(&mut self, name: &str) -> &mut Self {
            let fields: Vec<Value> = name
                .split(".")
                .map(|s| value::string(s.to_string()).into_value(Tag::unknown()))
                .collect();

            self.positionals.push(column_path(&fields));
            self
        }

        fn create(&self) -> CallInfo {
            CallInfo {
                args: EvaluatedArgs::new(Some(self.positionals.clone()), Some(self.flags.clone())),
                name_tag: Tag::unknown(),
            }
        }
    }

    fn structured_sample_record(key: &str, value: &str) -> Value {
        let mut record = TaggedDictBuilder::new(Tag::unknown());
        record.insert_untagged(key.clone(), value::string(value));
        record.into_value()
    }

    fn unstructured_sample_record(value: &str) -> Value {
        value::string(value).into_value(Tag::unknown())
    }

    #[test]
    fn str_plugin_configuration_flags_wired() {
        let mut plugin = Str::new();

        let configured = plugin.config().unwrap();

        for action_flag in &[
            "downcase",
            "upcase",
            "to-int",
            "substring",
            "replace",
            "find-replace",
        ] {
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
    fn str_plugin_accepts_replace() {
        let mut plugin = Str::new();

        let argument = String::from("replace_text");

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_named_parameter("replace", string(&argument))
                    .create()
            )
            .is_ok());

        match plugin.action {
            Some(Action::Replace(ReplaceAction::Direct(replace_with))) => {
                assert_eq!(replace_with, argument)
            }
            Some(_) | None => panic!("Din't accept."),
        }
    }

    #[test]
    fn str_plugin_accepts_find_replace() {
        let mut plugin = Str::new();

        let search_argument = String::from("kittens");
        let replace_argument = String::from("jotandrehuda");

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_named_parameter(
                        "find-replace",
                        table(&vec![string(&search_argument), string(&replace_argument)])
                    )
                    .create()
            )
            .is_ok());

        match plugin.action {
            Some(Action::Replace(ReplaceAction::FindAndReplace(find_with, replace_with))) => {
                assert_eq!(find_with, search_argument);
                assert_eq!(replace_with, replace_argument);
            }
            Some(_) | None => panic!("Din't accept."),
        }
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

        let actual = &*plugin.field.unwrap();
        let actual = UntaggedValue::Primitive(Primitive::ColumnPath(actual.clone()));
        let actual = actual.into_value(Tag::unknown());

        assert_eq!(
            actual,
            column_path(&vec![string("package"), string("description")])
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
        assert_eq!(strutils.apply("ANDRES").unwrap(), value::string("andres"));
    }

    #[test]
    fn str_upcases() {
        let mut strutils = Str::new();
        strutils.for_upcase();
        assert_eq!(strutils.apply("andres").unwrap(), value::string("ANDRES"));
    }

    #[test]
    fn str_to_int() {
        let mut strutils = Str::new();
        strutils.for_to_int();
        assert_eq!(strutils.apply("9999").unwrap(), value::int(9999 as i64));
    }

    #[test]
    fn str_replace() {
        let mut strutils = Str::new();
        strutils.for_replace(ReplaceAction::Direct("robalino".to_string()));

        assert_eq!(strutils.apply("andres").unwrap(), value::string("robalino"));
    }

    #[test]
    fn str_find_replace() {
        let mut strutils = Str::new();
        strutils.for_replace(ReplaceAction::FindAndReplace(
            "kittens".to_string(),
            "jotandrehuda".to_string(),
        ));
        assert_eq!(
            strutils.apply("wykittens").unwrap(),
            value::string("wyjotandrehuda")
        );
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
            ReturnSuccess::Value(Value {
                value: UntaggedValue::Row(o),
                ..
            }) => assert_eq!(
                *o.get_data(&String::from("name")).borrow(),
                value::string(String::from("JOTANDREHUDA")).into_untagged_value()
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
            ReturnSuccess::Value(Value {
                value: UntaggedValue::Primitive(Primitive::String(s)),
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
            ReturnSuccess::Value(Value {
                value: UntaggedValue::Row(o),
                ..
            }) => assert_eq!(
                *o.get_data(&String::from("name")).borrow(),
                value::string(String::from("jotandrehuda")).into_untagged_value()
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
            ReturnSuccess::Value(Value {
                value: UntaggedValue::Primitive(Primitive::String(s)),
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
            ReturnSuccess::Value(Value {
                value: UntaggedValue::Row(o),
                ..
            }) => assert_eq!(
                *o.get_data(&String::from("Nu_birthday")).borrow(),
                value::int(10).into_untagged_value()
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
            ReturnSuccess::Value(Value {
                value: UntaggedValue::Primitive(Primitive::Int(i)),
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
                    .with_named_parameter("substring", string("0,1"))
                    .create()
            )
            .is_ok());

        let subject = unstructured_sample_record("0123456789");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Value {
                value: UntaggedValue::Primitive(Primitive::String(s)),
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
                    .with_named_parameter("substring", string("0,11"))
                    .create()
            )
            .is_ok());

        let subject = unstructured_sample_record("0123456789");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Value {
                value: UntaggedValue::Primitive(Primitive::String(s)),
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
                    .with_named_parameter("substring", string("20,30"))
                    .create()
            )
            .is_ok());

        let subject = unstructured_sample_record("0123456789");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Value {
                value: UntaggedValue::Primitive(Primitive::String(s)),
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
                    .with_named_parameter("substring", string(",5"))
                    .create()
            )
            .is_ok());

        let subject = unstructured_sample_record("0123456789");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Value {
                value: UntaggedValue::Primitive(Primitive::String(s)),
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
                    .with_named_parameter("substring", string("2,"))
                    .create()
            )
            .is_ok());

        let subject = unstructured_sample_record("0123456789");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Value {
                value: UntaggedValue::Primitive(Primitive::String(s)),
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
                    .with_named_parameter("substring", string("3,1"))
                    .create()
            )
            .is_err());
        assert_eq!(
            plugin.error,
            Some("End must be greater than or equal to Start".to_string())
        );
    }

    #[test]
    fn str_plugin_applies_replace_with_field() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_parameter("rustconf")
                    .with_named_parameter("replace", string("22nd August 2019"))
                    .create()
            )
            .is_ok());

        let subject = structured_sample_record("rustconf", "1st January 1970");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Value {
                value: UntaggedValue::Row(o),
                ..
            }) => assert_eq!(
                *o.get_data(&String::from("rustconf")).borrow(),
                Value {
                    value: value::string(String::from("22nd August 2019")),
                    tag: Tag::unknown()
                }
            ),
            _ => {}
        }
    }

    #[test]
    fn str_plugin_applies_replace_without_field() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_named_parameter("replace", string("22nd August 2019"))
                    .create()
            )
            .is_ok());

        let subject = unstructured_sample_record("1st January 1970");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Value {
                value: UntaggedValue::Primitive(Primitive::String(s)),
                ..
            }) => assert_eq!(*s, String::from("22nd August 2019")),
            _ => {}
        }
    }

    #[test]
    fn str_plugin_applies_find_replace_with_field() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_parameter("staff")
                    .with_named_parameter(
                        "find-replace",
                        table(&vec![string("kittens"), string("jotandrehuda")])
                    )
                    .create()
            )
            .is_ok());

        let subject = structured_sample_record("staff", "wykittens");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Value {
                value: UntaggedValue::Row(o),
                ..
            }) => assert_eq!(
                *o.get_data(&String::from("staff")).borrow(),
                Value {
                    value: value::string(String::from("wyjotandrehuda")),
                    tag: Tag::unknown()
                }
            ),
            _ => {}
        }
    }

    #[test]
    fn str_plugin_applies_find_replace_without_field() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_named_parameter(
                        "find-replace",
                        table(&vec![string("kittens"), string("jotandrehuda")])
                    )
                    .create()
            )
            .is_ok());

        let subject = unstructured_sample_record("wykittens");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Value {
                value: UntaggedValue::Primitive(Primitive::String(s)),
                ..
            }) => assert_eq!(*s, String::from("wyjotandrehuda")),
            _ => {}
        }
    }
}
