use crate::Plugin;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{
    CallInfo, EvaluatedArgs, Primitive, ReturnSuccess, ReturnValue, UntaggedValue, Value,
};
use nu_source::Tag;
use nu_test_support::value::column_path;
use nu_value_ext::ValueExt;

pub struct PluginTest<'a, T: Plugin> {
    plugin: &'a mut T,
    call_info: CallInfo,
    input: Value,
}

impl<'a, T: Plugin> PluginTest<'a, T> {
    pub fn for_plugin(plugin: &'a mut T) -> Self {
        PluginTest {
            plugin,
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
        let mut return_values = return_values?;
        let end = self.plugin.end_filter();

        return_values.extend(end?);

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

            let flag_passed = flags_registered
                .as_ref()
                .map(|names| names.keys().map(String::from).collect::<Vec<String>>());

            if let Some(flags) = flag_passed {
                for flag in flags {
                    assert!(
                        flags_configured.iter().any(|f| *f == flag),
                        "The flag you passed is not configured in the plugin.",
                    );
                }
            }
        });

        let return_values = self.plugin.begin_filter(call_stub);

        callback(self.plugin, return_values);
        self
    }
}

pub fn plugin<T: Plugin>(plugin: &mut T) -> PluginTest<T> {
    PluginTest::for_plugin(plugin)
}

#[derive(Default)]
pub struct CallStub {
    positionals: Vec<Value>,
    flags: IndexMap<String, Value>,
}

impl CallStub {
    pub fn new() -> Self {
        Default::default()
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

    pub fn with_parameter(&mut self, name: &str) -> Result<&mut Self, ShellError> {
        let cp = column_path(name)
            .as_column_path()
            .expect("Failed! Expected valid column path.");
        let cp = UntaggedValue::Primitive(Primitive::ColumnPath(cp.item)).into_value(cp.tag);

        self.positionals.push(cp);
        Ok(self)
    }

    pub fn create(&self) -> CallInfo {
        CallInfo {
            args: EvaluatedArgs::new(Some(self.positionals.clone()), Some(self.flags.clone())),
            name_tag: Tag::unknown(),
        }
    }
}

pub fn expect_return_value_at(
    for_results: Result<Vec<Result<ReturnSuccess, ShellError>>, ShellError>,
    at: usize,
) -> Value {
    let return_values = for_results
        .expect("Failed! This seems to be an error getting back the results from the plugin.");

    for (idx, item) in return_values.iter().enumerate() {
        let item = match item {
            Ok(return_value) => return_value,
            Err(_) => panic!("Unexpected value"),
        };

        if idx == at {
            if let Some(value) = item.raw_value() {
                return value;
            } else {
                panic!("Internal error: could not get raw value in expect_return_value_at")
            }
        }
    }

    panic!("Couldn't get return value from stream.")
}
