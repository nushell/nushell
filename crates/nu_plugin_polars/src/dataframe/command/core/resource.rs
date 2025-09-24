use std::path::PathBuf;

use crate::{PolarsPlugin, cloud::build_cloud_options};
use nu_protocol::{ShellError, Span, Spanned};
use polars::{
    io::cloud::CloudOptions,
    prelude::{PlPath, SinkTarget},
};

#[derive(Clone)]
pub(crate) struct Resource {
    pub(crate) path: PlPath,
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
        spanned_path: &Spanned<String>,
    ) -> Result<Self, ShellError> {
        let path = PlPath::from_str(&spanned_path.item);

        let cloud_options: Option<CloudOptions> = if path.is_cloud_url() {
            let options = build_cloud_options(plugin, &path)?;
            if options.is_none() {
                return Err(ShellError::GenericError {
                    error: format!(
                        "Could not determine a supported cloud type from path: {}",
                        path.to_str()
                    ),
                    msg: "".into(),
                    span: None,
                    help: None,
                    inner: vec![],
                });
            }
            options
        } else {
            None
        };

        Ok(Self {
            path,
            cloud_options,
            span: spanned_path.span,
        })
    }

    pub fn as_string(&self) -> String {
        self.path.to_str().to_owned()
    }
}
impl TryInto<PathBuf> for Resource {
    type Error = ShellError;

    fn try_into(self) -> Result<PathBuf, Self::Error> {
        let path_str = self.path.to_str().to_owned();
        self.path
            .into_local_path()
            .ok_or_else(|| ShellError::GenericError {
                error: format!("Could not convert path to local path: {path_str}",),
                msg: "".into(),
                span: Some(self.span),
                help: None,
                inner: vec![],
            })
            .map(|p| (*p).into())
    }
}

impl From<Resource> for SinkTarget {
    fn from(r: Resource) -> SinkTarget {
        SinkTarget::Path(r.path)
    }
}
