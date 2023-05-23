use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use std::fmt;

// This module includes the progress bar used to show the progress when using the command `save`
// Eventually it would be nice to find a better place for it.

pub struct NuProgressBar {
    pub pb: ProgressBar,
    bytes_processed: u64,
    total_bytes: Option<u64>,
}

#[derive(PartialEq)]
pub enum ProgressType {
    ProgressBytes,
    ProgressBytesUnknown,
    ProgressItems,
    ProgressUnknown,
}

pub fn nu_progress_style(progress_type: ProgressType) -> ProgressStyle {
    let template_str = match progress_type {
        ProgressType::ProgressBytes => "{spinner:.green} [{elapsed_precise}] [{bar:30.cyan/blue}] [{bytes}/{total_bytes}] {binary_bytes_per_sec} {wide_msg}",
        ProgressType::ProgressBytesUnknown => "{spinner:.green} [{elapsed_precise}] {bytes} {binary_bytes_per_sec} {wide_msg}",
        ProgressType::ProgressItems => "{spinner:.green} [{elapsed_precise}] [{bar:30.cyan/blue}] [{pos}/{len}] {wide_msg}",
        ProgressType::ProgressUnknown => "{spinner:.green} [{elapsed_precise}] {wide_msg}",
    };

    return ProgressStyle::with_template(template_str)
        .unwrap_or_else(|_| ProgressStyle::default_bar())
        .with_key(
            "eta",
            |state: &ProgressState, w: &mut dyn std::fmt::Write| {
                let _ = std::fmt::write(w, format_args!("{:.1}s", state.eta().as_secs_f64()));
            },
        )
        .progress_chars("#>-");
}

impl NuProgressBar {
    pub fn new(
        progress_type: ProgressType,
        total_progress: Option<u64>,
        msg: String,
    ) -> NuProgressBar {
        let (progress_flag_current, progress_flag_goal) = match progress_type {
            ProgressType::ProgressBytes => ("{bytes}", "{total_bytes}"),
            ProgressType::ProgressBytesUnknown => ("", ""),
            ProgressType::ProgressItems => ("{pos}", "{len}"),
            ProgressType::ProgressUnknown => ("", ""),
        };

        let progress_flag_eta = if progress_type == ProgressType::ProgressBytes {
            "({eta})"
        } else {
            ""
        };
        let progress_flag_bytes_per_sec = if progress_type == ProgressType::ProgressBytes {
            "{binary_bytes_per_sec}"
        } else {
            ""
        };

        // Let's create the progress bar template.
        let template = match total_progress {
            Some(_) => {
                let str_template = format!(
                    "{{spinner:.green}} [{{elapsed_precise}}] [{{bar:30.cyan/blue}}] [{}/{}] {} {} {{wide_msg}}",
                    progress_flag_current,
                    progress_flag_goal,
                    progress_flag_bytes_per_sec,
                    progress_flag_eta
                );

                // We will use a progress bar if we know the total bytes of the stream
                ProgressStyle::with_template(&str_template)
            }
            _ => {
                let str_template = format!(
                    "{{spinner:.green}} [{{elapsed_precise}}] {} {} {{wide_msg}}",
                    progress_flag_current, progress_flag_bytes_per_sec
                );

                // But if we don't know the total then we just show the stats progress
                ProgressStyle::with_template(&str_template)
            }
        };

        let total_progress = total_progress.unwrap_or_default();

        let new_progress_bar = ProgressBar::new(total_progress);
        new_progress_bar.set_style(
            template
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .with_key("eta", |state: &ProgressState, w: &mut dyn fmt::Write| {
                    let _ = fmt::write(w, format_args!("{:.1}s", state.eta().as_secs_f64()));
                })
                .progress_chars("#>-"),
        );

        new_progress_bar.set_message(msg);

        NuProgressBar {
            pb: new_progress_bar,
            total_bytes: None,
            bytes_processed: 0,
        }
    }

    pub fn update_bar(&mut self, bytes_processed: u64) {
        self.pb.set_position(bytes_processed);
    }

    // pub fn finished_msg(&self, msg: String, clear: bool) {
    //     if clear {
    //         self.pb.finish_and_clear();
    //     } else {
    //         self.pb.finish_with_message(msg);
    //     }
    // }

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
