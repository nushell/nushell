use nu_plugin::serve_plugin;
use nu_plugin_from_bson::FromBSON;

fn main() {
    serve_plugin(&mut FromBSON::new())
}
