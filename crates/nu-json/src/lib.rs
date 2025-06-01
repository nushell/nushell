#![doc = include_str!("../README.md")]
pub use self::de::{
    Deserializer, StreamDeserializer, from_iter, from_reader, from_slice, from_str,
};
pub use self::error::{Error, ErrorCode, Result};
pub use self::ser::{
    Serializer, to_string, to_string_raw, to_string_with_indent, to_string_with_tab_indentation,
    to_vec, to_writer,
};
pub use self::value::{Map, Value, from_value, to_value};

pub mod builder;
pub mod de;
pub mod error;
pub mod ser;
mod util;
pub mod value;
