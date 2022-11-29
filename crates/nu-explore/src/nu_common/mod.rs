mod command;
mod table;
mod value;

use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
};

use nu_table::TextStyle;

pub use nu_ansi_term::{Color as NuColor, Style as NuStyle};
pub use nu_protocol::{Config as NuConfig, Span as NuSpan};

pub type NuText = (String, TextStyle);
pub type CtrlC = Option<Arc<AtomicBool>>;
pub type NuStyleTable = HashMap<String, NuStyle>;

pub use command::run_nu_command;
pub use table::try_build_table;
pub use value::{collect_input, collect_pipeline};
