crate mod entries;
crate mod generic;
crate mod list;
crate mod table;

use crate::prelude::*;

crate use entries::{EntriesListView, EntriesView};
crate use generic::GenericView;
crate use table::TableView;

crate trait RenderView {
    fn render_view(&self, host: &dyn Host) -> Vec<String>;
}

fn print_rendered(lines: &[String], host: &mut dyn Host) {
    for line in lines {
        host.stdout(line);
    }
}

crate fn print_view(view: &impl RenderView, host: &mut Host) {
    // let mut ctx = context.lock().unwrap();
    crate::format::print_rendered(&view.render_view(host), host);
}
