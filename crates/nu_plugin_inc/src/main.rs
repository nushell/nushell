use nu_plugin::{serve_plugin, JsonSerializer};
use nu_plugin_inc::Inc;

fn main() {
    serve_plugin(&Inc::new(), JsonSerializer {})
}
