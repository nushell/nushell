// Code adapted from plotly rust
use rand::{
    distributions::{Alphanumeric, DistString},
    thread_rng,
};
use std::process::Command;
use std::{env, fs::File, io::Write};

const DEFAULT_HTML_APP_NOT_FOUND: &str = "Could not find a useable browser to open plot!";

pub fn show_plot(html_str: String) {
    let mut temp = env::temp_dir();
    let mut plot_name = Alphanumeric.sample_string(&mut thread_rng(), 22);
    plot_name.push_str(".html");
    temp.push(plot_name);

    let temp_path = temp.to_str().unwrap();

    {
        let mut file = File::create(temp_path).unwrap();
        file.write_all(html_str.as_bytes())
            .expect("failed to write html output");
        file.flush().unwrap();
    }

    show_with_default_app(temp_path);
}
#[cfg(target_os = "linux")]
fn show_with_default_app(temp_path: &str) {
    Command::new("xdg-open")
        .args([temp_path])
        .output()
        .expect(DEFAULT_HTML_APP_NOT_FOUND);
}

#[cfg(target_os = "macos")]
fn show_with_default_app(temp_path: &str) {
    Command::new("open")
        .args(&[temp_path])
        .output()
        .expect(DEFAULT_HTML_APP_NOT_FOUND);
}

#[cfg(target_os = "windows")]
fn show_with_default_app(temp_path: &str) {
    Command::new("cmd")
        .args(&["/C", "start", &format!(r#"{}"#, temp_path)])
        .spawn()
        .expect(DEFAULT_HTML_APP_NOT_FOUND);
}
