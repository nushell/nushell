use futures::executor::block_on;
use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, ReturnValue, Signature, SyntaxShape};

use crate::handler;
use crate::handler::s3_helper;

impl Plugin for handler::Handler {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("s3")
            .usage("Load S3 resource into a cell, convert to table if possible (avoid by appending '--raw' or '-R')")
            .required(
                "RESOURCE",
                SyntaxShape::String,
                "the RESOURCE to fetch the contents from",
            )
            .named(
                "endpoint",
                SyntaxShape::Any,
                "the endpoint info for the S3 resource, i.g., s3.ap-northeast-1.amazonaws.com or 10.1.1.1",
                Some('e'),
            )
            .named(
                "access-key",
                SyntaxShape::Any,
                "the access key when authenticating",
                Some('a'),
            )
            .named(
                "secret-key",
                SyntaxShape::Any,
                "the secret key when authenticating",
                Some('s'),
            )
            .named(
                "region",
                SyntaxShape::Any,
                "the region of the resource, default will use us-east-1",
                Some('r'),
            )
            .switch("raw", "fetch contents as text rather than a table", Some('R'))
            .filter())
    }

    fn begin_filter(&mut self, callinfo: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        self.setup(callinfo)?;
        Ok(vec![block_on(s3_helper(
            &self.resource.clone().ok_or_else(|| {
                ShellError::labeled_error(
                    "internal error: resource not set",
                    "resource not set",
                    &self.tag,
                )
            })?,
            self.has_raw,
            &self.config,
        ))])
    }
}
