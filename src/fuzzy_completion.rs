use dialoguer::{
    console::{Style, Term},
    theme::ColorfulTheme,
    Select,
};
use reedline::{Completer, CompletionActionHandler, LineBuffer};

pub(crate) struct FuzzyCompletion {
    pub completer: Box<dyn Completer>,
}

impl CompletionActionHandler for FuzzyCompletion {
    fn handle(&mut self, present_buffer: &mut LineBuffer) {
        let completions = self
            .completer
            .complete(present_buffer.get_buffer(), present_buffer.offset());

        if completions.is_empty() {
            // do nothing
        } else if completions.len() == 1 {
            let span = completions[0].0;

            let mut offset = present_buffer.offset();
            offset += completions[0].1.len() - (span.end - span.start);

            // TODO improve the support for multiline replace
            present_buffer.replace(span.start..span.end, &completions[0].1);
            present_buffer.set_insertion_point(offset);
        } else {
            let selections: Vec<_> = completions.iter().map(|(_, string)| string).collect();

            let _ = crossterm::terminal::disable_raw_mode();
            println!();
            let theme = ColorfulTheme {
                active_item_style: Style::new().for_stderr().on_green().black(),
                ..Default::default()
            };
            let result = Select::with_theme(&theme)
                .default(0)
                .items(&selections[..])
                .interact_on_opt(&Term::stdout())
                .unwrap_or(None);
            let _ = crossterm::terminal::enable_raw_mode();

            if let Some(result) = result {
                let span = completions[result].0;

                let mut offset = present_buffer.offset();
                offset += completions[result].1.len() - (span.end - span.start);

                // TODO improve the support for multiline replace
                present_buffer.replace(span.start..span.end, &completions[result].1);
                present_buffer.set_insertion_point(offset);
            }
        }
    }
}
