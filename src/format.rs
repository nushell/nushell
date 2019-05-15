crate mod entries;
crate mod generic;
crate mod list;
crate mod table;

use crate::object::Value;
use crate::prelude::*;

crate use entries::{EntriesListView, EntriesView};
crate use generic::GenericView;
crate use list::ListView;
crate use table::TableView;

crate trait RenderView {
    fn render_view(&self, host: &dyn Host) -> Vec<String>;
}

crate fn print_rendered(lines: &[String], host: &mut dyn Host) {
    for line in lines {
        host.stdout(line);
    }
}
