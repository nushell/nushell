use std::path::PathBuf;

/// Metadata that is valid for the whole [`PipelineData`](crate::PipelineData)
#[derive(Debug, Clone)]
pub struct PipelineMetadata {
    pub data_source: DataSource,
}

/// Describes where the particular [`PipelineMetadata`] originates.
///
/// This can either be a particular family of commands (useful so downstream commands can adjust
/// the presentation e.g. `Ls`) or the opened file to protect against overwrite-attempts properly.
#[derive(Debug, Clone)]
pub enum DataSource {
    Ls,
    HtmlThemes,
    FilePath(PathBuf),
}
