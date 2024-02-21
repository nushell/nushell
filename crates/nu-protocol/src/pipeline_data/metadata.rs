use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct PipelineMetadata {
    pub data_source: DataSource,
}

#[derive(Debug, Clone)]
pub enum DataSource {
    Ls,
    HtmlThemes,
    FilePath(PathBuf),
}
