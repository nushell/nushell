use nu_plugin::serve_plugin;
use nu_plugin_ps::Ps;

fn main() {
    serve_plugin(&mut Ps::new());
}
