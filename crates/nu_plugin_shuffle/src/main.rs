use nu_plugin::serve_plugin;
use nu_plugin_shuffle::Shuffle;

fn main() {
    serve_plugin(&mut Shuffle::new());
}
