//! Color scheme and style helpers for the regex explorer UI.

use ratatui::style::{Color, Modifier, Style};

// UI colors - using standard ANSI colors that adapt to terminal theme
pub const ACCENT: Color = Color::Cyan;
pub const SUCCESS: Color = Color::Green;
pub const ERROR: Color = Color::Red;
pub const WARNING: Color = Color::Yellow;

// Text colors - using standard colors for terminal compatibility
pub const BG_DARK: Color = Color::Black;
pub const FG_PRIMARY: Color = Color::Reset; // Uses terminal default
pub const FG_MUTED: Color = Color::DarkGray;
pub const FG_LIGHT: Color = Color::Gray; // Brighter than muted, for modal descriptions

// Highlight colors for regex matches as an array for easy iteration
pub const HIGHLIGHT_COLORS: &[Color] = &[
    Color::LightBlue,
    Color::LightGreen,
    Color::LightRed,
    Color::LightYellow,
    Color::Blue,
    Color::Green,
    Color::Red,
    Color::Yellow,
    Color::Magenta,
];

/// Returns the appropriate foreground color for a given highlight background.
/// Uses white for darker backgrounds, black for lighter ones.
pub const fn highlight_fg(color: Color) -> Color {
    match color {
        Color::Red | Color::Magenta | Color::Blue => Color::White,
        _ => Color::Black,
    }
}

/// Creates a highlight style for the given group index (cycles through colors).
pub fn highlight_style(group: usize) -> Style {
    let bg = HIGHLIGHT_COLORS[group % HIGHLIGHT_COLORS.len()];
    Style::new().bg(bg).fg(highlight_fg(bg))
}

/// Style presets for common UI elements
pub mod styles {
    use super::*;

    /// Style for focused/active elements
    pub fn focused() -> Style {
        Style::new().fg(FG_PRIMARY).bold()
    }

    /// Style for unfocused/inactive elements
    pub fn unfocused() -> Style {
        Style::new().fg(FG_MUTED)
    }

    /// Style for the focus indicator ("> ")
    pub fn focus_indicator() -> Style {
        Style::new().fg(FG_PRIMARY)
    }

    /// Style for muted separator text
    pub fn separator() -> Style {
        Style::new().fg(FG_MUTED)
    }

    /// Style for status badges (the brackets)
    pub fn status_bracket() -> Style {
        Style::new().fg(FG_MUTED)
    }

    /// Style for success status text
    pub fn status_success() -> Style {
        Style::new().fg(SUCCESS)
    }

    /// Style for error status text
    pub fn status_error() -> Style {
        Style::new().fg(ERROR)
    }

    /// Style for warning status text
    pub fn status_warning() -> Style {
        Style::new().fg(WARNING)
    }

    /// Style for category headers in quick reference
    pub fn category_header() -> Style {
        Style::new()
            .fg(ACCENT)
            .bold()
            .add_modifier(Modifier::UNDERLINED)
    }

    /// Style for selected item (inverted colors)
    pub fn selected() -> Style {
        Style::new().fg(BG_DARK).bg(ACCENT)
    }

    /// Style for selected item text (bold variant)
    pub fn selected_bold() -> Style {
        selected().bold()
    }

    /// Border style for focused panels
    pub fn border_focused() -> Style {
        Style::new().fg(ACCENT)
    }

    /// Border style for focused panels with error state
    pub fn border_error() -> Style {
        Style::new().fg(ERROR)
    }

    /// Border style for unfocused panels
    pub fn border_unfocused() -> Style {
        Style::new().fg(FG_MUTED)
    }

    /// Style for modal help description text (brighter than separator)
    pub fn modal_desc() -> Style {
        Style::new().fg(FG_LIGHT)
    }
}
