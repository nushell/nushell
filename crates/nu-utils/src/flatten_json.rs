use serde_json::{json, Map, Value as SerdeValue};

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

impl Default for JsonFlattener<'_> {
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
impl JsonFlattener<'_> {
    /// Returns a flattener with the default arguments
    /// # Examples
    /// ```
    /// use nu_utils;
    ///
    /// let flattener = nu_utils::JsonFlattener::new();
    /// ```
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

            let expanded_identifier = self.filter_known_keys(&expanded_identifier);

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
        obj: &[SerdeValue],
    ) {
        for (k, v) in obj.iter().enumerate() {
            let with_key = format!("{identifier}{}{k}", self.separator);
            if with_key.contains("span.start") || with_key.contains("span.end") {
                continue;
            }

            let with_key = self.filter_known_keys(&with_key);

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

    fn filter_known_keys(&self, key: &str) -> String {
        let mut filtered_key = key.to_string();
        if filtered_key.contains(".String.val") {
            filtered_key = filtered_key.replace(".String.val", "");
        }
        if filtered_key.contains(".Record.val") {
            filtered_key = filtered_key.replace(".Record.val", "");
        }
        if filtered_key.contains(".List.vals") {
            filtered_key = filtered_key.replace(".List.vals", "");
        }
        if filtered_key.contains(".Int.val") {
            filtered_key = filtered_key.replace(".Int.val", "");
        }
        if filtered_key.contains(".Bool.val") {
            filtered_key = filtered_key.replace(".Bool.val", "");
        }
        if filtered_key.contains(".Truncate.suffix") {
            filtered_key = filtered_key.replace(".Truncate.suffix", ".truncating_suffix");
        }
        if filtered_key.contains(".RowCount") {
            filtered_key = filtered_key.replace(".RowCount", "");
        }
        if filtered_key.contains(".Wrap.try_to_keep_words") {
            filtered_key =
                filtered_key.replace(".Wrap.try_to_keep_words", ".wrapping_try_keep_words");
        }
        // For now, let's skip replacing these because they tell us which
        // numbers are closures and blocks which is useful for extracting the content
        // if filtered_key.contains(".Closure.val") {
        //     filtered_key = filtered_key.replace(".Closure.val", "");
        // }
        // if filtered_key.contains(".block_id") {
        //     filtered_key = filtered_key.replace(".block_id", "");
        // }
        filtered_key
    }
}
