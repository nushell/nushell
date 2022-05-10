use nu_protocol::{ShellError, Span};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, path::PathBuf};

pub mod db;
pub mod db_column;
pub mod db_constraint;
pub mod db_foreignkey;
pub mod db_index;
pub mod db_row;
pub mod db_schema;
pub mod db_table;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ConnectionDb {
    Path(PathBuf),
}

impl Display for ConnectionDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Path(path) => write!(f, "{}", path.to_str().unwrap_or("")),
        }
    }
}

impl ConnectionDb {
    pub fn as_path(&self, _span: Span) -> Result<&PathBuf, ShellError> {
        match self {
            Self::Path(path) => Ok(path),
        }
    }
}
