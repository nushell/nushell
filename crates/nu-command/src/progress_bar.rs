use std::io::{Write, stdout};
use crossterm::{
    QueueableCommand, style::Print,
    cursor
};

use std::thread;
use std::fmt;
use indicatif::{ProgressBar, ProgressState, ProgressStyle, MultiProgress};

// This module includes the progress bar used to show the progress when using the command `save`
// Eventually it would be nice to find a better plece for it.

pub struct KnownSizeBar{
    pub pb: ProgressBar,
    bytes_downloaded: u64,
    total_bytes: Option<u64>,
}

impl KnownSizeBar {
    pub fn new(total_bytes: Option<u64>) -> KnownSizeBar {
        match total_bytes {
            Some(total_size) => {
                KnownSizeBar {
                    pb: ProgressBar::new(total_size),
                    total_bytes: total_bytes,
                    bytes_downloaded: 0
                }
            }
            _ => {
                let r_num = 10; // <== the value of this variable doesn't matter 
                KnownSizeBar {
                    pb: ProgressBar::new(r_num),
                    total_bytes: None,
                    bytes_downloaded: 0
                }
            }
        }
        
    }

    pub fn create_style(&self){
        let template = match self.total_bytes {
            Some(_) => {
                ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} {binary_bytes_per_sec} ({eta})")
            }
            _ => {
                ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] {bytes} {binary_bytes_per_sec}")
            }
        };

        self.pb.set_style(template
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn fmt::Write| {
            fmt::write(w, format_args!("{:.1}s", state.eta().as_secs_f64()) ).unwrap()
        })
        .progress_chars("#>-"));
    }

    pub fn update_bar(&mut self, bytes_downloaded: u64){
        self.pb.set_position(bytes_downloaded);
    }

    pub fn bar_finished_msg(self){
        self.pb.finish_with_message("Downloaded!");
    }
}

pub fn print_progress_unk_length(bytes_downloaded: usize){
    let cursor_max_length: u16 = 20;
    let mut cursor_current_x_pos: u16 = 0;
    let _amount_downloaded = format!("{:.2}MB", (bytes_downloaded as f64) / 1000000.0);
    //let cursor_pos = cursor::position().unwrap();

    //println!("[{}] ", " ".repeat(cursor_max_length.into()));
    let cursor_pos = cursor::position().unwrap();

    if cursor_current_x_pos >= cursor_max_length { cursor_current_x_pos = 0 };

    let bar_print = format!("[{}{}]  {:.2}MB",
        " ".repeat(cursor_current_x_pos.into())+"#",
        " ".repeat((cursor_max_length - cursor_current_x_pos - 1).into()),
        bytes_downloaded
    );

    stdout()
        .queue(cursor::SavePosition)
        .unwrap()
        .queue(cursor::MoveTo(cursor_pos.0+1, cursor_pos.1))
        .unwrap()
        .queue(Print(bar_print))
        .unwrap()
        .queue(cursor::RestorePosition)
        .unwrap();

    stdout().flush().unwrap();
}




