use nu_plugin::serve_plugin;
use nu_plugin_from_bson::FromBson;

fn main() {
    serve_plugin(&mut FromBson::new())
}
