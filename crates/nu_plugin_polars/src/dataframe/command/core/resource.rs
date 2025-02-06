use std::path::{Component, Path, PathBuf};

use crate::{cloud::build_cloud_options, PolarsPlugin};
use nu_path::expand_path_with;
use nu_protocol::{ShellError, Span, Spanned};
use polars_io::cloud::CloudOptions;
use url::Url;

pub(crate) struct Resource {
    pub(crate) path: String,
    pub(crate) extension: Option<String>,
    pub(crate) cloud_options: Option<CloudOptions>,
    pub(crate) span: Span,
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
    pub(crate) fn new(
        plugin: &PolarsPlugin,
        engine: &nu_plugin::EngineInterface,
        spanned_path: &Spanned<String>,
    ) -> Result<Self, ShellError> {
        let mut path = spanned_path.item.clone();

        let (path_buf, cloud_options) = match path.parse::<Url>() {
            Ok(url) if !is_windows_path(&path) => {
                let cloud_options =
                    build_cloud_options(plugin, &url)?.ok_or(ShellError::GenericError {
                        error: format!(
                            "Could not determine a supported cloud type from url: {url}"
                        ),
                        msg: "".into(),
                        span: None,
                        help: None,
                        inner: vec![],
                    })?;
                let p: PathBuf = url.path().into();
                (p, Some(cloud_options))
            }
            _ => {
                let new_path = expand_path_with(path, engine.get_current_dir()?, true);
                path = new_path.to_string_lossy().to_string();
                (new_path, None)
            }
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

// This is needed because Url parses windows paths as
// valid URLs.
fn is_windows_path(path: &str) -> bool {
    // Only window spath will
    if path.contains('\\') {
        return true;
    }

    let path = Path::new(path);
    match path.components().next() {
        // This will only occur if their is a drive prefix
        Some(Component::Prefix(_)) => true,
        _ => false,
    }
}
