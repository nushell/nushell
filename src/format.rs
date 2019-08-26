crate mod table;
crate mod vtable;

use crate::prelude::*;

crate use table::TableView;
crate use vtable::VTableView;

crate trait RenderView {
    fn render_view(&self, host: &mut dyn Host) -> Result<(), ShellError>;
}

crate fn print_view(view: &impl RenderView, host: &mut dyn Host) -> Result<(), ShellError> {
    view.render_view(host)
}
