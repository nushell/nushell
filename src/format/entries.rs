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
    crate fn from_value(value: &Value) -> EntriesView {
        let descs = value.data_descriptors();
        let mut entries = vec![];

        for desc in descs {
            let value = value.get_data(&desc);

            let formatted_value = value.borrow().format_leaf(None);

            entries.push((desc.name.clone(), formatted_value))
        }

        EntriesView::new(entries)
    }
}

impl RenderView for EntriesView {
    fn render_view(&self, _host: &dyn Host) -> Vec<String> {
        if self.entries.len() == 0 {
            return vec![];
        }

        let max_name_size: usize = self.entries.iter().map(|(n, _)| n.len()).max().unwrap();

        self.entries
            .iter()
            .map(|(k, v)| format!("{:width$} : {}", k, v, width = max_name_size))
            .collect()
    }
}

pub struct EntriesListView {
    values: VecDeque<Value>,
}

impl EntriesListView {
    crate fn from_stream(values: VecDeque<Value>) -> EntriesListView {
        EntriesListView { values }
    }
}

impl RenderView for EntriesListView {
    fn render_view(&self, host: &dyn Host) -> Vec<String> {
        if self.values.len() == 0 {
            return vec![];
        }

        let mut strings = vec![];

        let last = self.values.len() - 1;

        for (i, item) in self.values.iter().enumerate() {
            let view = EntriesView::from_value(item);
            let out = view.render_view(host);

            strings.extend(out);

            if i != last {
                strings.push("\n".to_string());
            }
        }

        strings
    }
}
