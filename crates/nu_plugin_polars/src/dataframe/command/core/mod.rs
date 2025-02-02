mod cache;
mod columns;
mod fetch;
mod open;
mod profile;
mod save;
mod schema;
mod shape;
mod summary;
mod to_df;
mod to_lazy;
mod to_nu;
mod to_repr;

use std::path::PathBuf;

use crate::{cloud::build_cloud_options, PolarsPlugin};
use nu_plugin::PluginCommand;
use nu_protocol::{ShellError, Span, Spanned};
use polars_io::cloud::CloudOptions;

pub use self::open::OpenDataFrame;
use fetch::LazyFetch;
use nu_path::expand_path_with;
pub use schema::SchemaCmd;
pub use shape::ShapeDF;
pub use summary::Summary;
pub use to_df::ToDataFrame;
pub use to_lazy::ToLazyFrame;
pub use to_nu::ToNu;
pub use to_repr::ToRepr;
use url::Url;

pub(crate) struct Resource {
    path: String,
    extension: Option<String>,
    cloud_options: Option<CloudOptions>,
    span: Span,
}

impl std::fmt::Debug for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // We can't print out the cloud options as it might have
        // secrets in it.. So just print whether or not it was defined
        f.debug_struct("Resource")
            .field("path", &self.path)
            .field("extension", &self.extension)
            .field("has_cloud_options", &self.cloud_options.is_some())
            .field("span", &self.span)
            .finish()
    }
}

impl Resource {
    fn new(
        plugin: &PolarsPlugin,
        engine: &nu_plugin::EngineInterface,
        spanned_path: &Spanned<String>,
    ) -> Result<Self, ShellError> {
        let mut path = spanned_path.item.clone();
        let (path_buf, cloud_options) = if let Ok(url) = path.parse::<Url>() {
            let cloud_options =
                build_cloud_options(plugin, &url)?.ok_or(ShellError::GenericError {
                    error: format!("Could not determine a supported cloud type from url: {url}"),
                    msg: "".into(),
                    span: None,
                    help: None,
                    inner: vec![],
                })?;
            let p: PathBuf = url.path().into();
            (p, Some(cloud_options))
        } else {
            let new_path = expand_path_with(path, engine.get_current_dir()?, true);
            path = new_path.to_string_lossy().to_string();
            (new_path, None)
        };
        let extension = path_buf
            .extension()
            .and_then(|s| s.to_str().map(|s| s.to_string()));
        Ok(Self {
            path,
            extension,
            cloud_options,
            span: spanned_path.span,
        })
    }
}
pub(crate) fn core_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(columns::ColumnsDF),
        Box::new(cache::LazyCache),
        Box::new(LazyFetch),
        Box::new(OpenDataFrame),
        Box::new(profile::ProfileDF),
        Box::new(Summary),
        Box::new(ShapeDF),
        Box::new(SchemaCmd),
        Box::new(ToNu),
        Box::new(ToDataFrame),
        Box::new(save::SaveDF),
        Box::new(ToLazyFrame),
        Box::new(ToRepr),
    ]
}
