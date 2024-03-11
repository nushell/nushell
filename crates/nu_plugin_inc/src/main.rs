use nu_plugin::{serve_plugin, JsonSerializer};
use nu_plugin_inc::IncPlugin;

fn main() {
    serve_plugin(&IncPlugin, JsonSerializer {})
}
