use super::ReconstructVal;
use crate::{Span, Value};
use byte_unit::Unit;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    str::FromStr,
};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum FilesizeFormat {
    #[default]
    Auto,
    Unit(Unit),
}

impl Display for FilesizeFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilesizeFormat::Auto => write!(f, "auto"),
            FilesizeFormat::Unit(unit) => write!(f, "{unit}"),
        }
    }
}

impl FromStr for FilesizeFormat {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Err("expected either 'auto', 'B', 'KB', 'KiB', 'MB', 'MiB', 'GB', 'GiB', 'TB', 'TiB', 'PB', 'PiB', 'EB', 'EiB', 'b', 'Kb', 'Kib', 'Mb', 'Mib', 'Gb', 'Gib', 'Tb', 'Tib', 'Pb', 'Pib', 'Eb', or 'Eib'")
        } else if s.eq_ignore_ascii_case("auto") {
            Ok(FilesizeFormat::Auto)
        } else {
            Unit::parse_str(s, false, true).map(FilesizeFormat::Unit).map_err(|_| {
                "expected either 'auto', 'B', 'KB', 'KiB', 'MB', 'MiB', 'GB', 'GiB', 'TB', 'TiB', 'PB', 'PiB', 'EB', 'EiB', 'b', 'Kb', 'Kib', 'Mb', 'Mib', 'Gb', 'Gib', 'Tb', 'Tib', 'Pb', 'Pib', 'Eb', or 'Eib'"
            })
        }
    }
}

impl ReconstructVal for FilesizeFormat {
    fn reconstruct_value(&self, span: Span) -> Value {
        Value::string(self.to_string(), span)
    }
}
