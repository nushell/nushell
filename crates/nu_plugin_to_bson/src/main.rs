use nu_plugin::serve_plugin;
use nu_plugin_to_bson::ToBSON;

fn main() {
    serve_plugin(&mut ToBSON::new())
}
