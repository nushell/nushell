use nu_plugin::{serve_plugin, EncodingType};
use nu_plugin_inc::Inc;

fn main() {
    serve_plugin(&mut Inc::new(), EncodingType::Json)
}
