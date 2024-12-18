use nu_engine::command_prelude::*;
use serde_json::{json, Map, Value as SerdeValue};

#[derive(Clone)]
pub struct ConfigNu;

impl Command for ConfigNu {
    fn name(&self) -> &str {
        "config nu"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Env)
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .switch(
                "default",
                "Print the internal default `config.nu` file instead.",
                Some('d'),
            )
            .switch(
                "doc",
                "Print a commented `config.nu` with documentation instead.",
                Some('s'),
            )
            .switch(
                "flatten",
                "Print a flattened representation of `config.nu` file.",
                Some('f'),
            )

        // TODO: Signature narrower than what run actually supports theoretically
    }

    fn description(&self) -> &str {
        "Edit nu configurations."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "open user's config.nu in the default editor",
                example: "config nu",
                result: None,
            },
            Example {
                description: "pretty-print a commented `config.nu` that explains common settings",
                example: "config nu --doc | nu-highlight",
                result: None,
            },
            Example {
                description:
                    "pretty-print the internal `config.nu` file which is loaded before user's config",
                example: "config nu --default | nu-highlight",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let default_flag = call.has_flag(engine_state, stack, "default")?;
        let doc_flag = call.has_flag(engine_state, stack, "doc")?;
        let flatten_flag = call.has_flag(engine_state, stack, "flatten")?;

        if default_flag && doc_flag {
            return Err(ShellError::IncompatibleParameters {
                left_message: "can't use `--default` at the same time".into(),
                left_span: call.get_flag_span(stack, "default").expect("has flag"),
                right_message: "because of `--doc`".into(),
                right_span: call.get_flag_span(stack, "doc").expect("has flag"),
            });
        }

        // `--default` flag handling
        if default_flag {
            let head = call.head;
            return Ok(Value::string(nu_utils::get_default_config(), head).into_pipeline_data());
        }

        if flatten_flag {
            let config = engine_state.get_config();
            // Serialize the Config instance to JSON
            let serialized_config = serde_json::to_value(&**config).unwrap();

            let flattener = JsonFlattener {
                separator: ".",
                alt_array_flattening: false,
                preserve_arrays: true,
            };

            let mut flattened_config_str = flattener.flatten(&serialized_config).to_string();
            if flattened_config_str.contains(".String.val") {
                flattened_config_str = flattened_config_str.replace(".String.val", "");
            }
            if flattened_config_str.contains(".Record.val") {
                flattened_config_str = flattened_config_str.replace(".Record.val", "");
            }
            if flattened_config_str.contains(".Closure.val") {
                flattened_config_str = flattened_config_str.replace(".Closure.val", "");
            }
            if flattened_config_str.contains(".List.vals") {
                flattened_config_str = flattened_config_str.replace(".List.vals", "");
            }
            if flattened_config_str.contains(".Int.val") {
                flattened_config_str = flattened_config_str.replace(".Int.val", "");
            }
            if flattened_config_str.contains(".Bool.val") {
                flattened_config_str = flattened_config_str.replace(".Bool.val", "");
            }
            if flattened_config_str.contains(".block_id") {
                flattened_config_str = flattened_config_str.replace(".block_id", "");
            }
            return Ok(Value::string(flattened_config_str, call.head).into_pipeline_data());
        }

        // `--doc` flag handling
        if doc_flag {
            let head = call.head;
            return Ok(Value::string(nu_utils::get_doc_config(), head).into_pipeline_data());
        }

        super::config_::start_editor("config-path", engine_state, stack, call)
    }
}

// use serde_json::{json, Map, Value as SerdeValue};

/// JsonFlattener is the main driver when flattening JSON
/// # Examples
/// ```
/// use nu_utils;
///
/// let flattener = nu_utils::JsonFlattener { ..Default::default() };
/// ```
pub struct JsonFlattener<'a> {
    /// Alternate separator used between keys when flattening
    /// # Examples
    /// ```
    /// use nu_utils;
    /// let flattener = nu_utils::JsonFlattener { separator: "_", ..Default::default()};
    /// ```
    pub separator: &'a str,
    /// Opinionated flattening format that places values in an array if the object is nested inside an array
    /// # Examples
    /// ```
    /// use nu_utils;
    /// let flattener = nu_utils::JsonFlattener { alt_array_flattening: true, ..Default::default()};
    /// ```
    pub alt_array_flattening: bool,
    /// Completely flatten JSON and keep array structure in the key when flattening
    /// # Examples
    /// ```
    /// use nu_utils;
    /// let flattener = nu_utils::JsonFlattener { preserve_arrays: true, ..Default::default()};
    /// ```
    pub preserve_arrays: bool,
}

impl<'a> Default for JsonFlattener<'a> {
    fn default() -> Self {
        JsonFlattener {
            separator: ".",
            alt_array_flattening: false,
            preserve_arrays: false,
        }
    }
}

/// This implementation defines the core usage for the `JsonFlattener` structure.
/// # Examples
/// ```
/// use nu_utils;
/// use serde_json::json;
///
/// let flattener = nu_utils::JsonFlattener::new();
/// let example = json!({
///     "a": {
///         "b": "c"
///     }
///  });
///
/// let flattened_example = flattener.flatten(&example);
/// ```
impl<'a> JsonFlattener<'a> {
    /// Returns a flattener with the default arguments
    /// # Examples
    /// ```
    /// use nu_utils;
    ///
    /// let flattener = nu_utils::JsonFlattener::new();
    /// ```
    #[allow(dead_code)]
    pub fn new() -> Self {
        JsonFlattener {
            ..Default::default()
        }
    }

    /// Flattens JSON variants into a JSON object
    ///
    /// # Arguments
    ///
    /// * `json` - A serde_json Value to flatten
    ///
    /// # Examples
    /// ```
    /// use nu_utils;
    /// use serde_json::json;
    ///
    /// let flattener = nu_utils::JsonFlattener::new();
    /// let example = json!({
    ///     "name": "John Doe",
    ///     "age": 43,
    ///     "address": {
    ///         "street": "10 Downing Street",
    ///         "city": "London"
    ///     },
    ///     "phones": [
    ///         "+44 1234567",
    ///         "+44 2345678"
    ///     ]
    ///  });
    ///
    /// let flattened_example = flattener.flatten(&example);
    /// ```
    pub fn flatten(&self, json: &SerdeValue) -> SerdeValue {
        let mut flattened_val = Map::<String, SerdeValue>::new();
        match json {
            SerdeValue::Array(obj_arr) => {
                self.flatten_array(&mut flattened_val, &"".to_string(), obj_arr)
            }
            SerdeValue::Object(obj_val) => {
                self.flatten_object(&mut flattened_val, None, obj_val, false)
            }
            _ => self.flatten_value(&mut flattened_val, &"".to_string(), json, false),
        }
        SerdeValue::Object(flattened_val)
    }

    fn flatten_object(
        &self,
        builder: &mut Map<String, SerdeValue>,
        identifier: Option<&String>,
        obj: &Map<String, SerdeValue>,
        arr: bool,
    ) {
        for (k, v) in obj {
            let expanded_identifier = identifier.map_or_else(
                || k.clone(),
                |identifier| format!("{identifier}{}{k}", self.separator),
            );

            if expanded_identifier.contains("span.start")
                || expanded_identifier.contains("span.end")
            {
                continue;
            }

            match v {
                SerdeValue::Object(obj_val) => {
                    self.flatten_object(builder, Some(&expanded_identifier), obj_val, arr)
                }
                SerdeValue::Array(obj_arr) => {
                    self.flatten_array(builder, &expanded_identifier, obj_arr)
                }
                _ => self.flatten_value(builder, &expanded_identifier, v, arr),
            }
        }
    }

    fn flatten_array(
        &self,
        builder: &mut Map<String, SerdeValue>,
        identifier: &String,
        obj: &Vec<SerdeValue>,
    ) {
        for (k, v) in obj.iter().enumerate() {
            let with_key = format!("{identifier}{}{k}", self.separator);
            if with_key.contains("span.start") || with_key.contains("span.end") {
                continue;
            }

            match v {
                SerdeValue::Object(obj_val) => self.flatten_object(
                    builder,
                    Some(if self.preserve_arrays {
                        &with_key
                    } else {
                        identifier
                    }),
                    obj_val,
                    self.alt_array_flattening,
                ),
                SerdeValue::Array(obj_arr) => self.flatten_array(
                    builder,
                    if self.preserve_arrays {
                        &with_key
                    } else {
                        identifier
                    },
                    obj_arr,
                ),
                _ => self.flatten_value(
                    builder,
                    if self.preserve_arrays {
                        &with_key
                    } else {
                        identifier
                    },
                    v,
                    self.alt_array_flattening,
                ),
            }
        }
    }

    fn flatten_value(
        &self,
        builder: &mut Map<String, SerdeValue>,
        identifier: &String,
        obj: &SerdeValue,
        arr: bool,
    ) {
        if let Some(v) = builder.get_mut(identifier) {
            if let Some(arr) = v.as_array_mut() {
                arr.push(obj.clone());
            } else {
                let new_val = json!(vec![v, obj]);
                builder.remove(identifier);
                builder.insert(identifier.to_string(), new_val);
            }
        } else {
            builder.insert(
                identifier.to_string(),
                if arr {
                    json!(vec![obj.clone()])
                } else {
                    obj.clone()
                },
            );
        }
    }
}
