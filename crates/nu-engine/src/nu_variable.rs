use core::fmt;
use std::collections::HashMap;

use nu_protocol::{engine::EngineState, CustomValue, LazyRecord, ShellError, Span, Value};
use serde::{Deserialize, Serialize};

// a CustomValue for the special $nu variable
// $nu used to be a plain old Record, but CustomValue lets us load different fields/columns lazily. This is important for performance;
// collecting all the information in $nu is expensive and unnecessary if  you just want a subset of the data
// should this use #[typetag::serde] instead?
#[derive(Serialize, Deserialize)]
pub struct NuVariable {
    #[serde(skip)]
    pub engine_state: EngineState, // not serializable... so how does that work?
}

// manually implement so we can skip engine_state which doesn't implement Debug
impl fmt::Debug for NuVariable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NuVariable").finish()
    }
}

impl LazyRecord for NuVariable {
    fn value_string(&self) -> String {
        "$nu".to_string()
    }

    fn get_column_map(
        &self,
        span: Span,
    ) -> HashMap<String, Box<dyn Fn() -> Result<Value, ShellError>>> {
        let mut hm: HashMap<_, Box<dyn Fn() -> Result<Value, ShellError>>> = HashMap::new();

        hm.insert(
            "config-path".to_string(),
            Box::new(move || {
                let mut config_path = nu_path::config_dir().expect("could not get config path");
                config_path.push("nushell");
                config_path.push("config.nu");

                Ok(Value::String {
                    val: config_path.to_string_lossy().to_string(),
                    span,
                })
            }),
        );

        hm.insert(
            "asdf".to_string(),
            Box::new(move || Ok(Value::string("val", span))),
        );
        hm
    }

    fn typetag_name(&self) -> &'static str {
        todo!()
    }

    fn typetag_deserialize(&self) {
        todo!()
    }

    // fn get_column_map(&self) -> HashMap<String, Box<dyn Fn() -> Value>> {
    // }
}
