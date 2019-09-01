use nu::{
    serve_plugin, CallInfo, Plugin, Primitive, ReturnSuccess, ReturnValue, ShellError, Signature,
    SyntaxType, Tagged, Value,
};
use regex::Regex;

#[derive(Debug, Eq, PartialEq)]
enum Action {
    Downcase,
    Upcase,
    ToInteger,
    Replace(ReplaceAction),
}

#[derive(Debug, Eq, PartialEq)]
enum ReplaceAction {
    Direct,
    FindAndReplace,
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
            Some(Action::Replace(ref mode)) => match mode {
                ReplaceAction::Direct => Value::string(self.first_param()),
                ReplaceAction::FindAndReplace => {
                    let regex = Regex::new(self.first_param());

                    match regex {
                        Ok(re) => Value::string(re.replace(input, self.second_param()).to_owned()),
                        Err(_) => Value::string(input),
                    }
                }
            },
            None => Value::string(input),
        };

        Ok(applied)
    }

    fn did_supply_field(&self) -> bool {
        self.field.is_some()
    }

    fn first_param(&self) -> &str {
        let idx = if self.did_supply_field() { 1 } else { 0 };
        self.get_param(idx)
    }

    fn second_param(&self) -> &str {
        let idx = if self.did_supply_field() { 2 } else { 1 };
        self.get_param(idx)
    }

    fn get_param(&self, idx: usize) -> &str {
        self.params.as_ref().unwrap().get(idx).unwrap().as_str()
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

    fn for_replace(&mut self, mode: ReplaceAction) {
        if self.permit() {
            self.action = Some(Action::Replace(mode));
        } else {
            self.log_error("can only apply one");
        }
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
        "Usage: str field [--downcase|--upcase|--to-int|--replace|--find-replace]"
    }
}

impl Str {
    fn strutils(&self, value: Tagged<Value>) -> Result<Tagged<Value>, ShellError> {
        match value.item {
            Value::Primitive(Primitive::String(ref s)) => {
                Ok(Tagged::from_item(self.apply(&s)?, value.tag()))
            }
            Value::Object(_) => match self.field {
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
                    "str needs a field when applying it to a value in an object",
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
            .switch("replace")
            .switch("find-replace")
            .rest(SyntaxType::Member)
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
        if args.has("replace") {
            self.for_replace(ReplaceAction::Direct);
        }
        if args.has("find-replace") {
            self.for_replace(ReplaceAction::FindAndReplace);
        }

        if let Some(possible_field) = args.nth(0) {
            match possible_field {
                Tagged {
                    item: Value::Primitive(Primitive::String(s)),
                    ..
                } => match self.action {
                    Some(Action::Replace(ReplaceAction::Direct)) => {
                        if args.len() == 2 {
                            self.for_field(&s);
                        }
                    }
                    Some(Action::Replace(ReplaceAction::FindAndReplace)) => {
                        if args.len() == 3 {
                            self.for_field(&s);
                        }
                    }
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

    use super::{Action, ReplaceAction, Str};
    use indexmap::IndexMap;
    use nu::{
        CallInfo, EvaluatedArgs, Plugin, Primitive, ReturnSuccess, SourceMap, Span, Tag, Tagged,
        TaggedDictBuilder, TaggedItem, Value,
    };

    impl Str {
        fn replace_with(&mut self, value: &str) {
            self.params.as_mut().unwrap().push(value.to_string());
        }

        fn find_with(&mut self, search: &str) {
            self.params.as_mut().unwrap().push(search.to_string());
        }
    }

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

        fn create(&self) -> CallInfo {
            CallInfo {
                args: EvaluatedArgs::new(Some(self.positionals.clone()), Some(self.flags.clone())),
                source_map: SourceMap::new(),
                name_span: Span::unknown(),
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

        for action_flag in &["downcase", "upcase", "to-int", "replace", "find-replace"] {
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

        assert!(plugin
            .begin_filter(CallStub::new().with_long_flag("replace").create())
            .is_ok());
        assert_eq!(
            plugin.action.unwrap(),
            Action::Replace(ReplaceAction::Direct)
        );
    }

    #[test]
    fn str_plugin_accepts_find_replace() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(CallStub::new().with_long_flag("find-replace").create())
            .is_ok());
        assert_eq!(
            plugin.action.unwrap(),
            Action::Replace(ReplaceAction::FindAndReplace)
        );
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
    fn str_replace() {
        let mut strutils = Str::new();
        strutils.for_replace(ReplaceAction::Direct);
        strutils.replace_with("robalino");
        assert_eq!(strutils.apply("andres").unwrap(), Value::string("robalino"));
    }

    #[test]
    fn str_find_replace() {
        let mut strutils = Str::new();
        strutils.for_replace(ReplaceAction::FindAndReplace);
        strutils.find_with(r"kittens");
        strutils.replace_with("jotandrehuda");
        assert_eq!(
            strutils.apply("wykittens").unwrap(),
            Value::string("wyjotandrehuda")
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
                item: Value::Object(o),
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
            }) => assert_eq!(*i, 10),
            _ => {}
        }
    }

    #[test]
    fn str_plugin_applies_replace_with_field() {
        let mut plugin = Str::new();

        assert!(plugin
            .begin_filter(
                CallStub::new()
                    .with_parameter("rustconf")
                    .with_parameter("22nd August 2019")
                    .with_long_flag("replace")
                    .create()
            )
            .is_ok());

        let subject = structured_sample_record("rustconf", "1st January 1970");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Object(o),
                ..
            }) => assert_eq!(
                *o.get_data(&String::from("rustconf")).borrow(),
                Value::string(String::from("22nd August 2019"))
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
                    .with_parameter("22nd August 2019")
                    .with_long_flag("replace")
                    .create()
            )
            .is_ok());

        let subject = unstructured_sample_record("1st January 1970");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Primitive(Primitive::String(s)),
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
                    .with_parameter("kittens")
                    .with_parameter("jotandrehuda")
                    .with_long_flag("find-replace")
                    .create()
            )
            .is_ok());

        let subject = structured_sample_record("staff", "wykittens");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Object(o),
                ..
            }) => assert_eq!(
                *o.get_data(&String::from("staff")).borrow(),
                Value::string(String::from("wyjotandrehuda"))
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
                    .with_parameter("kittens")
                    .with_parameter("jotandrehuda")
                    .with_long_flag("find-replace")
                    .create()
            )
            .is_ok());

        let subject = unstructured_sample_record("wykittens");
        let output = plugin.filter(subject).unwrap();

        match output[0].as_ref().unwrap() {
            ReturnSuccess::Value(Tagged {
                item: Value::Primitive(Primitive::String(s)),
                ..
            }) => assert_eq!(*s, String::from("wyjotandrehuda")),
            _ => {}
        }
    }
}
