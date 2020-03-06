use crate::format::RenderView;
use crate::prelude::*;
use derive_new::new;
use nu_errors::ShellError;

// A list is printed one line at a time with an optional separator between groups

#[derive(new)]
pub struct ListView {
    list: Vec<Vec<String>>,
    sep: String,
}

impl RenderView for ListView {
    fn render_view(&self, host: &mut dyn Host) -> Result<(), ShellError> {
        for output in &self.list {
            let string: String = output.iter().map(|l| format!("{}\n", l)).collect();
            host.stdout(&format!("{}{}", string, self.sep));
        }

        Ok(())
    }
}
