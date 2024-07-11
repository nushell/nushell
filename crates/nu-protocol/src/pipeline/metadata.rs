use std::path::PathBuf;

/// Metadata that is valid for the whole [`PipelineData`](crate::PipelineData)
#[derive(Debug, Default, Clone)]
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
#[derive(Debug, Default, Clone)]
pub enum DataSource {
    Ls,
    HtmlThemes,
    FilePath(PathBuf),
    #[default]
    None,
}
