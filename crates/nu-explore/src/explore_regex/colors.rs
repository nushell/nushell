//! Color scheme for the regex explorer UI.

use ratatui::style::Color;

// UI colors - using standard ANSI colors that adapt to terminal theme
pub const ACCENT: Color = Color::Cyan;
pub const SUCCESS: Color = Color::Green;
pub const ERROR: Color = Color::Red;
pub const WARNING: Color = Color::Yellow;

// Text colors - using standard colors for terminal compatibility
pub const BG_DARK: Color = Color::Black;
pub const FG_PRIMARY: Color = Color::Reset; // Uses terminal default
pub const FG_MUTED: Color = Color::DarkGray;

// Highlight colors for regex matches (original colors)
pub const HIGHLIGHT_1: Color = Color::LightBlue;
pub const HIGHLIGHT_2: Color = Color::LightGreen;
pub const HIGHLIGHT_3: Color = Color::LightRed;
pub const HIGHLIGHT_4: Color = Color::LightYellow;
pub const HIGHLIGHT_5: Color = Color::Blue;
pub const HIGHLIGHT_6: Color = Color::Green;
pub const HIGHLIGHT_7: Color = Color::Red;
pub const HIGHLIGHT_8: Color = Color::Yellow;
pub const HIGHLIGHT_9: Color = Color::Magenta;
