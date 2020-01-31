use nu_plugin::serve_plugin;
use nu_plugin_sys::Sys;

fn main() {
    serve_plugin(&mut Sys::new());
}
