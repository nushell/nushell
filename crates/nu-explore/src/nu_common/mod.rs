mod command;
mod lscolor;
mod string;
mod table;
mod value;

use std::sync::{atomic::AtomicBool, Arc};

use nu_color_config::TextStyle;
use nu_protocol::Value;

pub use nu_ansi_term::{Color as NuColor, Style as NuStyle};
pub use nu_protocol::{Config as NuConfig, Span as NuSpan};

pub type NuText = (String, TextStyle);
pub type CtrlC = Option<Arc<AtomicBool>>;

pub use command::{is_ignored_command, run_command_with_value, run_nu_command};
pub use lscolor::{create_lscolors, lscolorize};
pub use string::truncate_str;
pub use table::try_build_table;
pub use value::{collect_input, collect_pipeline, create_map, map_into_value, nu_str};

pub fn has_simple_value(data: &[Vec<Value>]) -> Option<&Value> {
    let has_single_value = data.len() == 1 && data[0].len() == 1;
    let is_complex_type = matches!(&data[0][0], Value::List { .. } | Value::Record { .. });
    if has_single_value && !is_complex_type {
        Some(&data[0][0])
    } else {
        None
    }
}
