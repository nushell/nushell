use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Metadata that is valid for the whole [`PipelineData`](crate::PipelineData)
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PipelineMetadata {
    pub data_source: DataSource,
    pub content_type: Option<String>,
}

impl PipelineMetadata {
    pub fn with_data_source(self, data_source: DataSource) -> Self {
        Self {
            data_source,
            ..self
        }
    }

    pub fn with_content_type(self, content_type: Option<String>) -> Self {
        Self {
            content_type,
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
    // TODO: get rid of this by using `file://` protocol
    FilePath(PathBuf),
    // maybe use a more strict URI type here, in our dependency graph we would already have
    // `fluent_uri::Uri`
    Uri(String),
    #[default]
    None,
}
