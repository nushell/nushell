use crate::format::RenderView;
use crate::prelude::*;

use derive_new::new;

// An entries list is printed like this:
//
// name         : ...
// name2        : ...
// another_name : ...
#[derive(new)]
pub struct EntriesView {
    entries: Vec<(String, String)>,
}

impl EntriesView {
    pub(crate) fn from_value(value: &Value) -> EntriesView {
        let descs = value.data_descriptors();
        let mut entries = vec![];

        for desc in descs {
            let value = value.get_data(&desc);

            let formatted_value = value.borrow().format_leaf().plain_string(75);

            entries.push((desc.clone(), formatted_value))
        }

        EntriesView::new(entries)
    }
}

impl RenderView for EntriesView {
    fn render_view(&self, _host: &mut dyn Host) -> Result<(), ShellError> {
        if self.entries.len() == 0 {
            return Ok(());
        }

        let max_name_size: usize = self.entries.iter().map(|(n, _)| n.len()).max().unwrap();

        for (name, value) in &self.entries {
            outln!("{:width$} : {}", name, value, width = max_name_size)
        }

        Ok(())
    }
}
