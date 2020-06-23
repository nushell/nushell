use crate::prelude::*;
use nu_errors::ShellError;

pub(crate) trait RenderView {
    fn render_view(&self, host: &mut dyn Host) -> Result<(), ShellError>;
}
