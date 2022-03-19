use nu_plugin::serve_plugin;
use nu_plugin_to_bson::ToBson;

fn main() {
    serve_plugin(&mut ToBson::new())
}
