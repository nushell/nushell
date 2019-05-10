use crate::format::RenderView;
use crate::Host;
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

impl RenderView for EntriesView {
    fn render_view(&self, host: &dyn Host) -> Vec<String> {
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
