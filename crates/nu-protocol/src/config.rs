use crate::{ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const ANIMATE_PROMPT_DEFAULT: bool = true;

/// Definition of a parsed keybinding from the config object
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParsedKeybinding {
    pub modifier: Value,
    pub keycode: Value,
    pub event: Value,
    pub mode: Value,
}

/// Definition of a parsed menu from the config object
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParsedMenu {
    pub name: Value,
    pub marker: Value,
    pub only_buffer_difference: Value,
    pub style: Value,
    pub menu_type: Value,
    pub source: Value,
}

/// Definition of a parsed menu from the config object
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Hooks {
    pub pre_prompt: Option<Value>,
    pub pre_execution: Option<Value>,
    pub env_change: Option<Value>,
}

impl Hooks {
    pub fn new() -> Self {
        Self {
            pre_prompt: None,
            pre_execution: None,
            env_change: None,
        }
    }
}

impl Default for Hooks {
    fn default() -> Self {
        Self::new()
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
    pub quick_completions: bool,
    pub partial_completions: bool,
    pub completion_algorithm: String,
    pub edit_mode: String,
    pub max_history_size: i64,
    pub sync_history_on_enter: bool,
    pub log_level: String,
    pub keybindings: Vec<ParsedKeybinding>,
    pub menus: Vec<ParsedMenu>,
    pub hooks: Hooks,
    pub rm_always_trash: bool,
    pub shell_integration: bool,
    pub buffer_editor: String,
    pub disable_table_indexes: bool,
    pub cd_with_abbreviations: bool,
    pub case_sensitive_completions: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            filesize_metric: false,
            table_mode: "rounded".into(),
            use_ls_colors: true,
            color_config: HashMap::new(),
            use_grid_icons: false,
            footer_mode: FooterMode::RowCount(25),
            animate_prompt: ANIMATE_PROMPT_DEFAULT,
            float_precision: 4,
            filesize_format: "auto".into(),
            use_ansi_coloring: true,
            quick_completions: true,
            partial_completions: true,
            completion_algorithm: "prefix".into(),
            edit_mode: "emacs".into(),
            max_history_size: i64::MAX,
            sync_history_on_enter: true,
            log_level: String::new(),
            keybindings: Vec::new(),
            menus: Vec::new(),
            hooks: Hooks::new(),
            rm_always_trash: false,
            shell_integration: false,
            buffer_editor: String::new(),
            disable_table_indexes: false,
            cd_with_abbreviations: false,
            case_sensitive_completions: false,
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
                    "quick_completions" => {
                        if let Ok(b) = value.as_bool() {
                            config.quick_completions = b;
                        } else {
                            eprintln!("$config.quick_completions is not a bool")
                        }
                    }
                    "partial_completions" => {
                        if let Ok(b) = value.as_bool() {
                            config.partial_completions = b;
                        } else {
                            eprintln!("$config.partial_completions is not a bool")
                        }
                    }
                    "completion_algorithm" => {
                        if let Ok(v) = value.as_string() {
                            config.completion_algorithm = v.to_lowercase();
                        } else {
                            eprintln!("$config.completion_algorithm is not a string")
                        }
                    }
                    "rm_always_trash" => {
                        if let Ok(b) = value.as_bool() {
                            config.rm_always_trash = b;
                        } else {
                            eprintln!("$config.rm_always_trash is not a bool")
                        }
                    }
                    "filesize_format" => {
                        if let Ok(v) = value.as_string() {
                            config.filesize_format = v.to_lowercase();
                        } else {
                            eprintln!("$config.filesize_format is not a string")
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
                    "sync_history_on_enter" => {
                        if let Ok(b) = value.as_bool() {
                            config.sync_history_on_enter = b;
                        } else {
                            eprintln!("$config.sync_history_on_enter is not a bool")
                        }
                    }
                    "log_level" => {
                        if let Ok(v) = value.as_string() {
                            config.log_level = v.to_lowercase();
                        } else {
                            eprintln!("$config.log_level is not a string")
                        }
                    }
                    "menus" => match create_menus(value, &config) {
                        Ok(map) => config.menus = map,
                        Err(e) => {
                            eprintln!("$config.menus is not a valid list of menus");
                            eprintln!("{:?}", e);
                        }
                    },
                    "keybindings" => match create_keybindings(value, &config) {
                        Ok(keybindings) => config.keybindings = keybindings,
                        Err(e) => {
                            eprintln!("$config.keybindings is not a valid keybindings list");
                            eprintln!("{:?}", e);
                        }
                    },
                    "hooks" => match create_hooks(value) {
                        Ok(hooks) => config.hooks = hooks,
                        Err(e) => {
                            eprintln!("$config.hooks is not a valid hooks list");
                            eprintln!("{:?}", e);
                        }
                    },
                    "shell_integration" => {
                        if let Ok(b) = value.as_bool() {
                            config.shell_integration = b;
                        } else {
                            eprintln!("$config.shell_integration is not a bool")
                        }
                    }
                    "buffer_editor" => {
                        if let Ok(v) = value.as_string() {
                            config.buffer_editor = v.to_lowercase();
                        } else {
                            eprintln!("$config.buffer_editor is not a string")
                        }
                    }
                    "disable_table_indexes" => {
                        if let Ok(b) = value.as_bool() {
                            config.disable_table_indexes = b;
                        } else {
                            eprintln!("$config.disable_table_indexes is not a bool")
                        }
                    }
                    "cd_with_abbreviations" => {
                        if let Ok(b) = value.as_bool() {
                            config.cd_with_abbreviations = b;
                        } else {
                            eprintln!("$config.cd_with_abbreviations is not a bool")
                        }
                    }
                    "case_sensitive_completions" => {
                        if let Ok(b) = value.as_bool() {
                            config.case_sensitive_completions = b;
                        } else {
                            eprintln!("$config.case_sensitive_completions is not a bool")
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

pub fn color_value_string(
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

// Parse the hooks to find the blocks to run when the hooks fire
fn create_hooks(value: &Value) -> Result<Hooks, ShellError> {
    match value {
        Value::Record { cols, vals, span } => {
            let mut hooks = Hooks::new();

            for idx in 0..cols.len() {
                match cols[idx].as_str() {
                    "pre_prompt" => hooks.pre_prompt = Some(vals[idx].clone()),
                    "pre_execution" => hooks.pre_execution = Some(vals[idx].clone()),
                    "env_change" => hooks.env_change = Some(vals[idx].clone()),
                    x => {
                        return Err(ShellError::UnsupportedConfigValue(
                            "'pre_prompt', 'pre_execution', or 'env_change'".to_string(),
                            x.to_string(),
                            *span,
                        ));
                    }
                }
            }

            Ok(hooks)
        }
        v => match v.span() {
            Ok(span) => Err(ShellError::UnsupportedConfigValue(
                "record for 'hooks' config".into(),
                "non-record value".into(),
                span,
            )),
            _ => Err(ShellError::UnsupportedConfigValue(
                "record for 'hooks' config".into(),
                "non-record value".into(),
                Span { start: 0, end: 0 },
            )),
        },
    }
}

// Parses the config object to extract the strings that will compose a keybinding for reedline
fn create_keybindings(value: &Value, config: &Config) -> Result<Vec<ParsedKeybinding>, ShellError> {
    match value {
        Value::Record { cols, vals, span } => {
            // Finding the modifier value in the record
            let modifier = extract_value("modifier", cols, vals, span)?.clone();
            let keycode = extract_value("keycode", cols, vals, span)?.clone();
            let mode = extract_value("mode", cols, vals, span)?.clone();
            let event = extract_value("event", cols, vals, span)?.clone();

            let keybinding = ParsedKeybinding {
                modifier,
                keycode,
                mode,
                event,
            };

            // We return a menu to be able to do recursion on the same function
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

// Parses the config object to extract the strings that will compose a keybinding for reedline
pub fn create_menus(value: &Value, config: &Config) -> Result<Vec<ParsedMenu>, ShellError> {
    match value {
        Value::Record { cols, vals, span } => {
            // Finding the modifier value in the record
            let name = extract_value("name", cols, vals, span)?.clone();
            let marker = extract_value("marker", cols, vals, span)?.clone();
            let only_buffer_difference =
                extract_value("only_buffer_difference", cols, vals, span)?.clone();
            let style = extract_value("style", cols, vals, span)?.clone();
            let menu_type = extract_value("type", cols, vals, span)?.clone();

            // Source is an optional value
            let source = match extract_value("source", cols, vals, span) {
                Ok(source) => source.clone(),
                Err(_) => Value::Nothing { span: *span },
            };

            let menu = ParsedMenu {
                name,
                only_buffer_difference,
                marker,
                style,
                menu_type,
                source,
            };

            Ok(vec![menu])
        }
        Value::List { vals, .. } => {
            let res = vals
                .iter()
                .map(|inner_value| create_menus(inner_value, config))
                .collect::<Result<Vec<Vec<ParsedMenu>>, ShellError>>();

            let res = res?.into_iter().flatten().collect::<Vec<ParsedMenu>>();

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
