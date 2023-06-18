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
    pub command_not_found: Option<Value>,
}

impl Hooks {
    pub fn new() -> Self {
        Self {
            pre_prompt: None,
            pre_execution: None,
            env_change: None,
            display_output: None,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub external_completer: Option<usize>,
    pub filesize_metric: bool,
    pub table_mode: String,
    pub table_show_empty: bool,
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
    pub bracketed_paste: bool,
    pub show_clickable_links_in_ls: bool,
    pub render_right_prompt_on_last_line: bool,
    pub explore: HashMap<String, Value>,
    pub cursor_shape_vi_insert: NuCursorShape,
    pub cursor_shape_vi_normal: NuCursorShape,
    pub cursor_shape_emacs: NuCursorShape,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            filesize_metric: false,
            table_mode: "rounded".into(),
            table_show_empty: true,
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
            history_isolation: false,
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
            bracketed_paste: true,
            show_clickable_links_in_ls: true,
            render_right_prompt_on_last_line: false,
            explore: HashMap::new(),
            cursor_shape_vi_insert: NuCursorShape::Block,
            cursor_shape_vi_normal: NuCursorShape::UnderScore,
            cursor_shape_emacs: NuCursorShape::Line,
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

impl Value {
    pub fn into_config(&mut self, config: &Config) -> (Config, Option<ShellError>) {
        // Clone the passed-in config rather than mutating it.

        let mut config = config.clone();
        let mut legacy_options_used = false;

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
                    $span,
                    Some("This value will be ignored.".into()),
                    vec![],
                ));
            };
        }
        // Some extra helpers
        macro_rules! try_bool {
            ($cols:ident, $vals:ident, $index:ident, $span:expr, $setting:ident) => {
                if let Ok(b) = &$vals[$index].as_bool() {
                    config.$setting = *b;
                } else {
                    invalid!(Some(*$span), "should be a bool");
                    // Reconstruct
                    $vals[$index] = Value::boolean(config.$setting, *$span);
                }
            };
        }
        macro_rules! try_int {
            ($cols:ident, $vals:ident, $index:ident, $span:expr, $setting:ident) => {
                if let Ok(b) = &$vals[$index].as_integer() {
                    config.$setting = *b;
                } else {
                    invalid!(Some(*$span), "should be an int");
                    // Reconstruct
                    $vals[$index] = Value::int(config.$setting, *$span);
                }
            };
        }
        // When an unsupported config value is found, remove it from this record.
        macro_rules! invalid_key {
            // Because Value::Record discards all of the spans of its
            // column names (by storing them as Strings), the key name cannot be provided
            // as a value, even in key errors.
            ($cols:ident, $vals:ident, $index:ident, $span:expr, $msg:literal) => {
                errors.push(ShellError::GenericError(
                    "Error while applying config changes".into(),
                    format!($msg),
                    $span,
                    Some("This value will not appear in your $env.config record.".into()),
                    vec![],
                ));
                $cols.remove($index);
                $vals.remove($index);
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
        if let Value::Record { cols, vals, span } = self {
            // Because this whole algorithm removes while iterating, this must iterate in reverse.
            for index in (0..cols.len()).rev() {
                let value = &vals[index];
                let key = cols[index].as_str();
                match key {
                    // Grouped options
                    "ls" => {
                        if let Value::Record { cols, vals, span } = &mut vals[index] {
                            for index in (0..cols.len()).rev() {
                                let value = &vals[index];
                                let key2 = cols[index].as_str();
                                match key2 {
                                    "use_ls_colors" => {
                                        try_bool!(cols, vals, index, span, use_ls_colors)
                                    }
                                    "clickable_links" => try_bool!(
                                        cols,
                                        vals,
                                        index,
                                        span,
                                        show_clickable_links_in_ls
                                    ),
                                    x => {
                                        invalid_key!(
                                            cols,
                                            vals,
                                            index,
                                            value.span().ok(),
                                            "$env.config.{key}.{x} is an unknown config setting"
                                        );
                                    }
                                }
                            }
                        } else {
                            invalid!(vals[index].span().ok(), "should be a record");
                            // Reconstruct
                            vals[index] = Value::record(
                                vec!["use_ls_colors".into(), "clickable_links".into()],
                                vec![
                                    Value::boolean(config.use_ls_colors, *span),
                                    Value::boolean(config.show_clickable_links_in_ls, *span),
                                ],
                                *span,
                            );
                        }
                    }
                    "cd" => {
                        if let Value::Record { cols, vals, span } = &mut vals[index] {
                            for index in (0..cols.len()).rev() {
                                let value = &vals[index];
                                let key2 = cols[index].as_str();
                                match key2 {
                                    "abbreviations" => {
                                        try_bool!(cols, vals, index, span, cd_with_abbreviations)
                                    }
                                    x => {
                                        invalid_key!(
                                            cols,
                                            vals,
                                            index,
                                            value.span().ok(),
                                            "$env.config.{key}.{x} is an unknown config setting"
                                        );
                                    }
                                }
                            }
                        } else {
                            invalid!(vals[index].span().ok(), "should be a record");
                            // Reconstruct
                            vals[index] = Value::record(
                                vec!["use_ls_colors".into(), "clickable_links".into()],
                                vec![
                                    Value::boolean(config.use_ls_colors, *span),
                                    Value::boolean(config.show_clickable_links_in_ls, *span),
                                ],
                                *span,
                            );
                        }
                    }
                    "rm" => {
                        if let Value::Record { cols, vals, span } = &mut vals[index] {
                            for index in (0..cols.len()).rev() {
                                let value = &vals[index];
                                let key2 = cols[index].as_str();
                                match key2 {
                                    "always_trash" => {
                                        try_bool!(cols, vals, index, span, rm_always_trash)
                                    }
                                    x => {
                                        invalid_key!(
                                            cols,
                                            vals,
                                            index,
                                            value.span().ok(),
                                            "$env.config.{key}.{x} is an unknown config setting"
                                        );
                                    }
                                }
                            }
                        } else {
                            invalid!(vals[index].span().ok(), "should be a record");
                            // Reconstruct
                            vals[index] = Value::record(
                                vec!["always_trash".into()],
                                vec![Value::boolean(config.rm_always_trash, *span)],
                                *span,
                            );
                        }
                    }
                    "history" => {
                        macro_rules! reconstruct_history_file_format {
                            ($span:expr) => {
                                Value::string(
                                    match config.history_file_format {
                                        HistoryFileFormat::Sqlite => "sqlite",
                                        HistoryFileFormat::PlainText => "plaintext",
                                    },
                                    *$span,
                                )
                            };
                        }
                        if let Value::Record { cols, vals, span } = &mut vals[index] {
                            for index in (0..cols.len()).rev() {
                                let value = &vals[index];
                                let key2 = cols[index].as_str();
                                match key2 {
                                    "history_isolation" => {
                                        try_bool!(cols, vals, index, span, history_isolation)
                                    }
                                    "sync_on_enter" => {
                                        try_bool!(cols, vals, index, span, sync_history_on_enter)
                                    }
                                    "max_size" => {
                                        try_int!(cols, vals, index, span, max_history_size)
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
                                                    invalid!(Some(*span),
                                                        "unrecognized $env.config.{key}.{key2} '{val_str}'; expected either 'sqlite' or 'plaintext'"
                                                    );
                                                    // Reconstruct
                                                    vals[index] =
                                                        reconstruct_history_file_format!(span);
                                                }
                                            };
                                        } else {
                                            invalid!(Some(*span), "should be a string");
                                            // Reconstruct
                                            vals[index] = reconstruct_history_file_format!(span);
                                        }
                                    }
                                    x => {
                                        invalid_key!(
                                            cols,
                                            vals,
                                            index,
                                            value.span().ok(),
                                            "$env.config.{key}.{x} is an unknown config setting"
                                        );
                                    }
                                }
                            }
                        } else {
                            invalid!(vals[index].span().ok(), "should be a record");
                            // Reconstruct
                            vals[index] = Value::record(
                                vec![
                                    "sync_on_enter".into(),
                                    "max_size".into(),
                                    "file_format".into(),
                                ],
                                vec![
                                    Value::boolean(config.sync_history_on_enter, *span),
                                    Value::int(config.max_history_size, *span),
                                    reconstruct_history_file_format!(span),
                                ],
                                *span,
                            );
                        }
                    }
                    "completions" => {
                        macro_rules! reconstruct_external_completer {
                            ($span: expr) => {
                                if let Some(block) = config.external_completer {
                                    Value::Block {
                                        val: block,
                                        span: *$span,
                                    }
                                } else {
                                    Value::Nothing { span: *$span }
                                }
                            };
                        }
                        macro_rules! reconstruct_external {
                            ($span: expr) => {
                                Value::record(
                                    vec!["max_results".into(), "completer".into(), "enable".into()],
                                    vec![
                                        Value::int(config.max_external_completion_results, *$span),
                                        reconstruct_external_completer!($span),
                                        Value::boolean(config.enable_external_completion, *$span),
                                    ],
                                    *$span,
                                )
                            };
                        }
                        if let Value::Record { cols, vals, span } = &mut vals[index] {
                            for index in (0..cols.len()).rev() {
                                let value = &vals[index];
                                let key2 = cols[index].as_str();
                                match key2 {
                                    "quick" => {
                                        try_bool!(cols, vals, index, span, quick_completions)
                                    }
                                    "partial" => {
                                        try_bool!(cols, vals, index, span, partial_completions)
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
                                                    invalid!( Some(*span),
                                                        "unrecognized $env.config.{key}.{key2} '{val_str}'; expected either 'prefix' or 'fuzzy'"
                                                    );
                                                    // Reconstruct
                                                    vals[index] = Value::string(
                                                        config.completion_algorithm.clone(),
                                                        *span,
                                                    );
                                                }
                                            };
                                        } else {
                                            invalid!(Some(*span), "should be a string");
                                            // Reconstruct
                                            vals[index] = Value::string(
                                                config.completion_algorithm.clone(),
                                                *span,
                                            );
                                        }
                                    }
                                    "case_sensitive" => {
                                        try_bool!(
                                            cols,
                                            vals,
                                            index,
                                            span,
                                            case_sensitive_completions
                                        )
                                    }
                                    "external" => {
                                        if let Value::Record { cols, vals, span } = &mut vals[index]
                                        {
                                            for index in (0..cols.len()).rev() {
                                                let value = &vals[index];
                                                let key3 = cols[index].as_str();
                                                match key3 {
                                                    "max_results" => {
                                                        try_int!(
                                                            cols,
                                                            vals,
                                                            index,
                                                            span,
                                                            max_external_completion_results
                                                        )
                                                    }
                                                    "completer" => {
                                                        if let Ok(v) = value.as_block() {
                                                            config.external_completer = Some(v)
                                                        } else {
                                                            match value {
                                                                Value::Nothing { .. } => {}
                                                                _ => {
                                                                    invalid!(
                                                                        Some(*span),
                                                                        "should be a block or null"
                                                                    );
                                                                    // Reconstruct
                                                                    vals[index] = reconstruct_external_completer!(
                                                                        span
                                                                    );
                                                                }
                                                            }
                                                        }
                                                    }
                                                    "enable" => {
                                                        try_bool!(
                                                            cols,
                                                            vals,
                                                            index,
                                                            span,
                                                            enable_external_completion
                                                        )
                                                    }
                                                    x => {
                                                        invalid_key!(
                                                            cols,
                                                            vals,
                                                            index,
                                                            value.span().ok(),
                                                            "$env.config.{key}.{key2}.{x} is an unknown config setting"
                                                    );
                                                    }
                                                }
                                            }
                                        } else {
                                            invalid!(Some(*span), "should be a record");
                                            // Reconstruct
                                            vals[index] = reconstruct_external!(span);
                                        }
                                    }
                                    x => {
                                        invalid_key!(
                                            cols,
                                            vals,
                                            index,
                                            value.span().ok(),
                                            "$env.config.{key}.{x} is an unknown config setting"
                                        );
                                    }
                                }
                            }
                        } else {
                            invalid!(vals[index].span().ok(), "should be a record");
                            // Reconstruct record
                            vals[index] = Value::record(
                                vec![
                                    "quick".into(),
                                    "partial".into(),
                                    "algorithm".into(),
                                    "case_sensitive".into(),
                                    "external".into(),
                                ],
                                vec![
                                    Value::boolean(config.quick_completions, *span),
                                    Value::boolean(config.partial_completions, *span),
                                    Value::string(config.completion_algorithm.clone(), *span),
                                    Value::boolean(config.case_sensitive_completions, *span),
                                    reconstruct_external!(span),
                                ],
                                *span,
                            );
                        }
                    }
                    "cursor_shape" => {
                        macro_rules! reconstruct_cursor_shape {
                            ($name:expr, $span:expr) => {
                                Value::string(
                                    match $name {
                                        NuCursorShape::Line => "line",
                                        NuCursorShape::Block => "block",
                                        NuCursorShape::UnderScore => "underscore",
                                        NuCursorShape::BlinkLine => "blink_line",
                                        NuCursorShape::BlinkBlock => "blink_block",
                                        NuCursorShape::BlinkUnderScore => "blink_underscore",
                                    },
                                    *$span,
                                )
                            };
                        }
                        if let Value::Record { cols, vals, span } = &mut vals[index] {
                            for index in (0..cols.len()).rev() {
                                let value = &vals[index];
                                let key2 = cols[index].as_str();
                                match key2 {
                                    "vi_insert" => {
                                        if let Ok(v) = value.as_string() {
                                            let val_str = v.to_lowercase();
                                            match val_str.as_ref() {
                                                "line" => {
                                                    config.cursor_shape_vi_insert =
                                                        NuCursorShape::Line;
                                                }
                                                "block" => {
                                                    config.cursor_shape_vi_insert =
                                                        NuCursorShape::Block;
                                                }
                                                "underscore" => {
                                                    config.cursor_shape_vi_insert =
                                                        NuCursorShape::UnderScore;
                                                }
                                                "blink_line" => {
                                                    config.cursor_shape_vi_insert =
                                                        NuCursorShape::BlinkLine;
                                                }
                                                "blink_block" => {
                                                    config.cursor_shape_vi_insert =
                                                        NuCursorShape::BlinkBlock;
                                                }
                                                "blink_underscore" => {
                                                    config.cursor_shape_vi_insert =
                                                        NuCursorShape::BlinkUnderScore;
                                                }
                                                _ => {
                                                    invalid!(Some(*span),
                                                        "unrecognized $env.config.{key}.{key2} '{val_str}'; expected either 'line', 'block', 'underscore', 'blink_line', 'blink_block', or 'blink_underscore'"
                                                    );
                                                    // Reconstruct
                                                    vals[index] = reconstruct_cursor_shape!(
                                                        config.cursor_shape_vi_insert,
                                                        span
                                                    );
                                                }
                                            };
                                        } else {
                                            invalid!(Some(*span), "should be a string");
                                            // Reconstruct
                                            vals[index] = reconstruct_cursor_shape!(
                                                config.cursor_shape_vi_insert,
                                                span
                                            );
                                        }
                                    }
                                    "vi_normal" => {
                                        if let Ok(v) = value.as_string() {
                                            let val_str = v.to_lowercase();
                                            match val_str.as_ref() {
                                                "line" => {
                                                    config.cursor_shape_vi_normal =
                                                        NuCursorShape::Line;
                                                }
                                                "block" => {
                                                    config.cursor_shape_vi_normal =
                                                        NuCursorShape::Block;
                                                }
                                                "underscore" => {
                                                    config.cursor_shape_vi_normal =
                                                        NuCursorShape::UnderScore;
                                                }
                                                "blink_line" => {
                                                    config.cursor_shape_vi_normal =
                                                        NuCursorShape::BlinkLine;
                                                }
                                                "blink_block" => {
                                                    config.cursor_shape_vi_normal =
                                                        NuCursorShape::BlinkBlock;
                                                }
                                                "blink_underscore" => {
                                                    config.cursor_shape_vi_normal =
                                                        NuCursorShape::BlinkUnderScore;
                                                }
                                                _ => {
                                                    invalid!(Some(*span),
                                                        "unrecognized $env.config.{key}.{key2} '{val_str}'; expected either 'line', 'block', 'underscore', 'blink_line', 'blink_block', or 'blink_underscore'"
                                                    );
                                                    // Reconstruct
                                                    vals[index] = reconstruct_cursor_shape!(
                                                        config.cursor_shape_vi_normal,
                                                        span
                                                    );
                                                }
                                            };
                                        } else {
                                            invalid!(Some(*span), "should be a string");
                                            // Reconstruct
                                            vals[index] = reconstruct_cursor_shape!(
                                                config.cursor_shape_vi_normal,
                                                span
                                            );
                                        }
                                    }
                                    "emacs" => {
                                        if let Ok(v) = value.as_string() {
                                            let val_str = v.to_lowercase();
                                            match val_str.as_ref() {
                                                "line" => {
                                                    config.cursor_shape_emacs = NuCursorShape::Line;
                                                }
                                                "block" => {
                                                    config.cursor_shape_emacs =
                                                        NuCursorShape::Block;
                                                }
                                                "underscore" => {
                                                    config.cursor_shape_emacs =
                                                        NuCursorShape::UnderScore;
                                                }
                                                "blink_line" => {
                                                    config.cursor_shape_emacs =
                                                        NuCursorShape::BlinkLine;
                                                }
                                                "blink_block" => {
                                                    config.cursor_shape_emacs =
                                                        NuCursorShape::BlinkBlock;
                                                }
                                                "blink_underscore" => {
                                                    config.cursor_shape_emacs =
                                                        NuCursorShape::BlinkUnderScore;
                                                }
                                                _ => {
                                                    invalid!(Some(*span),
                                                        "unrecognized $env.config.{key}.{key2} '{val_str}'; expected either 'line', 'block', 'underscore', 'blink_line', 'blink_block', or 'blink_underscore'"
                                                    );
                                                    // Reconstruct
                                                    vals[index] = reconstruct_cursor_shape!(
                                                        config.cursor_shape_emacs,
                                                        span
                                                    );
                                                }
                                            };
                                        } else {
                                            invalid!(Some(*span), "should be a string");
                                            // Reconstruct
                                            vals[index] = reconstruct_cursor_shape!(
                                                config.cursor_shape_emacs,
                                                span
                                            );
                                        }
                                    }
                                    x => {
                                        invalid_key!(
                                            cols,
                                            vals,
                                            index,
                                            value.span().ok(),
                                            "$env.config.{key}.{x} is an unknown config setting"
                                        );
                                    }
                                }
                            }
                        } else {
                            invalid!(vals[index].span().ok(), "should be a record");
                            // Reconstruct
                            vals[index] = Value::record(
                                vec!["vi_insert".into(), "vi_normal".into(), "emacs".into()],
                                vec![
                                    reconstruct_cursor_shape!(config.cursor_shape_vi_insert, span),
                                    reconstruct_cursor_shape!(config.cursor_shape_vi_normal, span),
                                    reconstruct_cursor_shape!(config.cursor_shape_emacs, span),
                                ],
                                *span,
                            );
                        }
                    }
                    "table" => {
                        macro_rules! reconstruct_index_mode {
                            ($span:expr) => {
                                Value::string(
                                    match config.table_index_mode {
                                        TableIndexMode::Always => "always",
                                        TableIndexMode::Never => "never",
                                        TableIndexMode::Auto => "auto",
                                    },
                                    *$span,
                                )
                            };
                        }
                        macro_rules! reconstruct_trim_strategy {
                            ($span:expr) => {
                                match &config.trim_strategy {
                                    TrimStrategy::Wrap { try_to_keep_words } => Value::record(
                                        vec![
                                            "methodology".into(),
                                            "wrapping_try_keep_words".into(),
                                        ],
                                        vec![
                                            Value::string("wrapping", *$span),
                                            Value::boolean(*try_to_keep_words, *$span),
                                        ],
                                        *$span,
                                    ),
                                    TrimStrategy::Truncate { suffix } => Value::record(
                                        vec!["methodology".into(), "truncating_suffix".into()],
                                        match suffix {
                                            Some(s) => vec![
                                                Value::string("truncating", *$span),
                                                Value::string(s.clone(), *$span),
                                            ],
                                            None => vec![
                                                Value::string("truncating", *$span),
                                                Value::Nothing { span: *span },
                                            ],
                                        },
                                        *$span,
                                    ),
                                }
                            };
                        }
                        if let Value::Record { cols, vals, span } = &mut vals[index] {
                            for index in (0..cols.len()).rev() {
                                let value = &vals[index];
                                let key2 = cols[index].as_str();
                                match key2 {
                                    "mode" => {
                                        if let Ok(v) = value.as_string() {
                                            config.table_mode = v;
                                        } else {
                                            invalid!(Some(*span), "should be a string");
                                            vals[index] =
                                                Value::string(config.table_mode.clone(), *span);
                                        }
                                    }
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
                                                    invalid!( Some(*span),
                                                        "unrecognized $env.config.{key}.{key2} '{val_str}'; expected either 'never', 'always' or 'auto'"
                                                    );
                                                    vals[index] = reconstruct_index_mode!(span);
                                                }
                                            }
                                        } else {
                                            invalid!(Some(*span), "should be a string");
                                            vals[index] = reconstruct_index_mode!(span);
                                        }
                                    }
                                    "trim" => {
                                        match try_parse_trim_strategy(value, &mut errors) {
                                            Ok(v) => config.trim_strategy = v,
                                            Err(e) => {
                                                // try_parse_trim_strategy() already adds its own errors
                                                errors.push(e);
                                                vals[index] = reconstruct_trim_strategy!(span);
                                            }
                                        }
                                    }
                                    "show_empty" => {
                                        try_bool!(cols, vals, index, span, table_show_empty)
                                    }
                                    x => {
                                        invalid_key!(
                                            cols,
                                            vals,
                                            index,
                                            value.span().ok(),
                                            "$env.config.{key}.{x} is an unknown config setting"
                                        );
                                    }
                                }
                            }
                        } else {
                            invalid!(vals[index].span().ok(), "should be a record");
                            // Reconstruct
                            vals[index] = Value::record(
                                vec!["mode".into(), "index_mode".into(), "trim".into()],
                                vec![
                                    Value::string(config.table_mode.clone(), *span),
                                    reconstruct_index_mode!(span),
                                    reconstruct_trim_strategy!(span),
                                ],
                                *span,
                            )
                        }
                    }
                    "filesize" => {
                        if let Value::Record { cols, vals, span } = &mut vals[index] {
                            for index in (0..cols.len()).rev() {
                                let value = &vals[index];
                                let key2 = cols[index].as_str();
                                match key2 {
                                    "metric" => {
                                        try_bool!(cols, vals, index, span, filesize_metric)
                                    }
                                    "format" => {
                                        if let Ok(v) = value.as_string() {
                                            config.filesize_format = v.to_lowercase();
                                        } else {
                                            invalid!(Some(*span), "should be a string");
                                            // Reconstruct
                                            vals[index] = Value::string(
                                                config.filesize_format.clone(),
                                                *span,
                                            );
                                        }
                                    }
                                    x => {
                                        invalid_key!(
                                            cols,
                                            vals,
                                            index,
                                            value.span().ok(),
                                            "$env.config.{key}.{x} is an unknown config setting"
                                        );
                                    }
                                }
                            }
                        } else {
                            invalid!(vals[index].span().ok(), "should be a record");
                            // Reconstruct
                            vals[index] = Value::record(
                                vec!["metric".into(), "format".into()],
                                vec![
                                    Value::boolean(config.filesize_metric, *span),
                                    Value::string(config.filesize_format.clone(), *span),
                                ],
                                *span,
                            );
                        }
                    }
                    "explore" => {
                        if let Ok(map) = create_map(value) {
                            config.explore = map;
                        } else {
                            invalid!(vals[index].span().ok(), "should be a record");
                            // Reconstruct
                            vals[index] = Value::record_from_hashmap(&config.explore, *span);
                        }
                    }
                    // Misc. options
                    "color_config" => {
                        if let Ok(map) = create_map(value) {
                            config.color_config = map;
                        } else {
                            invalid!(vals[index].span().ok(), "should be a record");
                            // Reconstruct
                            vals[index] = Value::record_from_hashmap(&config.color_config, *span);
                        }
                    }
                    "use_grid_icons" => {
                        try_bool!(cols, vals, index, span, use_grid_icons);
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
                            invalid!(Some(*span), "should be a string");
                            // Reconstruct
                            vals[index] = Value::String {
                                val: match config.footer_mode {
                                    FooterMode::Auto => "auto".into(),
                                    FooterMode::Never => "never".into(),
                                    FooterMode::Always => "always".into(),
                                    FooterMode::RowCount(number) => number.to_string(),
                                },
                                span: *span,
                            };
                        }
                    }
                    "float_precision" => {
                        try_int!(cols, vals, index, span, float_precision);
                    }
                    "use_ansi_coloring" => {
                        try_bool!(cols, vals, index, span, use_ansi_coloring);
                    }
                    "edit_mode" => {
                        if let Ok(v) = value.as_string() {
                            config.edit_mode = v.to_lowercase();
                        } else {
                            invalid!(Some(*span), "should be a string");
                            // Reconstruct
                            vals[index] = Value::string(config.edit_mode.clone(), *span);
                        }
                    }
                    "log_level" => {
                        if let Ok(v) = value.as_string() {
                            config.log_level = v.to_lowercase();
                        } else {
                            invalid!(Some(*span), "should be a string");
                            // Reconstruct
                            vals[index] = Value::string(config.log_level.clone(), *span);
                        }
                    }
                    "menus" => match create_menus(value) {
                        Ok(map) => config.menus = map,
                        Err(e) => {
                            invalid!(Some(*span), "should be a valid list of menus");
                            errors.push(e);
                            // Reconstruct
                            vals[index] = Value::List {
                                vals: config
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
                                            Value::Record {
                                                cols: vec![
                                                    "name".into(),
                                                    "only_buffer_difference".into(),
                                                    "marker".into(),
                                                    "style".into(),
                                                    "type".into(),
                                                    "source".into(),
                                                ],
                                                vals: vec![
                                                    name.clone(),
                                                    only_buffer_difference.clone(),
                                                    marker.clone(),
                                                    style.clone(),
                                                    menu_type.clone(),
                                                    source.clone(),
                                                ],
                                                span: *span,
                                            }
                                        },
                                    )
                                    .collect(),
                                span: *span,
                            }
                        }
                    },
                    "keybindings" => match create_keybindings(value) {
                        Ok(keybindings) => config.keybindings = keybindings,
                        Err(e) => {
                            invalid!(Some(*span), "should be a valid keybindings list");
                            errors.push(e);
                            // Reconstruct
                            vals[index] = Value::List {
                                vals: config
                                    .keybindings
                                    .iter()
                                    .map(
                                        |ParsedKeybinding {
                                             modifier,
                                             keycode,
                                             mode,
                                             event,
                                         }| {
                                            Value::Record {
                                                cols: vec![
                                                    "modifier".into(),
                                                    "keycode".into(),
                                                    "mode".into(),
                                                    "event".into(),
                                                ],
                                                vals: vec![
                                                    modifier.clone(),
                                                    keycode.clone(),
                                                    mode.clone(),
                                                    event.clone(),
                                                ],
                                                span: *span,
                                            }
                                        },
                                    )
                                    .collect(),
                                span: *span,
                            }
                        }
                    },
                    "hooks" => match create_hooks(value) {
                        Ok(hooks) => config.hooks = hooks,
                        Err(e) => {
                            invalid!(Some(*span), "should be a valid hooks list");
                            errors.push(e);
                            // Reconstruct
                            let mut hook_cols = vec![];
                            let mut hook_vals = vec![];
                            if let Some(ref value) = config.hooks.pre_prompt {
                                hook_cols.push("pre_prompt".into());
                                hook_vals.push(value.clone());
                            }
                            if let Some(ref value) = config.hooks.pre_execution {
                                hook_cols.push("pre_execution".into());
                                hook_vals.push(value.clone());
                            }
                            if let Some(ref value) = config.hooks.env_change {
                                hook_cols.push("env_change".into());
                                hook_vals.push(value.clone());
                            }
                            if let Some(ref value) = config.hooks.display_output {
                                hook_cols.push("display_output".into());
                                hook_vals.push(value.clone());
                            }
                            vals.push(Value::Record {
                                cols: hook_cols,
                                vals: hook_vals,
                                span: *span,
                            });
                        }
                    },
                    "shell_integration" => {
                        try_bool!(cols, vals, index, span, shell_integration);
                    }
                    "buffer_editor" => {
                        if let Ok(v) = value.as_string() {
                            config.buffer_editor = v.to_lowercase();
                        } else {
                            invalid!(Some(*span), "should be a string");
                        }
                    }
                    "show_banner" => {
                        try_bool!(cols, vals, index, span, show_banner);
                    }
                    "render_right_prompt_on_last_line" => {
                        try_bool!(cols, vals, index, span, render_right_prompt_on_last_line);
                    }
                    "bracketed_paste" => {
                        try_bool!(cols, vals, index, span, bracketed_paste);
                    }
                    // Legacy config options (deprecated as of 2022-11-02)
                    // Legacy options do NOT reconstruct their values on error
                    "use_ls_colors" => {
                        legacy_options_used = true;
                        try_bool!(cols, vals, index, span, use_ls_colors);
                    }
                    "rm_always_trash" => {
                        legacy_options_used = true;
                        try_bool!(cols, vals, index, span, rm_always_trash);
                    }
                    "history_file_format" => {
                        legacy_options_used = true;
                        if let Ok(b) = value.as_string() {
                            let val_str = b.to_lowercase();
                            config.history_file_format = match val_str.as_ref() {
                                "sqlite" => HistoryFileFormat::Sqlite,
                                "plaintext" => HistoryFileFormat::PlainText,
                                _ => {
                                    invalid!(
                                        Some(*span),
                                        "unrecognized $env.config.{key} '{val_str}'"
                                    );
                                    HistoryFileFormat::PlainText
                                }
                            };
                        } else {
                            invalid!(Some(*span), "should be a string");
                        }
                    }
                    "sync_history_on_enter" => {
                        legacy_options_used = true;
                        try_bool!(cols, vals, index, span, sync_history_on_enter);
                    }
                    "max_history_size" => {
                        legacy_options_used = true;
                        try_int!(cols, vals, index, span, max_history_size);
                    }
                    "quick_completions" => {
                        legacy_options_used = true;
                        try_bool!(cols, vals, index, span, quick_completions);
                    }
                    "partial_completions" => {
                        legacy_options_used = true;
                        try_bool!(cols, vals, index, span, partial_completions);
                    }
                    "max_external_completion_results" => {
                        legacy_options_used = true;
                        try_int!(cols, vals, index, span, max_external_completion_results);
                    }
                    "completion_algorithm" => {
                        legacy_options_used = true;
                        if let Ok(v) = value.as_string() {
                            let val_str = v.to_lowercase();
                            config.completion_algorithm = match val_str.as_ref() {
                                // This should match the MatchAlgorithm enum in completions::completion_options
                                "prefix" => val_str,
                                "fuzzy" => val_str,
                                _ => {
                                    invalid!( Some(*span),
                                        "unrecognized $env.config.{key} '{val_str}'; expected either 'prefix' or 'fuzzy'"
                                    );
                                    val_str
                                }
                            };
                        } else {
                            invalid!(Some(*span), "should be a string");
                        }
                    }
                    "case_sensitive_completions" => {
                        legacy_options_used = true;
                        try_bool!(cols, vals, index, span, case_sensitive_completions);
                    }
                    "enable_external_completion" => {
                        legacy_options_used = true;
                        try_bool!(cols, vals, index, span, enable_external_completion);
                    }
                    "external_completer" => {
                        legacy_options_used = true;
                        if let Ok(v) = value.as_block() {
                            config.external_completer = Some(v)
                        }
                        // No error here because external completers are optional.
                        // Idea: maybe error if this is a non-block, non-null?
                    }
                    "table_mode" => {
                        legacy_options_used = true;
                        if let Ok(v) = value.as_string() {
                            config.table_mode = v;
                        } else {
                            invalid!(Some(*span), "should be a string");
                        }
                    }
                    "table_index_mode" => {
                        legacy_options_used = true;
                        if let Ok(b) = value.as_string() {
                            let val_str = b.to_lowercase();
                            match val_str.as_ref() {
                                "always" => config.table_index_mode = TableIndexMode::Always,
                                "never" => config.table_index_mode = TableIndexMode::Never,
                                "auto" => config.table_index_mode = TableIndexMode::Auto,
                                _ => {
                                    invalid!( Some(*span),
                                        "unrecognized $env.config.table_index_mode '{val_str}'; expected either 'never', 'always' or 'auto'"
                                    );
                                }
                            }
                        } else {
                            invalid!(Some(*span), "should be a string");
                        }
                    }
                    "table_trim" => {
                        legacy_options_used = true;
                        match try_parse_trim_strategy(value, &mut errors) {
                            Ok(v) => config.trim_strategy = v,
                            Err(e) => {
                                // try_parse_trim_strategy() already calls eprintln!() on error
                                cols.remove(index);
                                vals.remove(index);
                                errors.push(e);
                            }
                        }
                    }
                    "show_clickable_links_in_ls" => {
                        legacy_options_used = true;
                        try_bool!(cols, vals, index, span, show_clickable_links_in_ls);
                    }
                    "cd_with_abbreviations" => {
                        legacy_options_used = true;
                        try_bool!(cols, vals, index, span, cd_with_abbreviations);
                    }
                    "filesize_metric" => {
                        legacy_options_used = true;
                        try_bool!(cols, vals, index, span, filesize_metric);
                    }
                    "filesize_format" => {
                        legacy_options_used = true;
                        if let Ok(v) = value.as_string() {
                            config.filesize_format = v.to_lowercase();
                        } else {
                            invalid!(Some(*span), "should be a string");
                        }
                    }
                    "cursor_shape_vi_insert" => {
                        legacy_options_used = true;
                        if let Ok(b) = value.as_string() {
                            let val_str = b.to_lowercase();
                            config.cursor_shape_vi_insert = match val_str.as_ref() {
                                "block" => NuCursorShape::Block,
                                "underline" => NuCursorShape::UnderScore,
                                "line" => NuCursorShape::Line,
                                _ => {
                                    invalid!(
                                        Some(*span),
                                        "unrecognized $env.config.{key} '{val_str}'"
                                    );
                                    NuCursorShape::Line
                                }
                            };
                        } else {
                            invalid!(Some(*span), "should be a string");
                        }
                    }
                    "cursor_shape_vi_normal" => {
                        legacy_options_used = true;
                        if let Ok(b) = value.as_string() {
                            let val_str = b.to_lowercase();
                            config.cursor_shape_vi_normal = match val_str.as_ref() {
                                "block" => NuCursorShape::Block,
                                "underline" => NuCursorShape::UnderScore,
                                "line" => NuCursorShape::Line,
                                _ => {
                                    invalid!(
                                        Some(*span),
                                        "unrecognized $env.config.{key} '{val_str}'"
                                    );
                                    NuCursorShape::Line
                                }
                            };
                        } else {
                            invalid!(Some(*span), "should be a string");
                        }
                    }
                    "cursor_shape_emacs" => {
                        legacy_options_used = true;
                        if let Ok(b) = value.as_string() {
                            let val_str = b.to_lowercase();
                            config.cursor_shape_emacs = match val_str.as_ref() {
                                "block" => NuCursorShape::Block,
                                "underline" => NuCursorShape::UnderScore,
                                "line" => NuCursorShape::Line,
                                _ => {
                                    invalid!(
                                        Some(*span),
                                        "unrecognized $env.config.{key} '{val_str}'"
                                    );
                                    NuCursorShape::Line
                                }
                            };
                        } else {
                            invalid!(Some(*span), "should be a string");
                        }
                    }

                    // End legacy options
                    x => {
                        invalid_key!(
                            cols,
                            vals,
                            index,
                            value.span().ok(),
                            "$env.config.{x} is an unknown config setting"
                        );
                    }
                }
            }
        } else {
            return (
                config,
                Some(ShellError::GenericError(
                    "Error while applying config changes".into(),
                    "$env.config is not a record".into(),
                    self.span().ok(),
                    None,
                    vec![],
                )),
            );
        }

        if legacy_options_used {
            // This is a notification message, not an error.
            eprintln!(
                r#"The format of $env.config has recently changed, and several options have been grouped into sub-records. You may need to update your config.nu file.
Please consult https://www.nushell.sh/blog/2022-11-29-nushell-0.72.html for details. Support for the old format will be removed in an upcoming Nu release."#
            );
        }

        // Return the config and the vec of errors.
        (
            config,
            if !errors.is_empty() {
                // Because the config was iterated in reverse, these errors
                // need to be reversed, too.
                errors.reverse();
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
            value.span().ok(),
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
                value.span().ok(),
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
                        value.span().ok(),
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
                        value.span().ok(),
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
    let (cols, inner_vals) = value.as_record()?;
    let mut hm: HashMap<String, Value> = HashMap::new();

    for (k, v) in cols.iter().zip(inner_vals) {
        hm.insert(k.to_string(), v.clone());
    }

    Ok(hm)
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
                    "command_not_found" => hooks.command_not_found = Some(vals[idx].clone()),
                    x => {
                        return Err(ShellError::UnsupportedConfigValue(
                            "'pre_prompt', 'pre_execution', 'env_change', 'display_output', 'command_not_found'"
                                .to_string(),
                            x.to_string(),
                            *span,
                        ));
                    }
                }
            }

            Ok(hooks)
        }
        v => Err(ShellError::UnsupportedConfigValue(
            "record for 'hooks' config".into(),
            "non-record value".into(),
            v.span().unwrap_or_else(|_| Span::unknown()),
        )),
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
