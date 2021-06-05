use nu_source::Tagged;
use serde::{self, Deserialize};
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct CdArgs {
    pub path: Option<Tagged<PathBuf>>,
}

#[derive(Deserialize)]
pub struct CopyArgs {
    pub src: Tagged<PathBuf>,
    pub dst: Tagged<PathBuf>,
    pub recursive: bool,
}

#[derive(Deserialize)]
pub struct LsArgs {
    pub path: Option<Tagged<PathBuf>>,
    pub all: bool,
    pub long: bool,
    #[serde(rename = "short-names")]
    pub short_names: bool,
    #[serde(rename = "du")]
    pub du: bool,
}

#[derive(Deserialize)]
pub struct MvArgs {
    pub src: Tagged<PathBuf>,
    pub dst: Tagged<PathBuf>,
}

#[derive(Deserialize)]
pub struct MkdirArgs {
    pub rest: Vec<Tagged<PathBuf>>,
    #[serde(rename = "show-created-paths")]
    pub show_created_paths: bool,
}

#[derive(Deserialize)]
pub struct RemoveArgs {
    pub rest: Vec<Tagged<PathBuf>>,
    pub recursive: bool,
    #[allow(unused)]
    pub trash: bool,
    #[allow(unused)]
    pub permanent: bool,
    pub force: bool,
}
