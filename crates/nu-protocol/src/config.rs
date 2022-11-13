use crate::{ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const TRIM_STRATEGY_DEFAULT: TrimStrategy = TrimStrategy::Wrap {
    try_to_keep_words: true,
};

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
    pub display_output: Option<Value>,
}

impl Hooks {
    pub fn new() -> Self {
        Self {
            pre_prompt: None,
            pre_execution: None,
            env_change: None,
            display_output: None,
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
    pub external_completer: Option<usize>,
    pub filesize_metric: bool,
    pub table_mode: String,
    pub use_ls_colors: bool,
    pub color_config: HashMap<String, Value>,
    pub use_grid_icons: bool,
    pub footer_mode: FooterMode,
    pub float_precision: i64,
    pub max_external_completion_results: i64,
    pub filesize_format: String,
    pub use_ansi_coloring: bool,
    pub quick_completions: bool,
    pub partial_completions: bool,
    pub completion_algorithm: String,
    pub edit_mode: String,
    pub max_history_size: i64,
    pub sync_history_on_enter: bool,
    pub history_file_format: HistoryFileFormat,
    pub log_level: String,
    pub keybindings: Vec<ParsedKeybinding>,
    pub menus: Vec<ParsedMenu>,
    pub hooks: Hooks,
    pub rm_always_trash: bool,
    pub shell_integration: bool,
    pub buffer_editor: String,
    pub table_index_mode: TableIndexMode,
    pub cd_with_abbreviations: bool,
    pub case_sensitive_completions: bool,
    pub enable_external_completion: bool,
    pub trim_strategy: TrimStrategy,
    pub show_banner: bool,
    pub show_clickable_links_in_ls: bool,
    pub render_right_prompt_on_last_line: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            filesize_metric: false,
            table_mode: "rounded".into(),
            external_completer: None,
            use_ls_colors: true,
            color_config: HashMap::new(),
            use_grid_icons: false,
            footer_mode: FooterMode::RowCount(25),
            float_precision: 4,
            max_external_completion_results: 100,
            filesize_format: "auto".into(),
            use_ansi_coloring: true,
            quick_completions: true,
            partial_completions: true,
            completion_algorithm: "prefix".into(),
            edit_mode: "emacs".into(),
            max_history_size: i64::MAX,
            sync_history_on_enter: true,
            history_file_format: HistoryFileFormat::PlainText,
            log_level: String::new(),
            keybindings: Vec::new(),
            menus: Vec::new(),
            hooks: Hooks::new(),
            rm_always_trash: false,
            shell_integration: false,
            buffer_editor: String::new(),
            table_index_mode: TableIndexMode::Always,
            cd_with_abbreviations: false,
            case_sensitive_completions: false,
            enable_external_completion: true,
            trim_strategy: TRIM_STRATEGY_DEFAULT,
            show_banner: true,
            show_clickable_links_in_ls: true,
            render_right_prompt_on_last_line: false,
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

#[derive(Serialize, Deserialize, Clone, Debug, Copy)]
pub enum HistoryFileFormat {
    /// Store history as an SQLite database with additional context
    Sqlite,
    /// store history as a plain text file where every line is one command (without any context such as timestamps)
    PlainText,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TableIndexMode {
    /// Always show indexes
    Always,
    /// Never show indexes
    Never,
    /// Show indexes when a table has "index" column
    Auto,
}

/// A Table view configuration, for a situation where
/// we need to limit cell width in order to adjust for a terminal size.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TrimStrategy {
    /// Wrapping strategy.
    ///
    /// It it's simmilar to original nu_table, strategy.
    Wrap {
        /// A flag which indicates whether is it necessary to try
        /// to keep word bounderies.
        try_to_keep_words: bool,
    },
    /// Truncating strategy, where we just cut the string.
    /// And append the suffix if applicable.
    Truncate {
        /// Suffix which can be appended to a truncated string after being cut.
        ///
        /// It will be applied only when there's enough room for it.
        /// For example in case where a cell width must be 12 chars, but
        /// the suffix takes 13 chars it won't be used.
        suffix: Option<String>,
    },
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
                    "external_completer" => {
                        if let Ok(v) = value.as_block() {
                            config.external_completer = Some(v)
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
                            eprintln!("$env.config.color_config is not a record")
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
                    "max_external_completion_results" => {
                        if let Ok(i) = value.as_integer() {
                            config.max_external_completion_results = i;
                        } else {
                            eprintln!("$config.max_external_completion_results is not an integer")
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
                    "history_file_format" => {
                        if let Ok(b) = value.as_string() {
                            let val_str = b.to_lowercase();
                            config.history_file_format = match val_str.as_ref() {
                                "sqlite" => HistoryFileFormat::Sqlite,
                                "plaintext" => HistoryFileFormat::PlainText,
                                _ => {
                                    eprintln!(
                                        "unrecognized $config.history_file_format '{val_str}'"
                                    );
                                    HistoryFileFormat::PlainText
                                }
                            };
                        } else {
                            eprintln!("$config.history_file_format is not a string")
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
                    "menus" => match create_menus(value) {
                        Ok(map) => config.menus = map,
                        Err(e) => {
                            eprintln!("$config.menus is not a valid list of menus");
                            eprintln!("{:?}", e);
                        }
                    },
                    "keybindings" => match create_keybindings(value) {
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
                    "table_index_mode" => {
                        if let Ok(b) = value.as_string() {
                            let val_str = b.to_lowercase();
                            match val_str.as_ref() {
                                "always" => config.table_index_mode = TableIndexMode::Always,
                                "never" => config.table_index_mode = TableIndexMode::Never,
                                "auto" => config.table_index_mode = TableIndexMode::Auto,
                                _ => eprintln!(
                                    "$config.table_index_mode must be a never, always or auto"
                                ),
                            }
                        } else {
                            eprintln!("$config.table_index_mode is not a string")
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
                    "enable_external_completion" => {
                        if let Ok(b) = value.as_bool() {
                            config.enable_external_completion = b;
                        } else {
                            eprintln!("$config.enable_external_completion is not a bool")
                        }
                    }
                    "table_trim" => config.trim_strategy = try_parse_trim_strategy(value, &config)?,
                    "show_banner" => {
                        if let Ok(b) = value.as_bool() {
                            config.show_banner = b;
                        } else {
                            eprintln!("$config.show_banner is not a bool")
                        }
                    }
                    "show_clickable_links_in_ls" => {
                        if let Ok(b) = value.as_bool() {
                            config.show_clickable_links_in_ls = b;
                        } else {
                            eprintln!("$config.show_clickable_links_in_ls is not a bool")
                        }
                    }
                    "render_right_prompt_on_last_line" => {
                        if let Ok(b) = value.as_bool() {
                            config.render_right_prompt_on_last_line = b;
                        } else {
                            eprintln!("$config.render_right_prompt_on_last_line is not a bool")
                        }
                    }
                    x => {
                        eprintln!("$config.{} is an unknown config setting", x)
                    }
                }
            }
        } else {
            eprintln!("$env.config is not a record");
        }

        Ok(config)
    }
}

fn try_parse_trim_strategy(value: &Value, config: &Config) -> Result<TrimStrategy, ShellError> {
    let map = create_map(value, config).map_err(|e| {
        eprintln!("$env.config.table_trim is not a record");
        e
    })?;

    let mut methodology = match map.get("methodology") {
        Some(value) => match try_parse_trim_methodology(value) {
            Some(methodology) => methodology,
            None => return Ok(TRIM_STRATEGY_DEFAULT),
        },
        None => {
            eprintln!("$config.table_trim.methodology was not provided");
            return Ok(TRIM_STRATEGY_DEFAULT);
        }
    };

    match &mut methodology {
        TrimStrategy::Wrap { try_to_keep_words } => {
            if let Some(value) = map.get("wrapping_try_keep_words") {
                if let Ok(b) = value.as_bool() {
                    *try_to_keep_words = b;
                } else {
                    eprintln!("$config.table_trim.wrap_try_keep_words is not a bool");
                }
            }
        }
        TrimStrategy::Truncate { suffix } => {
            if let Some(value) = map.get("truncating_suffix") {
                if let Ok(v) = value.as_string() {
                    *suffix = Some(v);
                } else {
                    eprintln!("$config.table_trim.truncating_suffix is not a string")
                }
            }
        }
    }

    Ok(methodology)
}

fn try_parse_trim_methodology(value: &Value) -> Option<TrimStrategy> {
    match value.as_string() {
        Ok(value) => match value.to_lowercase().as_str() {
            "wrapping" => {
                return Some(TrimStrategy::Wrap {
                    try_to_keep_words: false,
                });
            }
            "truncating" => return Some(TrimStrategy::Truncate { suffix: None }),
            _ => eprintln!("unrecognized $config.trim_methodology value; expected values ['truncating', 'wrapping']"),
        },
        Err(_) => eprintln!("$config.trim_methodology is not a string"),
    }

    None
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
                    "display_output" => hooks.display_output = Some(vals[idx].clone()),
                    x => {
                        return Err(ShellError::UnsupportedConfigValue(
                            "'pre_prompt', 'pre_execution', 'env_change'".to_string(),
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
fn create_keybindings(value: &Value) -> Result<Vec<ParsedKeybinding>, ShellError> {
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
                .map(create_keybindings)
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
pub fn create_menus(value: &Value) -> Result<Vec<ParsedMenu>, ShellError> {
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
                .map(create_menus)
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
