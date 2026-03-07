use std::path::PathBuf;

use crate::{PolarsPlugin, cloud::build_cloud_options};
use nu_path::expand_path_with;
use nu_plugin::EngineInterface;
use nu_protocol::{ShellError, Span, Spanned};
use polars::{
    io::cloud::CloudOptions,
    prelude::{PlRefPath, SinkDestination, SinkTarget},
};

#[derive(Clone)]
pub(crate) struct Resource {
    pub(crate) path: PlRefPath,
    pub(crate) cloud_options: Option<CloudOptions>,
    pub(crate) span: Span,
}

impl std::fmt::Debug for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // We can't print out the cloud options as it might have
        // secrets in it.. So just print whether or not it was defined
        f.debug_struct("Resource")
            .field("path", &self.path)
            .field("has_cloud_options", &self.cloud_options.is_some())
            .field("span", &self.span)
            .finish()
    }
}

impl Resource {
    pub(crate) fn new(
        plugin: &PolarsPlugin,
        engine: &EngineInterface,
        spanned_path: &Spanned<String>,
    ) -> Result<Self, ShellError> {
        let path = PlRefPath::from(spanned_path.item.as_str());

        let (path, cloud_options): (PlRefPath, Option<CloudOptions>) = if path.has_scheme() {
            let options = build_cloud_options(plugin, &path)?;
            if options.is_none() {
                return Err(ShellError::GenericError {
                    error: format!("Could not determine a supported cloud type from path: {path}"),
                    msg: "".into(),
                    span: None,
                    help: None,
                    inner: vec![],
                });
            }
            (path, options)
        } else {
            let new_path = expand_path_with(&spanned_path.item, engine.get_current_dir()?, true);
            (
                PlRefPath::try_from_path(&new_path).map_err(|e| ShellError::GenericError {
                    error: format!("Could not resolve resource: {new_path:?} : {e}"),
                    msg: "".into(),
                    span: Some(spanned_path.span),
                    help: None,
                    inner: vec![],
                })?,
                None,
            )
        };

        Ok(Self {
            path,
            cloud_options,
            span: spanned_path.span,
        })
    }

    pub fn as_string(&self) -> String {
        self.path.to_string().to_owned()
    }

    pub fn as_path_buf(&self) -> PathBuf {
        let path: &std::path::Path = self.path.as_ref();
        path.to_owned()
    }
}

impl From<Resource> for SinkTarget {
    fn from(r: Resource) -> SinkTarget {
        SinkTarget::Path(r.path)
    }
}

impl From<Resource> for SinkDestination {
    fn from(val: Resource) -> Self {
        SinkDestination::File {
            target: SinkTarget::Path(val.path.clone()),
        }
    }
}
