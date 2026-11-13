//! Conversions between nushell styles and ratatui styles, shared with `nu-tui`.
//!
//! `explore` and the `tui` commands both draw with ratatui, so the mapping
//! from `$env.config.color_config`, `LS_COLORS`, and ANSI escapes lives here
//! once.

pub use crate::explore::nu_common::{create_lscolors, get_path_style};
pub use crate::explore::views::colored_text_widget::style_to_tui as ansi_style_to_tui;
pub use crate::explore::views::util::{
    nu_ansi_color_to_tui_color, nu_style_to_tui, text_style_to_tui_style,
};
