use crate::{record, Record, ShellError, Span, Value};
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

/// Definition of a parsed hook from the config object
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Hooks {
    pub pre_prompt: Option<Value>,
    pub pre_execution: Option<Value>,
    pub env_change: Option<Value>,
    pub display_output: Option<Value>,
    pub command_not_found: Option<Value>,
}

impl Hooks {
    pub fn new() -> Self {
        Self {
            pre_prompt: None,
            pre_execution: None,
            env_change: None,
            display_output: Some(Value::string(
                "if (term size).columns >= 100 { table -e } else { table }",
                Span::unknown(),
            )),
            command_not_found: None,
        }
    }
}

impl Default for Hooks {
    fn default() -> Self {
        Self::new()
    }
}

/// Definition of a Nushell CursorShape (to be mapped to crossterm::cursor::CursorShape)
#[derive(Serialize, Deserialize, Clone, Debug, Copy)]
pub enum NuCursorShape {
    UnderScore,
    Line,
    Block,
    BlinkUnderScore,
    BlinkLine,
    BlinkBlock,
}

fn reconstruct_cursor_shape(name: Option<NuCursorShape>, span: Span) -> Value {
    Value::string(
        match name {
            Some(NuCursorShape::Line) => "line",
            Some(NuCursorShape::Block) => "block",
            Some(NuCursorShape::UnderScore) => "underscore",
            Some(NuCursorShape::BlinkLine) => "blink_line",
            Some(NuCursorShape::BlinkBlock) => "blink_block",
            Some(NuCursorShape::BlinkUnderScore) => "blink_underscore",
            None => "inherit",
        },
        span,
    )
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub external_completer: Option<usize>,
    pub filesize_metric: bool,
    pub table_mode: String,
    pub table_move_header: bool,
    pub table_show_empty: bool,
    pub table_indent: TableIndent,
    pub table_abbreviation_threshold: Option<usize>,
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
    pub history_isolation: bool,
    pub keybindings: Vec<ParsedKeybinding>,
    pub menus: Vec<ParsedMenu>,
    pub hooks: Hooks,
    pub rm_always_trash: bool,
    pub shell_integration: bool,
    pub buffer_editor: Value,
    pub table_index_mode: TableIndexMode,
    pub case_sensitive_completions: bool,
    pub enable_external_completion: bool,
    pub trim_strategy: TrimStrategy,
    pub show_banner: bool,
    pub bracketed_paste: bool,
    pub show_clickable_links_in_ls: bool,
    pub render_right_prompt_on_last_line: bool,
    pub explore: HashMap<String, Value>,
    pub cursor_shape_vi_insert: Option<NuCursorShape>,
    pub cursor_shape_vi_normal: Option<NuCursorShape>,
    pub cursor_shape_emacs: Option<NuCursorShape>,
    pub datetime_normal_format: Option<String>,
    pub datetime_table_format: Option<String>,
    pub error_style: String,
    pub use_kitty_protocol: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            show_banner: true,

            use_ls_colors: true,
            show_clickable_links_in_ls: true,

            rm_always_trash: false,

            table_mode: "rounded".into(),
            table_index_mode: TableIndexMode::Always,
            table_show_empty: true,
            trim_strategy: TRIM_STRATEGY_DEFAULT,
            table_move_header: false,
            table_indent: TableIndent { left: 1, right: 1 },
            table_abbreviation_threshold: None,

            datetime_normal_format: None,
            datetime_table_format: None,

            explore: HashMap::new(),

            max_history_size: 100_000,
            sync_history_on_enter: true,
            history_file_format: HistoryFileFormat::PlainText,
            history_isolation: false,

            case_sensitive_completions: false,
            quick_completions: true,
            partial_completions: true,
            completion_algorithm: "prefix".into(),
            enable_external_completion: true,
            max_external_completion_results: 100,
            external_completer: None,

            filesize_metric: false,
            filesize_format: "auto".into(),

            cursor_shape_emacs: None,
            cursor_shape_vi_insert: None,
            cursor_shape_vi_normal: None,

            color_config: HashMap::new(),
            use_grid_icons: true,
            footer_mode: FooterMode::RowCount(25),
            float_precision: 2,
            buffer_editor: Value::nothing(Span::unknown()),
            use_ansi_coloring: true,
            bracketed_paste: true,
            edit_mode: "emacs".into(),
            shell_integration: false,
            render_right_prompt_on_last_line: false,

            hooks: Hooks::new(),

            menus: Vec::new(),

            keybindings: Vec::new(),

            error_style: "fancy".into(),

            use_kitty_protocol: false,
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

fn reconstruct_history_file_format(config: &Config, span: Span) -> Value {
    Value::string(
        match config.history_file_format {
            HistoryFileFormat::Sqlite => "sqlite",
            HistoryFileFormat::PlainText => "plaintext",
        },
        span,
    )
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

fn reconstruct_index_mode(config: &Config, span: Span) -> Value {
    Value::string(
        match config.table_index_mode {
            TableIndexMode::Always => "always",
            TableIndexMode::Never => "never",
            TableIndexMode::Auto => "auto",
        },
        span,
    )
}

/// A Table view configuration, for a situation where
/// we need to limit cell width in order to adjust for a terminal size.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TrimStrategy {
    /// Wrapping strategy.
    ///
    /// It it's similar to original nu_table, strategy.
    Wrap {
        /// A flag which indicates whether is it necessary to try
        /// to keep word boundaries.
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

impl TrimStrategy {
    pub fn wrap(dont_split_words: bool) -> Self {
        Self::Wrap {
            try_to_keep_words: dont_split_words,
        }
    }

    pub fn truncate(suffix: Option<String>) -> Self {
        Self::Truncate { suffix }
    }
}

fn reconstruct_trim_strategy(config: &Config, span: Span) -> Value {
    match &config.trim_strategy {
        TrimStrategy::Wrap { try_to_keep_words } => Value::record(
            record! {
                "methodology" => Value::string("wrapping", span),
                "wrapping_try_keep_words" => Value::bool(*try_to_keep_words, span),
            },
            span,
        ),
        TrimStrategy::Truncate { suffix } => Value::record(
            match suffix {
                Some(s) => record! {
                    "methodology" => Value::string("truncating", span),
                    "truncating_suffix" => Value::string(s.clone(), span),
                },
                None => record! {
                    "methodology" => Value::string("truncating", span),
                    "truncating_suffix" => Value::nothing(span),
                },
            },
            span,
        ),
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TableIndent {
    pub left: usize,
    pub right: usize,
}

impl Value {
    pub fn into_config(&mut self, config: &Config) -> (Config, Option<ShellError>) {
        // Clone the passed-in config rather than mutating it.
        let mut config = config.clone();

        // Vec for storing errors.
        // Current Nushell behaviour (Dec 2022) is that having some typo like "always_trash": tru in your config.nu's
        // set-env config record shouldn't abort all config parsing there and then.
        // Thus, errors are simply collected one-by-one and wrapped in a GenericError at the end.
        let mut errors = vec![];

        // When an unsupported config value is found, ignore it.
        macro_rules! invalid {
            ($span:expr, $msg:literal) => {
                errors.push(ShellError::GenericError(
                    "Error while applying config changes".into(),
                    format!($msg),
                    Some($span),
                    Some("This value will be ignored.".into()),
                    vec![],
                ));
            };
        }
        // Some extra helpers
        macro_rules! try_bool {
            ($val_ref:ident, $setting:ident) => {
                if let Ok(b) = &$val_ref.as_bool() {
                    config.$setting = *b;
                } else {
                    invalid!($val_ref.span(), "should be a bool");
                    // Reconstruct
                    *$val_ref = Value::bool(config.$setting, $val_ref.span());
                }
            };
        }
        macro_rules! try_int {
            ($val_ref:ident, $setting:ident) => {
                if let Ok(b) = &$val_ref.as_int() {
                    config.$setting = *b;
                } else {
                    invalid!($val_ref.span(), "should be an int");
                    // Reconstruct
                    *$val_ref = Value::int(config.$setting, $val_ref.span());
                }
            };
        }
        // When an unsupported config value is found, remove it from this record.
        macro_rules! invalid_key {
            // Because Value::Record discards all of the spans of its
            // column names (by storing them as Strings), the key name cannot be provided
            // as a value, even in key errors.
            ($span:expr, $msg:literal) => {
                errors.push(ShellError::GenericError(
                    "Error while applying config changes".into(),
                    format!($msg),
                    Some($span),
                    Some("This value will not appear in your $env.config record.".into()),
                    vec![],
                ));
            };
        }

        // Config record (self) mutation rules:
        // * When parsing a config Record, if a config key error occurs, remove the key.
        // * When parsing a config Record, if a config value error occurs, replace the value
        // with a reconstructed Nu value for the current (unaltered) configuration for that setting.
        // For instance:
        // $env.config.ls.use_ls_colors = 2 results in an error, so
        // the current use_ls_colors config setting is converted to a Value::Boolean and inserted in the
        // record in place of the 2.

        if let Value::Record { val, .. } = self {
            val.retain_mut( |key, value| {
                let span = value.span();
                match key {
                    // Grouped options
                    "ls" => {
                        if let Value::Record { val, .. } = value {
                            val.retain_mut(|key2, value| {
                                let span = value.span();
                                match key2 {
                                    "use_ls_colors" => {
                                        try_bool!(value, use_ls_colors);
                                    }
                                    "clickable_links" => {
                                        try_bool!(value, show_clickable_links_in_ls);
                                    }
                                    _ => {
                                        invalid_key!(
                                            span,
                                            "$env.config.{key}.{key2} is an unknown config setting"
                                        );
                                        return false;
                                    }
                                }; true
                            });
                        } else {
                            invalid!(span, "should be a record");
                            // Reconstruct
                            *value = Value::record(
                                record! {
                                    "use_ls_colors" => Value::bool(config.use_ls_colors, span),
                                    "clickable_links" => Value::bool(config.show_clickable_links_in_ls, span),
                                },
                                span,
                            );
                        }
                    }
                    "rm" => {
                        if let Value::Record { val, .. } = value {
                            val.retain_mut(|key2, value| {
                                let span = value.span();
                                match key2 {
                                    "always_trash" => {
                                        try_bool!(value, rm_always_trash);
                                    }
                                    _ => {
                                        invalid_key!(
                                            span,
                                            "$env.config.{key}.{key2} is an unknown config setting"
                                        );
                                        return false;
                                    }
                                };
                                true
                            });
                        } else {
                            invalid!(span, "should be a record");
                            // Reconstruct
                            *value = Value::record(
                                record! {
                                    "always_trash" => Value::bool(config.rm_always_trash, span),
                                },
                                span,
                            );
                        }
                    }
                    "history" => {
                        if let Value::Record { val, .. } = value {
                            val.retain_mut(|key2, value| {
                                let span = value.span();
                                match key2 {
                                    "isolation" => {
                                        try_bool!(value, history_isolation);
                                    }
                                    "sync_on_enter" => {
                                        try_bool!(value, sync_history_on_enter);
                                    }
                                    "max_size" => {
                                        try_int!(value, max_history_size);
                                    }
                                    "file_format" => {
                                        if let Ok(v) = value.as_string() {
                                            let val_str = v.to_lowercase();
                                            match val_str.as_ref() {
                                                "sqlite" => {
                                                    config.history_file_format =
                                                        HistoryFileFormat::Sqlite
                                                }
                                                "plaintext" => {
                                                    config.history_file_format =
                                                        HistoryFileFormat::PlainText
                                                }
                                                _ => {
                                                    invalid!(span,
                                                        "unrecognized $env.config.{key}.{key2} '{val_str}'; expected either 'sqlite' or 'plaintext'"
                                                    );
                                                    // Reconstruct
                                                    *value = reconstruct_history_file_format(
                                                        &config, span,
                                                    );
                                                }
                                            };
                                        } else {
                                            invalid!(span, "should be a string");
                                            // Reconstruct
                                            *value =
                                                reconstruct_history_file_format(&config, span);
                                        }
                                    }
                                    _ => {
                                        invalid_key!(
                                            span,
                                            "$env.config.{key}.{key2} is an unknown config setting"
                                        );
                                        return false;
                                    }
                                };
                                true
                            });
                        } else {
                            invalid!(span, "should be a record");
                            // Reconstruct
                            *value = Value::record(
                                record! {
                                    "sync_on_enter" => Value::bool(config.sync_history_on_enter, span),
                                    "max_size" => Value::int(config.max_history_size, span),
                                    "file_format" => reconstruct_history_file_format(&config, span),
                                    "isolation" => Value::bool(config.history_isolation, span),
                                },
                                span,
                            );
                        }
                    }
                    "completions" => {
                        fn reconstruct_external_completer(config: &Config, span: Span) -> Value {
                            if let Some(block) = config.external_completer {
                                Value::block(block, span)
                            } else {
                                Value::nothing(span)
                            }
                        }

                        fn reconstruct_external(config: &Config, span: Span) -> Value {
                            Value::record(
                                record! {
                                    "max_results" => Value::int(config.max_external_completion_results, span),
                                    "completer" => reconstruct_external_completer(config, span),
                                    "enable" => Value::bool(config.enable_external_completion, span),
                                },
                                span,
                            )
                        }

                        if let Value::Record { val, .. } = value {
                            val.retain_mut(|key2, value| {
                                let span = value.span();
                                match key2 {
                                    "quick" => {
                                        try_bool!(value, quick_completions);
                                    }
                                    "partial" => {
                                        try_bool!(value, partial_completions);
                                    }
                                    "algorithm" => {
                                        if let Ok(v) = value.as_string() {
                                            let val_str = v.to_lowercase();
                                            match val_str.as_ref() {
                                                // This should match the MatchAlgorithm enum in completions::completion_options
                                                "prefix" | "fuzzy" => {
                                                    config.completion_algorithm = val_str
                                                }
                                                _ => {
                                                    invalid!(span,
                                                        "unrecognized $env.config.{key}.{key2} '{val_str}'; expected either 'prefix' or 'fuzzy'"
                                                    );
                                                    // Reconstruct
                                                    *value = Value::string(
                                                        config.completion_algorithm.clone(),
                                                        span,
                                                    );
                                                }
                                            };
                                        } else {
                                            invalid!(span, "should be a string");
                                            // Reconstruct
                                            *value = Value::string(
                                                config.completion_algorithm.clone(),
                                                span,
                                            );
                                        }
                                    }
                                    "case_sensitive" => {
                                        try_bool!(value, case_sensitive_completions);
                                    }
                                    "external" => {
                                        if let Value::Record { val, .. } = value {
                                            val.retain_mut(|key3, value|
                                                {
                                                    let span = value.span();
                                                    match key3 {
                                                    "max_results" => {
                                                        try_int!(value, max_external_completion_results);
                                                    }
                                                    "completer" => {
                                                        if let Ok(v) = value.as_block() {
                                                            config.external_completer = Some(v)
                                                        } else {
                                                            match value {
                                                                Value::Nothing { .. } => {}
                                                                _ => {
                                                                    invalid!(span, "should be a block or null");
                                                                    // Reconstruct
                                                                    *value = reconstruct_external_completer(&config,
                                                                        span
                                                                    );
                                                                }
                                                            }
                                                        }
                                                    }
                                                    "enable" => {
                                                        try_bool!(value, enable_external_completion);
                                                    }
                                                    _ => {
                                                        invalid_key!(
                                                            span,
                                                            "$env.config.{key}.{key2}.{key3} is an unknown config setting"
                                                    );
                                                        return false;
                                                    }
                                                    };
                                                    true
                                                });
                                        } else {
                                            invalid!(span, "should be a record");
                                            // Reconstruct
                                            *value = reconstruct_external(&config, span);
                                        }
                                    }
                                    _ => {
                                        invalid_key!(
                                            span,
                                            "$env.config.{key}.{key2} is an unknown config setting"
                                        );
                                        return false;
                                    }
                                };
                                true
                            });
                        } else {
                            invalid!(span, "should be a record");
                            // Reconstruct record
                            *value = Value::record(
                                record! {
                                    "quick" => Value::bool(config.quick_completions, span),
                                    "partial" => Value::bool(config.partial_completions, span),
                                    "algorithm" => Value::string(config.completion_algorithm.clone(), span),
                                    "case_sensitive" => Value::bool(config.case_sensitive_completions, span),
                                    "external" => reconstruct_external(&config, span),
                                },
                                span,
                            );
                        }
                    }
                    "cursor_shape" => {
                        if let Value::Record { val, .. } = value {
                            val.retain_mut(|key2, value| {
                                let span = value.span();
                                match key2 {
                                    "vi_insert" => {
                                        if let Ok(v) = value.as_string() {
                                            let val_str = v.to_lowercase();
                                            match val_str.as_ref() {
                                                "line" => {
                                                    config.cursor_shape_vi_insert =
                                                        Some(NuCursorShape::Line);
                                                }
                                                "block" => {
                                                    config.cursor_shape_vi_insert =
                                                        Some(NuCursorShape::Block);
                                                }
                                                "underscore" => {
                                                    config.cursor_shape_vi_insert =
                                                        Some(NuCursorShape::UnderScore);
                                                }
                                                "blink_line" => {
                                                    config.cursor_shape_vi_insert =
                                                        Some(NuCursorShape::BlinkLine);
                                                }
                                                "blink_block" => {
                                                    config.cursor_shape_vi_insert =
                                                        Some(NuCursorShape::BlinkBlock);
                                                }
                                                "blink_underscore" => {
                                                    config.cursor_shape_vi_insert =
                                                        Some(NuCursorShape::BlinkUnderScore);
                                                }
                                                "inherit" => {
                                                    config.cursor_shape_vi_insert = None;
                                                }
                                                _ => {
                                                    invalid!( span,
                                                        "unrecognized $env.config.{key}.{key2} '{val_str}'; expected either 'line', 'block', 'underscore', 'blink_line', 'blink_block', 'blink_underscore' or 'inherit'"
                                                    );
                                                    // Reconstruct
                                                    *value = reconstruct_cursor_shape(
                                                        config.cursor_shape_vi_insert,
                                                        span,
                                                    );
                                                }
                                            };
                                        } else {
                                            invalid!(span, "should be a string");
                                            // Reconstruct
                                            *value = reconstruct_cursor_shape(
                                                config.cursor_shape_vi_insert,
                                                span,
                                            );
                                        }
                                    }
                                    "vi_normal" => {
                                        if let Ok(v) = value.as_string() {
                                            let val_str = v.to_lowercase();
                                            match val_str.as_ref() {
                                                "line" => {
                                                    config.cursor_shape_vi_normal =
                                                        Some(NuCursorShape::Line);
                                                }
                                                "block" => {
                                                    config.cursor_shape_vi_normal =
                                                        Some(NuCursorShape::Block);
                                                }
                                                "underscore" => {
                                                    config.cursor_shape_vi_normal =
                                                        Some(NuCursorShape::UnderScore);
                                                }
                                                "blink_line" => {
                                                    config.cursor_shape_vi_normal =
                                                        Some(NuCursorShape::BlinkLine);
                                                }
                                                "blink_block" => {
                                                    config.cursor_shape_vi_normal =
                                                        Some(NuCursorShape::BlinkBlock);
                                                }
                                                "blink_underscore" => {
                                                    config.cursor_shape_vi_normal =
                                                        Some(NuCursorShape::BlinkUnderScore);
                                                }
                                                "inherit" => {
                                                    config.cursor_shape_vi_normal = None;
                                                }
                                                _ => {
                                                    invalid!(span,
                                                        "unrecognized $env.config.{key}.{key2} '{val_str}'; expected either 'line', 'block', 'underscore', 'blink_line', 'blink_block', 'blink_underscore' or 'inherit'"
                                                    );
                                                    // Reconstruct
                                                    *value = reconstruct_cursor_shape(
                                                        config.cursor_shape_vi_normal,
                                                        span,
                                                    );
                                                }
                                            };
                                        } else {
                                            invalid!(span, "should be a string");
                                            // Reconstruct
                                            *value = reconstruct_cursor_shape(
                                                config.cursor_shape_vi_normal,
                                                span,
                                            );
                                        }
                                    }
                                    "emacs" => {
                                        if let Ok(v) = value.as_string() {
                                            let val_str = v.to_lowercase();
                                            match val_str.as_ref() {
                                                "line" => {
                                                    config.cursor_shape_emacs =
                                                        Some(NuCursorShape::Line);
                                                }
                                                "block" => {
                                                    config.cursor_shape_emacs =
                                                        Some(NuCursorShape::Block);
                                                }
                                                "underscore" => {
                                                    config.cursor_shape_emacs =
                                                        Some(NuCursorShape::UnderScore);
                                                }
                                                "blink_line" => {
                                                    config.cursor_shape_emacs =
                                                        Some(NuCursorShape::BlinkLine);
                                                }
                                                "blink_block" => {
                                                    config.cursor_shape_emacs =
                                                        Some(NuCursorShape::BlinkBlock);
                                                }
                                                "blink_underscore" => {
                                                    config.cursor_shape_emacs =
                                                        Some(NuCursorShape::BlinkUnderScore);
                                                }
                                                "inherit" => {
                                                    config.cursor_shape_emacs = None;
                                                }
                                                _ => {
                                                    invalid!(span,
                                                        "unrecognized $env.config.{key}.{key2} '{val_str}'; expected either 'line', 'block', 'underscore', 'blink_line', 'blink_block', 'blink_underscore' or 'inherit'"
                                                    );
                                                    // Reconstruct
                                                    *value = reconstruct_cursor_shape(
                                                        config.cursor_shape_emacs,
                                                        span,
                                                    );
                                                }
                                            };
                                        } else {
                                            invalid!(span, "should be a string");
                                            // Reconstruct
                                            *value = reconstruct_cursor_shape(
                                                config.cursor_shape_emacs,
                                                span,
                                            );
                                        }
                                    }
                                    _ => {
                                        invalid_key!(
                                            span,
                                            "$env.config.{key}.{key2} is an unknown config setting"
                                        );
                                        return false;
                                    }
                                };
                                true
                            });
                        } else {
                            invalid!(span, "should be a record");
                            // Reconstruct
                            *value = Value::record(
                                record! {
                                    "vi_insert" => reconstruct_cursor_shape(config.cursor_shape_vi_insert, span),
                                    "vi_normal" => reconstruct_cursor_shape(config.cursor_shape_vi_normal, span),
                                    "emacs" => reconstruct_cursor_shape(config.cursor_shape_emacs, span),
                                },
                                span,
                            );
                        }
                    }
                    "table" => {
                        if let Value::Record { val, .. } = value {
                            val.retain_mut(|key2, value| {
                                let span = value.span();
                                match key2 {
                                    "mode" => {
                                        if let Ok(v) = value.as_string() {
                                            config.table_mode = v;
                                        } else {
                                            invalid!(span, "should be a string");
                                            *value =
                                                Value::string(config.table_mode.clone(), span);
                                        }
                                    }
                                    "header_on_separator" => {
                                        try_bool!(value, table_move_header);
                                    }
                                    "padding" => match value {
                                        Value::Int { val, .. } => {
                                            if *val < 0 {
                                                invalid!(span, "unexpected $env.config.{key}.{key2} '{val}'; expected a unsigned integer");
                                            }

                                            config.table_indent.left = *val as usize;
                                            config.table_indent.right = *val as usize;
                                        }
                                        Value::Record { val, .. } => {
                                            if let Some(left) = val.get("left") {
                                                match left.as_int() {
                                                    Ok(val) => {
                                                        if val < 0 {
                                                            invalid!(span, "unexpected $env.config.{key}.{key2} '{val}'; expected a unsigned integer");
                                                        }

                                                        config.table_indent.left = val as usize;
                                                    }
                                                    Err(_) => {
                                                        invalid!(span, "unexpected $env.config.{key}.{key2} value; expected a unsigned integer or a record");
                                                    }
                                                }
                                            }

                                            if let Some(right) = val.get("right") {
                                                match right.as_int() {
                                                    Ok(val) => {
                                                        if val < 0 {
                                                            invalid!(span, "unexpected $env.config.{key}.{key2} '{val}'; expected a unsigned integer");
                                                        }

                                                        config.table_indent.right = val as usize;
                                                    }
                                                    Err(_) => {
                                                        invalid!(span, "unexpected $env.config.{key}.{key2} value; expected a unsigned integer or a record");
                                                    }
                                                }
                                            }
                                        }
                                        _ => {
                                            invalid!(span, "unexpected $env.config.{key}.{key2} value; expected a unsigned integer or a record");
                                        }
                                    },
                                    "index_mode" => {
                                        if let Ok(b) = value.as_string() {
                                            let val_str = b.to_lowercase();
                                            match val_str.as_ref() {
                                                "always" => {
                                                    config.table_index_mode = TableIndexMode::Always
                                                }
                                                "never" => {
                                                    config.table_index_mode = TableIndexMode::Never
                                                }
                                                "auto" => {
                                                    config.table_index_mode = TableIndexMode::Auto
                                                }
                                                _ => {
                                                    invalid!( span,
                                                        "unrecognized $env.config.{key}.{key2} '{val_str}'; expected either 'never', 'always' or 'auto'"
                                                    );
                                                    *value =
                                                        reconstruct_index_mode(&config, span);
                                                }
                                            }
                                        } else {
                                            invalid!(span, "should be a string");
                                            *value = reconstruct_index_mode(&config, span);
                                        }
                                    }
                                    "trim" => {
                                        match try_parse_trim_strategy(value, &mut errors) {
                                            Ok(v) => config.trim_strategy = v,
                                            Err(e) => {
                                                // try_parse_trim_strategy() already adds its own errors
                                                errors.push(e);
                                                *value =
                                                    reconstruct_trim_strategy(&config, span);
                                            }
                                        }
                                    }
                                    "show_empty" => {
                                        try_bool!(value, table_show_empty);
                                    }
                                    "abbreviated_row_count" => {
                                        if let Ok(b) = value.as_int() {
                                            if b < 0 {
                                                invalid!(span, "should be an int unsigned");
                                            }

                                            config.table_abbreviation_threshold = Some(b as usize);
                                        } else {
                                            invalid!(span, "should be an int");
                                        }
                                    }
                                    x => {
                                        invalid_key!(
                                            span,
                                            "$env.config.{key}.{x} is an unknown config setting"
                                        );
                                        return false
                                    }
                                };
                                true
                             });
                        } else {
                            invalid!(span, "should be a record");
                            // Reconstruct
                            *value = Value::record(
                                record! {
                                    "mode" => Value::string(config.table_mode.clone(), span),
                                    "index_mode" => reconstruct_index_mode(&config, span),
                                    "trim" => reconstruct_trim_strategy(&config, span),
                                    "show_empty" => Value::bool(config.table_show_empty, span),
                                },
                                span,
                            );
                        }
                    }
                    "filesize" => {
                        if let Value::Record { val, .. } = value {
                            val.retain_mut(|key2, value| {
                                let span = value.span();
                                match key2 {
                                "metric" => {
                                    try_bool!(value, filesize_metric);
                                }
                                "format" => {
                                    if let Ok(v) = value.as_string() {
                                        config.filesize_format = v.to_lowercase();
                                    } else {
                                        invalid!(span, "should be a string");
                                        // Reconstruct
                                        *value =
                                            Value::string(config.filesize_format.clone(), span);
                                    }
                                }
                                _ => {
                                    invalid_key!(
                                        span,
                                        "$env.config.{key}.{key2} is an unknown config setting"
                                    );
                                    return false;
                                }
                            };
                            true
                        })
                        } else {
                            invalid!(span, "should be a record");
                            // Reconstruct
                            *value = Value::record(
                                record! {
                                    "metric" => Value::bool(config.filesize_metric, span),
                                    "format" => Value::string(config.filesize_format.clone(), span),
                                },
                                span,
                            );
                        }
                    }
                    "explore" => {
                        if let Ok(map) = create_map(value) {
                            config.explore = map;
                        } else {
                            invalid!(span, "should be a record");
                            // Reconstruct
                            *value = Value::record(
                                config
                                    .explore
                                    .iter()
                                    .map(|(k, v)| (k.clone(), v.clone()))
                                    .collect(),
                                span,
                            );
                        }
                    }
                    // Misc. options
                    "color_config" => {
                        if let Ok(map) = create_map(value) {
                            config.color_config = map;
                        } else {
                            invalid!(span, "should be a record");
                            // Reconstruct
                            *value = Value::record(
                                config
                                    .color_config
                                    .iter()
                                    .map(|(k, v)| (k.clone(), v.clone()))
                                    .collect(),
                                span,
                            );
                        }
                    }
                    "use_grid_icons" => {
                        try_bool!(value, use_grid_icons);
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
                            invalid!(span, "should be a string");
                            // Reconstruct
                            *value = Value::string(
                                match config.footer_mode {
                                    FooterMode::Auto => "auto".into(),
                                    FooterMode::Never => "never".into(),
                                    FooterMode::Always => "always".into(),
                                    FooterMode::RowCount(number) => number.to_string(),
                                },
                                span,
                            );
                        }
                    }
                    "float_precision" => {
                        try_int!(value, float_precision);
                    }
                    "use_ansi_coloring" => {
                        try_bool!(value, use_ansi_coloring);
                    }
                    "edit_mode" => {
                        if let Ok(v) = value.as_string() {
                            config.edit_mode = v.to_lowercase();
                        } else {
                            invalid!(span, "should be a string");
                            // Reconstruct
                            *value = Value::string(config.edit_mode.clone(), span);
                        }
                    }
                    "shell_integration" => {
                        try_bool!(value, shell_integration);
                    }
                    "buffer_editor" => match value {
                        Value::Nothing { .. } | Value::String { .. } => {
                            config.buffer_editor = value.clone();
                        }
                        Value::List { vals, .. }
                            if vals.iter().all(|val| matches!(val, Value::String { .. })) =>
                        {
                            config.buffer_editor = value.clone();
                        }
                        _ => {
                            invalid!(span, "should be a string, list<string>, or null");
                        }
                    },
                    "show_banner" => {
                        try_bool!(value, show_banner);
                    }
                    "render_right_prompt_on_last_line" => {
                        try_bool!(value, render_right_prompt_on_last_line);
                    }
                    "bracketed_paste" => {
                        try_bool!(value, bracketed_paste);
                    }
                    "use_kitty_protocol" => {
                        try_bool!(value, use_kitty_protocol);
                    }
                    // Menus
                    "menus" => match create_menus(value) {
                        Ok(map) => config.menus = map,
                        Err(e) => {
                            invalid!(span, "should be a valid list of menus");
                            errors.push(e);
                            // Reconstruct
                            *value = Value::list(config
                                    .menus
                                    .iter()
                                    .map(
                                        |ParsedMenu {
                                             name,
                                             only_buffer_difference,
                                             marker,
                                             style,
                                             menu_type, // WARNING: this is not the same name as what is used in Config.nu! ("type")
                                             source,
                                         }| {
                                            Value::record(
                                                record! {
                                                    "name" => name.clone(),
                                                    "only_buffer_difference" => only_buffer_difference.clone(),
                                                    "marker" => marker.clone(),
                                                    "style" => style.clone(),
                                                    "type" => menu_type.clone(),
                                                    "source" => source.clone(),
                                                },
                                                span,
                                            )
                                        },
                                    )
                                    .collect(),
                                span,
                            )
                        }
                    },
                    // Keybindings
                    "keybindings" => match create_keybindings(value) {
                        Ok(keybindings) => config.keybindings = keybindings,
                        Err(e) => {
                            invalid!(span, "should be a valid keybindings list");
                            errors.push(e);
                            // Reconstruct
                            *value = Value::list(
                                config
                                    .keybindings
                                    .iter()
                                    .map(
                                        |ParsedKeybinding {
                                             modifier,
                                             keycode,
                                             mode,
                                             event,
                                         }| {
                                            Value::record(
                                                record! {
                                                    "modifier" => modifier.clone(),
                                                    "keycode" => keycode.clone(),
                                                    "mode" => mode.clone(),
                                                    "event" => event.clone(),
                                                },
                                                span,
                                            )
                                        },
                                    )
                                    .collect(),
                                span,
                            )
                        }
                    },
                    // Hooks
                    "hooks" => match create_hooks(value) {
                        Ok(hooks) => config.hooks = hooks,
                        Err(e) => {
                            invalid!(span, "should be a valid hooks list");
                            errors.push(e);
                            // Reconstruct
                            let mut hook = Record::new();
                            if let Some(ref value) = config.hooks.pre_prompt {
                                hook.push("pre_prompt", value.clone());
                            }
                            if let Some(ref value) = config.hooks.pre_execution {
                                hook.push("pre_execution", value.clone());
                            }
                            if let Some(ref value) = config.hooks.env_change {
                                hook.push("env_change", value.clone());
                            }
                            if let Some(ref value) = config.hooks.display_output {
                                hook.push("display_output", value.clone());
                            }
                            *value = Value::record(hook, span);
                        }
                    },
                    "datetime_format" => {
                        if let Value::Record { val, .. } = value {
                            val.retain_mut(|key2, value|
                                {
                                let span = value.span();
                                match key2 {
                                "normal" => {
                                    if let Ok(v) = value.as_string() {
                                        config.datetime_normal_format = Some(v);
                                    } else {
                                        invalid!(span, "should be a string");
                                    }
                                }
                                "table" => {
                                    if let Ok(v) = value.as_string() {
                                        config.datetime_table_format = Some(v);
                                    } else {
                                        invalid!(span, "should be a string");
                                    }
                                }
                                x => {
                                    invalid_key!(
                                        span,
                                        "$env.config.{key}.{x} is an unknown config setting"
                                    );
                                    return false;
                                }
                            }; true})
                        } else {
                            invalid!(span, "should be a record");
                            // Reconstruct
                            *value = Value::record(
                                record! {
                                    "metric" => Value::bool(config.filesize_metric, span),
                                    "format" => Value::string(config.filesize_format.clone(), span),
                                },
                                span,
                            );
                        }
                    }
                    "error_style" => {
                        if let Ok(style) = value.as_string() {
                            config.error_style = style;
                        } else {
                            invalid!(span, "should be a string");
                            *value = Value::string(config.error_style.clone(), span);
                        }
                    }
                    // Catch all
                    _ => {
                        invalid_key!(
                            span,
                            "$env.config.{key} is an unknown config setting"
                        );
                        return false;
                    }
            };
            true
        });
        } else {
            return (
                config,
                Some(ShellError::GenericError(
                    "Error while applying config changes".into(),
                    "$env.config is not a record".into(),
                    Some(self.span()),
                    None,
                    vec![],
                )),
            );
        }

        // Return the config and the vec of errors.
        (
            config,
            if !errors.is_empty() {
                Some(ShellError::GenericError(
                    "Config record contains invalid values or unknown settings".into(),
                    // Without a span, this second string is ignored.
                    "".into(),
                    None,
                    None,
                    errors,
                ))
            } else {
                None
            },
        )
    }
}

fn try_parse_trim_strategy(
    value: &Value,
    errors: &mut Vec<ShellError>,
) -> Result<TrimStrategy, ShellError> {
    let map = create_map(value).map_err(|e| {
        ShellError::GenericError(
            "Error while applying config changes".into(),
            "$env.config.table.trim is not a record".into(),
            Some(value.span()),
            Some("Please consult the documentation for configuring Nushell.".into()),
            vec![e],
        )
    })?;

    let mut methodology = match map.get("methodology") {
        Some(value) => match try_parse_trim_methodology(value) {
            Some(methodology) => methodology,
            None => return Ok(TRIM_STRATEGY_DEFAULT),
        },
        None => {
            errors.push(ShellError::GenericError(
                "Error while applying config changes".into(),
                "$env.config.table.trim.methodology was not provided".into(),
                Some(value.span()),
                Some("Please consult the documentation for configuring Nushell.".into()),
                vec![],
            ));
            return Ok(TRIM_STRATEGY_DEFAULT);
        }
    };

    match &mut methodology {
        TrimStrategy::Wrap { try_to_keep_words } => {
            if let Some(value) = map.get("wrapping_try_keep_words") {
                if let Ok(b) = value.as_bool() {
                    *try_to_keep_words = b;
                } else {
                    errors.push(ShellError::GenericError(
                        "Error while applying config changes".into(),
                        "$env.config.table.trim.wrapping_try_keep_words is not a bool".into(),
                        Some(value.span()),
                        Some("Please consult the documentation for configuring Nushell.".into()),
                        vec![],
                    ));
                }
            }
        }
        TrimStrategy::Truncate { suffix } => {
            if let Some(value) = map.get("truncating_suffix") {
                if let Ok(v) = value.as_string() {
                    *suffix = Some(v);
                } else {
                    errors.push(ShellError::GenericError(
                        "Error while applying config changes".into(),
                        "$env.config.table.trim.truncating_suffix is not a string".into(),
                        Some(value.span()),
                        Some("Please consult the documentation for configuring Nushell.".into()),
                        vec![],
                    ));
                }
            }
        }
    }

    Ok(methodology)
}

fn try_parse_trim_methodology(value: &Value) -> Option<TrimStrategy> {
    if let Ok(value) = value.as_string() {
        match value.to_lowercase().as_str() {
            "wrapping" => {
                return Some(TrimStrategy::Wrap {
                    try_to_keep_words: false,
                });
            }
            "truncating" => return Some(TrimStrategy::Truncate { suffix: None }),
            _ => eprintln!("unrecognized $config.table.trim.methodology value; expected either 'truncating' or 'wrapping'"),
        }
    } else {
        eprintln!("$env.config.table.trim.methodology is not a string")
    }

    None
}

fn create_map(value: &Value) -> Result<HashMap<String, Value>, ShellError> {
    Ok(value
        .as_record()?
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect())
}

// Parse the hooks to find the blocks to run when the hooks fire
fn create_hooks(value: &Value) -> Result<Hooks, ShellError> {
    let span = value.span();
    match value {
        Value::Record { val, .. } => {
            let mut hooks = Hooks::new();

            for (col, val) in val {
                match col.as_str() {
                    "pre_prompt" => hooks.pre_prompt = Some(val.clone()),
                    "pre_execution" => hooks.pre_execution = Some(val.clone()),
                    "env_change" => hooks.env_change = Some(val.clone()),
                    "display_output" => hooks.display_output = Some(val.clone()),
                    "command_not_found" => hooks.command_not_found = Some(val.clone()),
                    x => {
                        return Err(ShellError::UnsupportedConfigValue(
                            "'pre_prompt', 'pre_execution', 'env_change', 'display_output', 'command_not_found'"
                                .to_string(),
                            x.to_string(),
                            span,
                        ));
                    }
                }
            }

            Ok(hooks)
        }
        _ => Err(ShellError::UnsupportedConfigValue(
            "record for 'hooks' config".into(),
            "non-record value".into(),
            span,
        )),
    }
}

// Parses the config object to extract the strings that will compose a keybinding for reedline
fn create_keybindings(value: &Value) -> Result<Vec<ParsedKeybinding>, ShellError> {
    let span = value.span();
    match value {
        Value::Record { val, .. } => {
            // Finding the modifier value in the record
            let modifier = extract_value("modifier", val, span)?.clone();
            let keycode = extract_value("keycode", val, span)?.clone();
            let mode = extract_value("mode", val, span)?.clone();
            let event = extract_value("event", val, span)?.clone();

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
    let span = value.span();
    match value {
        Value::Record { val, .. } => {
            // Finding the modifier value in the record
            let name = extract_value("name", val, span)?.clone();
            let marker = extract_value("marker", val, span)?.clone();
            let only_buffer_difference =
                extract_value("only_buffer_difference", val, span)?.clone();
            let style = extract_value("style", val, span)?.clone();
            let menu_type = extract_value("type", val, span)?.clone();

            // Source is an optional value
            let source = match extract_value("source", val, span) {
                Ok(source) => source.clone(),
                Err(_) => Value::nothing(span),
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
    record: &'record Record,
    span: Span,
) -> Result<&'record Value, ShellError> {
    record
        .get(name)
        .ok_or_else(|| ShellError::MissingConfigValue(name.to_string(), span))
}
