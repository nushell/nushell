crate mod entries;
crate mod generic;
crate mod list;
crate mod table;

use crate::object::Value;
use crate::Host;

crate use entries::EntriesView;
crate use generic::GenericView;
crate use list::ListView;
crate use table::TableView;

crate trait RenderView {
    fn render_view(&self, host: &dyn Host) -> Vec<String>;
}
