use std::path::PathBuf;

use nu_utils::strings::SharedString;
use serde::{Deserialize, Serialize};

/// Metadata that is valid for the whole [`PipelineData`](crate::PipelineData)
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PipelineMetadata {
    pub data_source: DataSource,
    pub content_type: Option<SharedString>,
}

impl PipelineMetadata {
    pub fn with_data_source(self, data_source: DataSource) -> Self {
        Self {
            data_source,
            ..self
        }
    }

    pub fn with_content_type(self, content_type: impl Into<SharedString>) -> Self {
        Self {
            content_type: Some(content_type.into()),
            ..self
        }
    }

    pub fn remove_content_type(self) -> Self {
        Self {
            content_type: None,
            ..self
        }
    }
}

/// Describes where the particular [`PipelineMetadata`] originates.
///
/// This can either be a particular family of commands (useful so downstream commands can adjust
/// the presentation e.g. `Ls`) or the opened file to protect against overwrite-attempts properly.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum DataSource {
    Ls,
    HtmlThemes,
    FilePath(Box<PathBuf>),
    #[default]
    None,
}
