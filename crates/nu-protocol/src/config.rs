use crate::{BlockId, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const ANIMATE_PROMPT_DEFAULT: bool = true;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EnvConversion {
    pub from_string: Option<(BlockId, Span)>,
    pub to_string: Option<(BlockId, Span)>,
}

impl EnvConversion {
    pub fn from_record(value: &Value) -> Result<Self, ShellError> {
        let record = value.as_record()?;

        let mut conv_map = HashMap::new();

        for (k, v) in record.0.iter().zip(record.1) {
            if (k == "from_string") || (k == "to_string") {
                conv_map.insert(k.as_str(), (v.as_block()?, v.span()?));
            } else {
                return Err(ShellError::UnsupportedConfigValue(
                    "'from_string' and 'to_string' fields".into(),
                    k.into(),
                    value.span()?,
                ));
            }
        }

        let from_string = conv_map.get("from_string").cloned();
        let to_string = conv_map.get("to_string").cloned();

        Ok(EnvConversion {
            from_string,
            to_string,
        })
    }
}

/// Definition of a parsed keybinding from the config object
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParsedKeybinding {
    pub modifier: Value,
    pub keycode: Value,
    pub event: Value,
    pub mode: Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub filesize_metric: bool,
    pub table_mode: String,
    pub use_ls_colors: bool,
    pub color_config: HashMap<String, Value>,
    pub use_grid_icons: bool,
    pub footer_mode: FooterMode,
    pub animate_prompt: bool,
    pub float_precision: i64,
    pub filesize_format: String,
    pub use_ansi_coloring: bool,
    pub env_conversions: HashMap<String, EnvConversion>,
    pub edit_mode: String,
    pub max_history_size: i64,
    pub log_level: String,
    pub menu_config: HashMap<String, Value>,
    pub keybindings: Vec<ParsedKeybinding>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            filesize_metric: false,
            table_mode: "rounded".into(),
            use_ls_colors: true,
            color_config: HashMap::new(),
            use_grid_icons: false,
            footer_mode: FooterMode::Never,
            animate_prompt: ANIMATE_PROMPT_DEFAULT,
            float_precision: 4,
            filesize_format: "auto".into(),
            use_ansi_coloring: true,
            env_conversions: HashMap::new(), // TODO: Add default conversoins
            edit_mode: "emacs".into(),
            max_history_size: 1000,
            log_level: String::new(),
            menu_config: HashMap::new(),
            keybindings: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum FooterMode {
    /// Never show the footer
    Never,
    /// Always show the footer
    Always,
    /// Only show the footer if there are more than RowCount rows
    RowCount(u64),
    /// Calculate the screen height, calculate row count, if display will be bigger than screen, add the footer
    Auto,
}

impl Value {
    pub fn into_config(self) -> Result<Config, ShellError> {
        let v = self.as_record();

        let mut config = Config::default();

        if let Ok(v) = v {
            for (key, value) in v.0.iter().zip(v.1) {
                match key.as_str() {
                    "filesize_metric" => {
                        if let Ok(b) = value.as_bool() {
                            config.filesize_metric = b;
                        } else {
                            eprintln!("$config.filesize_metric is not a bool")
                        }
                    }
                    "table_mode" => {
                        if let Ok(v) = value.as_string() {
                            config.table_mode = v;
                        } else {
                            eprintln!("$config.table_mode is not a string")
                        }
                    }
                    "use_ls_colors" => {
                        if let Ok(b) = value.as_bool() {
                            config.use_ls_colors = b;
                        } else {
                            eprintln!("$config.use_ls_colors is not a bool")
                        }
                    }
                    "color_config" => {
                        if let Ok(map) = create_map(value, &config) {
                            config.color_config = map;
                        } else {
                            eprintln!("$config.color_config is not a record")
                        }
                    }
                    "use_grid_icons" => {
                        if let Ok(b) = value.as_bool() {
                            config.use_grid_icons = b;
                        } else {
                            eprintln!("$config.use_grid_icons is not a bool")
                        }
                    }
                    "footer_mode" => {
                        if let Ok(b) = value.as_string() {
                            let val_str = b.to_lowercase();
                            config.footer_mode = match val_str.as_ref() {
                                "auto" => FooterMode::Auto,
                                "never" => FooterMode::Never,
                                "always" => FooterMode::Always,
                                _ => match &val_str.parse::<u64>() {
                                    Ok(number) => FooterMode::RowCount(*number),
                                    _ => FooterMode::Never,
                                },
                            };
                        } else {
                            eprintln!("$config.footer_mode is not a string")
                        }
                    }
                    "animate_prompt" => {
                        if let Ok(b) = value.as_bool() {
                            config.animate_prompt = b;
                        } else {
                            eprintln!("$config.animate_prompt is not a bool")
                        }
                    }
                    "float_precision" => {
                        if let Ok(i) = value.as_integer() {
                            config.float_precision = i;
                        } else {
                            eprintln!("$config.float_precision is not an integer")
                        }
                    }
                    "use_ansi_coloring" => {
                        if let Ok(b) = value.as_bool() {
                            config.use_ansi_coloring = b;
                        } else {
                            eprintln!("$config.use_ansi_coloring is not a bool")
                        }
                    }
                    "filesize_format" => {
                        if let Ok(v) = value.as_string() {
                            config.filesize_format = v.to_lowercase();
                        } else {
                            eprintln!("$config.filesize_format is not a string")
                        }
                    }
                    "env_conversions" => {
                        if let Ok((env_vars, conversions)) = value.as_record() {
                            let mut env_conversions = HashMap::new();

                            for (env_var, record) in env_vars.iter().zip(conversions) {
                                // println!("{}: {:?}", env_var, record);
                                if let Ok(conversion) = EnvConversion::from_record(record) {
                                    env_conversions.insert(env_var.into(), conversion);
                                } else {
                                    eprintln!("$config.env_conversions has incorrect conversion")
                                }
                            }

                            config.env_conversions = env_conversions;
                        } else {
                            eprintln!("$config.env_conversions is not a record")
                        }
                    }
                    "edit_mode" => {
                        if let Ok(v) = value.as_string() {
                            config.edit_mode = v.to_lowercase();
                        } else {
                            eprintln!("$config.edit_mode is not a string")
                        }
                    }
                    "max_history_size" => {
                        if let Ok(i) = value.as_i64() {
                            config.max_history_size = i;
                        } else {
                            eprintln!("$config.max_history_size is not an integer")
                        }
                    }
                    "log_level" => {
                        if let Ok(v) = value.as_string() {
                            config.log_level = v.to_lowercase();
                        } else {
                            eprintln!("$config.log_level is not a string")
                        }
                    }
                    "menu_config" => {
                        if let Ok(map) = create_map(value, &config) {
                            config.menu_config = map;
                        } else {
                            eprintln!("$config.menu_config is not a record")
                        }
                    }
                    "keybindings" => {
                        if let Ok(keybindings) = create_keybindings(value, &config) {
                            config.keybindings = keybindings;
                        } else {
                            eprintln!("$config.keybindings is not a valid keybindings list")
                        }
                    }
                    x => {
                        eprintln!("$config.{} is an unknown config setting", x)
                    }
                }
            }
        } else {
            eprintln!("$config is not a record");
        }

        Ok(config)
    }
}

fn create_map(value: &Value, config: &Config) -> Result<HashMap<String, Value>, ShellError> {
    let (cols, inner_vals) = value.as_record()?;
    let mut hm: HashMap<String, Value> = HashMap::new();

    for (k, v) in cols.iter().zip(inner_vals) {
        match &v {
            Value::Record {
                cols: inner_cols,
                vals: inner_vals,
                span,
            } => {
                let val = color_value_string(span, inner_cols, inner_vals, config);
                hm.insert(k.to_string(), val);
            }
            _ => {
                hm.insert(k.to_string(), v.clone());
            }
        }
    }

    Ok(hm)
}

fn color_value_string(
    span: &Span,
    inner_cols: &[String],
    inner_vals: &[Value],
    config: &Config,
) -> Value {
    // make a string from our config.color_config section that
    // looks like this: { fg: "#rrggbb" bg: "#rrggbb" attr: "abc", }
    // the real key here was to have quotes around the values but not
    // require them around the keys.

    // maybe there's a better way to generate this but i'm not sure
    // what it is.
    let val: String = inner_cols
        .iter()
        .zip(inner_vals)
        .map(|(x, y)| format!("{}: \"{}\" ", x, y.into_string(", ", config)))
        .collect();

    // now insert the braces at the front and the back to fake the json string
    Value::String {
        val: format!("{{{}}}", val),
        span: *span,
    }
}

// Parses the config object to extract the strings that will compose a keybinding for reedline
fn create_keybindings(value: &Value, config: &Config) -> Result<Vec<ParsedKeybinding>, ShellError> {
    match value {
        Value::Record { cols, vals, span } => {
            // Finding the modifier value in the record
            let modifier = extract_value("modifier", cols, vals, span)?;
            let keycode = extract_value("keycode", cols, vals, span)?;
            let mode = extract_value("mode", cols, vals, span)?;
            let event = extract_value("event", cols, vals, span)?;

            let keybinding = ParsedKeybinding {
                modifier: modifier.clone(),
                keycode: keycode.clone(),
                mode: mode.clone(),
                event: event.clone(),
            };

            Ok(vec![keybinding])
        }
        Value::List { vals, .. } => {
            let res = vals
                .iter()
                .map(|inner_value| create_keybindings(inner_value, config))
                .collect::<Result<Vec<Vec<ParsedKeybinding>>, ShellError>>();

            let res = res?
                .into_iter()
                .flatten()
                .collect::<Vec<ParsedKeybinding>>();

            Ok(res)
        }
        _ => Ok(Vec::new()),
    }
}

pub fn extract_value<'record>(
    name: &str,
    cols: &'record [String],
    vals: &'record [Value],
    span: &Span,
) -> Result<&'record Value, ShellError> {
    cols.iter()
        .position(|col| col.as_str() == name)
        .and_then(|index| vals.get(index))
        .ok_or_else(|| ShellError::MissingConfigValue(name.to_string(), *span))
}
