#![doc = include_str!("../README.md")]
mod color_config;
mod matching_brackets_style;
mod nu_style;
mod shape_color;
mod style_computer;
mod text_style;

pub use color_config::*;
pub use matching_brackets_style::*;
pub use nu_style::*;
pub use shape_color::*;
pub use style_computer::*;
pub use text_style::*;
