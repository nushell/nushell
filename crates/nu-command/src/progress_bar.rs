use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use std::fmt;

// This module includes the progress bar used to show the progress when using the command `save`
// Eventually it would be nice to find a better place for it.

pub struct NuProgressBar {
    pub pb: ProgressBar,
    bytes_processed: u64,
    total_bytes: Option<u64>,
}

impl NuProgressBar {
    pub fn new(total_bytes: Option<u64>) -> NuProgressBar {
        // Let's create the progress bar template.
        let template = match total_bytes {
            Some(_) => {
                // We will use a progress bar if we know the total bytes of the stream
                ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} {binary_bytes_per_sec} ({eta}) {msg}")
            }
            _ => {
                // But if we don't know the total then we just show the stats progress
                ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] {bytes} {binary_bytes_per_sec} {msg}",
                )
            }
        };

        let total_bytes = match total_bytes {
            Some(total_size) => total_size,
            _ => 0,
        };

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
            total_bytes: None,
            bytes_processed: 0,
        }
    }

    pub fn update_bar(&mut self, bytes_processed: u64) {
        self.pb.set_position(bytes_processed);
    }

    // Commenting this for now but adding it in the future
    //pub fn finished_msg(&self, msg: String) {
    //    self.pb.finish_with_message(msg);
    //}

    pub fn abandoned_msg(&self, msg: String) {
        self.pb.abandon_with_message(msg);
    }

    pub fn clone(&self) -> NuProgressBar {
        NuProgressBar {
            pb: self.pb.clone(),
            bytes_processed: self.bytes_processed,
            total_bytes: self.total_bytes,
        }
    }
}
