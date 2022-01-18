use crate::{BlockId, ShellError, Span, Spanned, Value};
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
    pub modifier: Spanned<String>,
    pub keycode: Spanned<String>,
    pub event: Spanned<EventType>,
    pub mode: Spanned<EventMode>,
}

impl Default for ParsedKeybinding {
    fn default() -> Self {
        Self {
            modifier: Spanned {
                item: "".to_string(),
                span: Span { start: 0, end: 0 },
            },
            keycode: Spanned {
                item: "".to_string(),
                span: Span { start: 0, end: 0 },
            },
            event: Spanned {
                item: EventType::Single("".to_string()),
                span: Span { start: 0, end: 0 },
            },
            mode: Spanned {
                item: EventMode::Emacs,
                span: Span { start: 0, end: 0 },
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum EventType {
    Single(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum EventMode {
    Emacs,
    ViNormal,
    ViInsert,
}

impl EventMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventMode::Emacs => "emacs",
            EventMode::ViNormal => "vi_normal",
            EventMode::ViInsert => "vi_insert",
        }
    }
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
        let v = self.as_record()?;

        let mut config = Config::default();

        for (key, value) in v.0.iter().zip(v.1) {
            match key.as_str() {
                "filesize_metric" => {
                    config.filesize_metric = value.as_bool()?;
                }
                "table_mode" => {
                    config.table_mode = value.as_string()?;
                }
                "use_ls_colors" => {
                    config.use_ls_colors = value.as_bool()?;
                }
                "color_config" => {
                    config.color_config = create_map(value, &config)?;
                }
                "use_grid_icons" => {
                    config.use_grid_icons = value.as_bool()?;
                }
                "footer_mode" => {
                    let val_str = value.as_string()?.to_lowercase();
                    config.footer_mode = match val_str.as_ref() {
                        "auto" => FooterMode::Auto,
                        "never" => FooterMode::Never,
                        "always" => FooterMode::Always,
                        _ => match &val_str.parse::<u64>() {
                            Ok(number) => FooterMode::RowCount(*number),
                            _ => FooterMode::Never,
                        },
                    };
                }
                "animate_prompt" => {
                    config.animate_prompt = value.as_bool()?;
                }
                "float_precision" => {
                    config.float_precision = value.as_integer()?;
                }
                "use_ansi_coloring" => {
                    config.use_ansi_coloring = value.as_bool()?;
                }
                "filesize_format" => {
                    config.filesize_format = value.as_string()?.to_lowercase();
                }
                "env_conversions" => {
                    let (env_vars, conversions) = value.as_record()?;
                    let mut env_conversions = HashMap::new();

                    for (env_var, record) in env_vars.iter().zip(conversions) {
                        // println!("{}: {:?}", env_var, record);
                        env_conversions.insert(env_var.into(), EnvConversion::from_record(record)?);
                    }

                    config.env_conversions = env_conversions;
                }
                "edit_mode" => {
                    config.edit_mode = value.as_string()?;
                }
                "max_history_size" => {
                    config.max_history_size = value.as_i64()?;
                }
                "log_level" => {
                    config.log_level = value.as_string()?;
                }
                "menu_config" => {
                    config.menu_config = create_map(value, &config)?;
                }
                "keybindings" => config.keybindings = create_keybindings(value, &config)?,
                _ => {}
            }
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
        .map(|(x, y)| {
            let clony = y.clone();
            format!("{}: \"{}\" ", x, clony.into_string(", ", config))
        })
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
        Value::Record { cols, vals, .. } => {
            let mut keybinding = ParsedKeybinding::default();

            for (col, val) in cols.iter().zip(vals.iter()) {
                match col.as_str() {
                    "modifier" => {
                        keybinding.modifier = Spanned {
                            item: val.clone().into_string("", config),
                            span: val.span()?,
                        }
                    }
                    "keycode" => {
                        keybinding.keycode = Spanned {
                            item: val.clone().into_string("", config),
                            span: val.span()?,
                        }
                    }
                    "mode" => {
                        keybinding.mode = match val.clone().into_string("", config).as_str() {
                            "emacs" => Spanned {
                                item: EventMode::Emacs,
                                span: val.span()?,
                            },
                            "vi_normal" => Spanned {
                                item: EventMode::ViNormal,
                                span: val.span()?,
                            },
                            "vi_insert" => Spanned {
                                item: EventMode::ViInsert,
                                span: val.span()?,
                            },
                            e => {
                                return Err(ShellError::UnsupportedConfigValue(
                                    "emacs or vi".to_string(),
                                    e.to_string(),
                                    val.span()?,
                                ))
                            }
                        };
                    }
                    "event" => match val {
                        Value::Record {
                            cols: event_cols,
                            vals: event_vals,
                            span: event_span,
                        } => {
                            let event_type_idx = event_cols
                                .iter()
                                .position(|key| key == "type")
                                .ok_or_else(|| {
                                    ShellError::MissingConfigValue("type".to_string(), *event_span)
                                })?;

                            let event_idx = event_cols
                                .iter()
                                .position(|key| key == "event")
                                .ok_or_else(|| {
                                    ShellError::MissingConfigValue("event".to_string(), *event_span)
                                })?;

                            let event_type =
                                event_vals[event_type_idx].clone().into_string("", config);

                            // Extracting the event type information from the record based on the type
                            match event_type.as_str() {
                                "single" => {
                                    let event_value =
                                        event_vals[event_idx].clone().into_string("", config);

                                    keybinding.event = Spanned {
                                        item: EventType::Single(event_value),
                                        span: *event_span,
                                    }
                                }
                                e => {
                                    return Err(ShellError::UnsupportedConfigValue(
                                        "single".to_string(),
                                        e.to_string(),
                                        *event_span,
                                    ))
                                }
                            };
                        }
                        e => {
                            return Err(ShellError::UnsupportedConfigValue(
                                "record type".to_string(),
                                format!("{:?}", e.get_type()),
                                e.span()?,
                            ))
                        }
                    },
                    "name" => {} // don't need to store name
                    e => {
                        return Err(ShellError::UnsupportedConfigValue(
                            "name, mode, modifier, keycode or event".to_string(),
                            e.to_string(),
                            val.span()?,
                        ))
                    }
                }
            }

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
