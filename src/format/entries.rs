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
    entries: Vec<(crate::object::DescriptorName, String)>,
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
    fn render_view(&self, _host: &mut dyn Host) -> Result<(), ShellError> {
        if self.entries.len() == 0 {
            return Ok(());
        }

        let max_name_size: usize = self
            .entries
            .iter()
            .map(|(n, _)| n.display().len())
            .max()
            .unwrap();

        for (name, value) in &self.entries {
            println!(
                "{:width$} : {}",
                name.display(),
                value,
                width = max_name_size
            )
        }

        Ok(())
    }
}

pub struct EntriesListView {
    values: VecDeque<Value>,
}

impl EntriesListView {
    crate async fn from_stream(values: InputStream) -> EntriesListView {
        EntriesListView {
            values: values.collect().await,
        }
    }
}

impl RenderView for EntriesListView {
    fn render_view(&self, host: &mut dyn Host) -> Result<(), ShellError> {
        if self.values.len() == 0 {
            return Ok(());
        }

        let last = self.values.len() - 1;

        for (i, item) in self.values.iter().enumerate() {
            let view = EntriesView::from_value(item);
            view.render_view(host);

            if i != last {
                host.stdout("\n");
            }
        }

        Ok(())
    }
}
