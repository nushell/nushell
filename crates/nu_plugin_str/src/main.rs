use nu_plugin::serve_plugin;
use nu_plugin_str::Str;

fn main() {
    serve_plugin(&mut Str::new())
}
