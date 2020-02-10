use futures::executor::block_on;
use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, ReturnValue, Signature, SyntaxShape, Value};

use crate::post::post_helper;
use crate::Post;

impl Plugin for Post {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("post")
            .desc("Post content to a url and retrieve data as a table if possible.")
            .required("path", SyntaxShape::Any, "the URL to post to")
            .required("body", SyntaxShape::Any, "the contents of the post body")
            .named("user", SyntaxShape::Any, "the username when authenticating")
            .named(
                "password",
                SyntaxShape::Any,
                "the password when authenticating",
            )
            .named(
                "content-type",
                SyntaxShape::Any,
                "the MIME type of content to post",
            )
            .named(
                "content-length",
                SyntaxShape::Any,
                "the length of the content being posted",
            )
            .switch("raw", "return values as a string instead of a table")
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        self.setup(call_info)?;
        Ok(vec![])
    }

    fn filter(&mut self, row: Value) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![block_on(post_helper(
            &self.path.clone().ok_or_else(|| {
                ShellError::labeled_error("expected a 'path'", "expected a 'path'", &self.tag)
            })?,
            self.has_raw,
            &self.body.clone().ok_or_else(|| {
                ShellError::labeled_error("expected a 'body'", "expected a 'body'", &self.tag)
            })?,
            self.user.clone(),
            self.password.clone(),
            &self.headers.clone(),
            row,
        ))])
    }
}
