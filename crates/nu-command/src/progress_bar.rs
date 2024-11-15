use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use std::fmt;

// This module includes the progress bar used to show the progress when using the command `save`
// Eventually it would be nice to find a better place for it.

pub struct NuProgressBar {
    pub pb: ProgressBar,
}

impl NuProgressBar {
    pub fn new(total_bytes: Option<u64>) -> NuProgressBar {
        // Let's create the progress bar template.
        let template = match total_bytes {
            Some(_) => {
                // We will use a progress bar if we know the total bytes of the stream
                ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{bar:30.cyan/blue}] [{bytes}/{total_bytes}] {binary_bytes_per_sec} ({eta}) {wide_msg}")
            }
            _ => {
                // But if we don't know the total then we just show the stats progress
                ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] {bytes} {binary_bytes_per_sec} {wide_msg}",
                )
            }
        };

        let total_bytes = total_bytes.unwrap_or_default();

        let new_progress_bar = ProgressBar::new(total_bytes);
        new_progress_bar.set_style(
            template
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .with_key("eta", |state: &ProgressState, w: &mut dyn fmt::Write| {
                    let _ = fmt::write(w, format_args!("{:.1}s", state.eta().as_secs_f64()));
                })
                .progress_chars("#>-"),
        );

        NuProgressBar {
            pb: new_progress_bar,
        }
    }

    pub fn update_bar(&mut self, bytes_processed: u64) {
        self.pb.set_position(bytes_processed);
    }

    pub fn abandoned_msg(&self, msg: String) {
        self.pb.abandon_with_message(msg);
    }
}
