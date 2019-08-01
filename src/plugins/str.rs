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
        (self.downcase && !self.upcase) || (!self.downcase && self.upcase)
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
        "Usage: str [--downcase, --upcase]"
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
                None => Err(ShellError::string(
                    "str needs a field when applying it to a value in an object",
                )),
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
            None => {}
        }

        Ok(vec![])
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
