use nu_plugin::serve_plugin;
use nu_plugin_start::Start;

fn main() {
    serve_plugin(&mut Start {
        filenames: vec![],
        application: None,
    });
}
