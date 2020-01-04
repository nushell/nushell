use crate::data::value;
use crate::format::RenderView;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::Value;

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

            let formatted_value = value::format_leaf(value.borrow()).plain_string(75);

            entries.push((desc.clone(), formatted_value))
        }

        EntriesView::new(entries)
    }
}

impl RenderView for EntriesView {
    fn render_view(&self, _host: &mut dyn Host) -> Result<(), ShellError> {
        if self.entries.is_empty() {
            return Ok(());
        }

        if let Some(max_name_size) = self.entries.iter().map(|(n, _)| n.len()).max() {
            for (name, value) in &self.entries {
                outln!("{:width$} : {}", name, value, width = max_name_size)
            }
        }

        Ok(())
    }
}
