mod command;
mod lscolor;
mod string;
mod table;
mod value;

use nu_color_config::TextStyle;
use nu_protocol::Value;

pub use nu_ansi_term::{Color as NuColor, Style as NuStyle};
pub use nu_protocol::{Config as NuConfig, Span as NuSpan};

pub type NuText = (String, TextStyle);

pub use command::run_command_with_value;
pub use lscolor::{create_lscolors, lscolorize};
pub use string::{string_width, truncate_str};
pub use table::try_build_table;
pub use value::{collect_input, collect_pipeline, create_map};

pub fn has_simple_value(data: &[Vec<Value>]) -> Option<&Value> {
    if data.len() == 1
        && data[0].len() == 1
        && !matches!(&data[0][0], Value::List { .. } | Value::Record { .. })
    {
        Some(&data[0][0])
    } else {
        None
    }
}
