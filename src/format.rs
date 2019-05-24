crate mod entries;
crate mod generic;
crate mod list;
crate mod table;

use crate::prelude::*;

crate use entries::{EntriesListView, EntriesView};
crate use generic::GenericView;
crate use table::TableView;

crate trait RenderView {
    fn render_view(&self, host: &mut dyn Host) -> Result<(), ShellError>;
}

crate fn print_view(view: &impl RenderView, host: &mut dyn Host) -> Result<(), ShellError> {
    view.render_view(host)
}
