use nu_plugin::serve_plugin;
use nu_plugin_gstat::GStat;

fn main() {
    serve_plugin(&mut GStat::new())
}
