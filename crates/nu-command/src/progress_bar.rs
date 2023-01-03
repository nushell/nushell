use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use std::fmt;

// This module includes the progress bar used to show the progress when using the command `save`
// Eventually it would be nice to find a better plece for it.

pub struct NuProgressBar {
    pub pb: ProgressBar,
    bytes_downloaded: u64,
    total_bytes: Option<u64>,
}

impl NuProgressBar {
    pub fn new(total_bytes: Option<u64>) -> NuProgressBar {
        match total_bytes {
            Some(total_size) => NuProgressBar {
                pb: ProgressBar::new(total_size),
                total_bytes: total_bytes,
                bytes_downloaded: 0,
            },
            _ => {
                let r_num = 10; // <== the value of this variable doesn't matter
                NuProgressBar {
                    pb: ProgressBar::new(r_num),
                    total_bytes: None,
                    bytes_downloaded: 0,
                }
            }
        }
    }

    pub fn create_style(&self) {
        let template = match self.total_bytes {
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

        self.pb.set_style(
            template
                .unwrap()
                .with_key("eta", |state: &ProgressState, w: &mut dyn fmt::Write| {
                    fmt::write(w, format_args!("{:.1}s", state.eta().as_secs_f64())).unwrap()
                })
                .progress_chars("#>-"),
        );
    }

    pub fn update_bar(&mut self, bytes_downloaded: u64) {
        self.pb.set_position(bytes_downloaded);
    }

    pub fn bar_finished_msg(&self, msg: String) {
        self.pb.finish_with_message(msg);
    }

    pub fn clone(&self) -> NuProgressBar {
        NuProgressBar {
            pb: self.pb.clone(),
            bytes_downloaded: self.bytes_downloaded.clone(),
            total_bytes: self.total_bytes.clone(),
        }
    }
}
