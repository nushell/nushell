use crate::Plugin;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{CallInfo, EvaluatedArgs, Primitive, ReturnValue, UntaggedValue, Value};
use nu_source::Tag;
use nu_value_ext::ValueExt;

pub struct PluginTest<'a, T: Plugin> {
    plugin: &'a mut T,
    call_info: CallInfo,
    input: Value,
}

impl<'a, T: Plugin> PluginTest<'a, T> {
    pub fn for_plugin(plugin: &'a mut T) -> Self {
        PluginTest {
            plugin: plugin,
            call_info: CallStub::new().create(),
            input: UntaggedValue::nothing().into_value(Tag::unknown()),
        }
    }

    pub fn args(&mut self, call_info: CallInfo) -> &mut PluginTest<'a, T> {
        self.call_info = call_info;
        self
    }

    pub fn configure(&mut self, callback: impl FnOnce(Vec<String>)) -> &mut PluginTest<'a, T> {
        let signature = self
            .plugin
            .config()
            .expect("There was a problem configuring the plugin.");
        callback(signature.named.keys().map(String::from).collect());
        self
    }

    pub fn input(&mut self, value: Value) -> &mut PluginTest<'a, T> {
        self.input = value;
        self
    }

    pub fn test(&mut self) -> Result<Vec<ReturnValue>, ShellError> {
        let return_values = self.plugin.filter(self.input.clone());

        let mut return_values = match return_values {
            Ok(filtered) => filtered,
            Err(reason) => return Err(reason),
        };

        let end = self.plugin.end_filter();

        match end {
            Ok(filter_ended) => return_values.extend(filter_ended),
            Err(reason) => return Err(reason),
        }

        self.plugin.quit();
        Ok(return_values)
    }

    pub fn setup(
        &mut self,
        callback: impl FnOnce(&mut T, Result<Vec<ReturnValue>, ShellError>),
    ) -> &mut PluginTest<'a, T> {
        let call_stub = self.call_info.clone();

        self.configure(|flags_configured| {
            let flags_registered = &call_stub.args.named;

            let flag_passed = match flags_registered {
                Some(names) => Some(names.keys().map(String::from).collect::<Vec<String>>()),
                None => None,
            };

            if let Some(flags) = flag_passed {
                for flag in flags {
                    assert!(
                        flags_configured.iter().any(|f| *f == flag),
                        format!(
                            "The flag you passed ({}) is not configured in the plugin.",
                            flag
                        )
                    );
                }
            }
        });

        let began = self.plugin.begin_filter(call_stub);

        let return_values = match began {
            Ok(values) => Ok(values),
            Err(reason) => Err(reason),
        };

        callback(self.plugin, return_values);
        self
    }
}

pub fn plugin<T: Plugin>(plugin: &mut T) -> PluginTest<T> {
    PluginTest::for_plugin(plugin)
}

pub fn table(list: &Vec<Value>) -> Value {
    UntaggedValue::table(list).into_untagged_value()
}

pub fn column_path(paths: &Vec<Value>) -> Value {
    UntaggedValue::Primitive(Primitive::ColumnPath(
        table(&paths.iter().cloned().collect())
            .as_column_path()
            .unwrap()
            .item,
    ))
    .into_untagged_value()
}
pub struct CallStub {
    positionals: Vec<Value>,
    flags: IndexMap<String, Value>,
}

impl CallStub {
    pub fn new() -> CallStub {
        CallStub {
            positionals: vec![],
            flags: indexmap::IndexMap::new(),
        }
    }

    pub fn with_named_parameter(&mut self, name: &str, value: Value) -> &mut Self {
        self.flags.insert(name.to_string(), value);
        self
    }

    pub fn with_long_flag(&mut self, name: &str) -> &mut Self {
        self.flags.insert(
            name.to_string(),
            UntaggedValue::boolean(true).into_value(Tag::unknown()),
        );
        self
    }

    pub fn with_parameter(&mut self, name: &str) -> &mut Self {
        let fields: Vec<Value> = name
            .split(".")
            .map(|s| UntaggedValue::string(s.to_string()).into_value(Tag::unknown()))
            .collect();

        self.positionals.push(column_path(&fields));
        self
    }

    pub fn create(&self) -> CallInfo {
        CallInfo {
            args: EvaluatedArgs::new(Some(self.positionals.clone()), Some(self.flags.clone())),
            name_tag: Tag::unknown(),
        }
    }
}
