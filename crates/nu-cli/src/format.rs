pub(crate) mod entries;
pub(crate) mod generic;
pub(crate) mod list;
pub(crate) mod table;

use crate::prelude::*;
use nu_errors::ShellError;

pub(crate) use entries::EntriesView;
pub(crate) use table::TableView;

pub(crate) trait RenderView {
    fn render_view(&self, host: &mut dyn Host) -> Result<(), ShellError>;
}

pub(crate) fn print_view(view: &impl RenderView, host: &mut dyn Host) -> Result<(), ShellError> {
    view.render_view(host)
}
