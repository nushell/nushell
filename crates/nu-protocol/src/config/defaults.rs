//! Built-in defaults for `$env.config` that must be visible without loading
//! `default_config.nu` (e.g. under `nu -n --no-std-lib`).
//!
//! Includes:
//! - `color_config` theme (formerly only in `default_config.nu`)
//! - default Reedline menus
//! - Nushell-owned menu keybindings (not reedline's base emacs/vi maps)

use super::reedline::{ParsedKeybinding, ParsedMenu};
use crate::{Record, Span, Value, record};
use std::collections::HashMap;

fn span() -> Span {
    Span::unknown()
}

fn str(s: &str) -> Value {
    Value::string(s, span())
}

fn bool(b: bool) -> Value {
    Value::bool(b, span())
}

fn int(i: i64) -> Value {
    Value::int(i, span())
}

fn rec(r: Record) -> Value {
    Value::record(r, span())
}

/// Default syntax/UI color theme, previously embedded in `default_config.nu`.
pub fn default_color_config() -> HashMap<String, Value> {
    let entries: &[(&str, Value)] = &[
        ("separator", str("default")),
        (
            "leading_trailing_space_bg",
            rec(record! { "attr" => str("n") }),
        ),
        ("header", str("green_bold")),
        ("empty", str("blue")),
        ("bool", str("light_cyan")),
        ("int", str("default")),
        ("filesize", str("cyan")),
        ("duration", str("default")),
        ("datetime", str("purple")),
        ("range", str("default")),
        ("float", str("default")),
        ("string", str("default")),
        ("nothing", str("default")),
        ("binary", str("default")),
        ("binary_null_char", str("grey42")),
        ("binary_printable", str("cyan_bold")),
        ("binary_whitespace", str("green_bold")),
        ("binary_ascii_other", str("purple_bold")),
        ("binary_non_ascii", str("yellow_bold")),
        ("cell-path", str("default")),
        ("row_index", str("green_bold")),
        ("record", str("default")),
        ("list", str("default")),
        ("closure", str("green_bold")),
        ("glob", str("cyan_bold")),
        ("semver", str("cyan_bold")),
        ("semver-range", str("cyan_bold")),
        ("block", str("default")),
        ("hints", str("dark_gray")),
        (
            "search_result",
            rec(record! {
                "bg" => str("red"),
                "fg" => str("default"),
            }),
        ),
        ("shape_binary", str("purple_bold")),
        ("shape_block", str("blue_bold")),
        ("shape_bool", str("light_cyan")),
        ("shape_closure", str("green_bold")),
        ("shape_custom", str("green")),
        ("shape_datetime", str("cyan_bold")),
        ("shape_directory", str("cyan")),
        ("shape_external", str("cyan")),
        ("shape_externalarg", str("green_bold")),
        ("shape_external_resolved", str("light_yellow_bold")),
        ("shape_filepath", str("cyan")),
        ("shape_flag", str("blue_bold")),
        ("shape_float", str("purple_bold")),
        ("shape_glob_interpolation", str("cyan_bold")),
        ("shape_globpattern", str("cyan_bold")),
        ("shape_int", str("purple_bold")),
        ("shape_internalcall", str("cyan_bold")),
        ("shape_keyword", str("cyan_bold")),
        ("shape_list", str("cyan_bold")),
        ("shape_literal", str("blue")),
        ("shape_match_pattern", str("green")),
        // String form so matching-brackets merge works with `lookup_ansi_color_style`
        ("shape_matching_brackets", str("default_underline")),
        ("shape_nothing", str("light_cyan")),
        ("shape_operator", str("yellow")),
        ("shape_pipe", str("purple_bold")),
        ("shape_range", str("yellow_bold")),
        ("shape_record", str("cyan_bold")),
        ("shape_redirection", str("purple_bold")),
        ("shape_signature", str("green_bold")),
        ("shape_string", str("green")),
        ("shape_string_interpolation", str("cyan_bold")),
        ("shape_table", str("blue_bold")),
        ("shape_variable", str("purple")),
        ("shape_vardecl", str("purple")),
        ("shape_raw_string", str("light_purple")),
        (
            "shape_garbage",
            rec(record! {
                "fg" => str("default"),
                "bg" => str("red"),
                "attr" => str("b"),
            }),
        ),
    ];

    entries
        .iter()
        .map(|(k, v)| ((*k).to_string(), v.clone()))
        .collect()
}

fn menu_style_basic() -> Value {
    rec(record! {
        "text" => str("green"),
        "selected_text" => str("green_reverse"),
        "description_text" => str("yellow"),
    })
}

/// Default Reedline menus applied when none are configured by the user.
pub fn default_menus() -> Vec<ParsedMenu> {
    vec![
        ParsedMenu {
            name: str("completion_menu"),
            marker: str("| "),
            only_buffer_difference: Some(bool(false)),
            input_mode: None,
            output_mode: None,
            style: menu_style_basic(),
            r#type: rec(record! {
                "layout" => str("columnar"),
                "columns" => int(4),
                "col_width" => int(20),
                "col_padding" => int(2),
                "tab_traversal" => str("horizontal"),
            }),
            source: None,
        },
        ParsedMenu {
            name: str("ide_completion_menu"),
            marker: str("| "),
            only_buffer_difference: Some(bool(false)),
            input_mode: None,
            output_mode: None,
            style: rec(record! {
                "text" => str("green"),
                "selected_text" => rec(record! { "attr" => str("r") }),
                "description_text" => str("yellow"),
                "match_text" => rec(record! { "attr" => str("u") }),
                "selected_match_text" => rec(record! { "attr" => str("ur") }),
            }),
            r#type: rec(record! {
                "layout" => str("ide"),
                "min_completion_width" => int(0),
                "max_completion_width" => int(50),
                "max_completion_height" => int(10),
                "padding" => int(0),
                "border" => bool(true),
                "cursor_offset" => int(0),
                "description_mode" => str("prefer_right"),
                "min_description_width" => int(15),
                "max_description_width" => int(50),
                "max_description_height" => int(10),
                "description_offset" => int(1),
                "correct_cursor_pos" => bool(false),
            }),
            source: None,
        },
        ParsedMenu {
            name: str("history_menu"),
            marker: str("? "),
            only_buffer_difference: Some(bool(true)),
            input_mode: None,
            output_mode: None,
            style: menu_style_basic(),
            r#type: rec(record! {
                "layout" => str("list"),
                "page_size" => int(10),
            }),
            source: None,
        },
        ParsedMenu {
            name: str("help_menu"),
            marker: str("? "),
            only_buffer_difference: Some(bool(true)),
            input_mode: None,
            output_mode: None,
            style: menu_style_basic(),
            r#type: rec(record! {
                "layout" => str("description"),
                "columns" => int(4),
                "col_width" => int(20),
                "col_padding" => int(2),
                "selection_rows" => int(4),
                "description_rows" => int(15),
            }),
            source: None,
        },
    ]
}

/// Every edit mode the Nushell-owned keybindings below apply to.
///
/// The helix tables are named unconditionally, like `cursor_shape.helix_*`: a
/// build without the `helix` feature has no table to bind them into and skips
/// them when the keybindings are applied. `helix_select` is absent because
/// reedline shares one table between helix normal and select mode, so
/// `helix_normal` already covers both.
fn all_modes() -> Value {
    Value::list(
        vec![
            str("emacs"),
            str("vi_normal"),
            str("vi_insert"),
            str("helix_normal"),
            str("helix_insert"),
        ],
        span(),
    )
}

fn keybinding(name: &str, modifier: &str, keycode: &str, event: Value) -> ParsedKeybinding {
    ParsedKeybinding {
        name: Some(str(name)),
        modifier: str(modifier),
        keycode: str(keycode),
        event,
        mode: all_modes(),
    }
}

fn until(events: Vec<Value>) -> Value {
    rec(record! {
        "until" => Value::list(events, span()),
    })
}

fn send(name: &str) -> Value {
    rec(record! {
        "send" => str(name),
    })
}

fn send_menu(menu_name: &str) -> Value {
    rec(record! {
        "send" => str("menu"),
        "name" => str(menu_name),
    })
}

fn edit(name: &str) -> Value {
    rec(record! {
        "edit" => str(name),
    })
}

/// Default `$env.config.explore` values matching `ExploreConfig::default()`
/// colors in `nu-explore` (inherit-from-terminal styles stay unset).
pub fn default_explore() -> HashMap<String, Value> {
    [
        ("selected_cell", rec(record! { "bg" => str("light_blue") })),
        (
            "highlight",
            rec(record! {
                "fg" => str("black"),
                "bg" => str("yellow"),
            }),
        ),
        (
            "status",
            rec(record! {
                "success" => rec(record! {
                    "fg" => str("black"),
                    "bg" => str("green"),
                }),
                "error" => rec(record! {
                    "fg" => str("white"),
                    "bg" => str("red"),
                }),
            }),
        ),
        (
            "try",
            rec(record! {
                "reactive" => bool(false),
            }),
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect()
}

/// Nushell-owned menu keybindings (Tab completion, history menu, help, etc.).
///
/// Reedline's base emacs/vi maps are **not** included; those remain library
/// defaults applied underneath in `create_keybindings`.
pub fn default_keybindings() -> Vec<ParsedKeybinding> {
    vec![
        keybinding(
            "completion_menu",
            "none",
            "tab",
            until(vec![
                send_menu("completion_menu"),
                send("menunext"),
                edit("complete"),
            ]),
        ),
        keybinding(
            "ide_completion_menu",
            "control",
            "space",
            until(vec![
                send_menu("ide_completion_menu"),
                send("menunext"),
                edit("complete"),
            ]),
        ),
        keybinding(
            "completion_previous",
            "shift",
            "backtab",
            send("menuprevious"),
        ),
        keybinding(
            "history_menu",
            "control",
            "char_r",
            send_menu("history_menu"),
        ),
        keybinding("next_page_menu", "control", "char_x", send("menupagenext")),
        keybinding(
            "undo_or_previous_page_menu",
            "control",
            "char_z",
            until(vec![send("menupageprevious"), edit("undo")]),
        ),
        keybinding("help_menu", "none", "f1", send_menu("help_menu")),
        keybinding("search_history", "control", "char_q", send("searchhistory")),
    ]
}
