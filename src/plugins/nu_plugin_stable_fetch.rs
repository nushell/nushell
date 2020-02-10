use nu_plugin::serve_plugin;
use nu_plugin_fetch::Fetch;

fn main() {
    serve_plugin(&mut Fetch::new());
}
