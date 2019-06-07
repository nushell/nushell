crate mod entries;
crate mod generic;
crate mod list;
crate mod table;
crate mod tree;

use crate::prelude::*;

crate use entries::EntriesView;
crate use generic::GenericView;
crate use table::TableView;
crate use tree::TreeView;

crate trait RenderView {
    fn render_view(&self, host: &mut dyn Host) -> Result<(), ShellError>;
}

crate fn print_view(view: &impl RenderView, host: &mut dyn Host) -> Result<(), ShellError> {
    view.render_view(host)
}
