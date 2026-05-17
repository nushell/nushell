use nu_plugin::{JsonSerializer, serve_plugin};
use nu_plugin_inc::IncPlugin;

fn main() {
    serve_plugin(&IncPlugin, JsonSerializer {})
}
