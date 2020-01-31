use nu_plugin::serve_plugin;
use nu_plugin_textview::TextView;

fn main() {
    serve_plugin(&mut TextView::new());
}
