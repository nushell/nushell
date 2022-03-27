use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, Signature, Value};

use crate::tree::TreeView;
use crate::TreeViewer;

impl Plugin for TreeViewer {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("tree").usage("View the contents of the pipeline as a tree."))
    }

    fn sink(&mut self, _call_info: CallInfo, input: Vec<Value>) {
        for i in &input {
            let view = TreeView::from_value(i);
            let _ = view.render_view();
        }
    }
}
