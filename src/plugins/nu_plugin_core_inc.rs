<<<<<<< HEAD
use nu_plugin::serve_plugin;
use nu_plugin_inc::Inc;

fn main() {
    serve_plugin(&mut Inc::new());
=======
use nu_plugin::{serve_plugin, CapnpSerializer};
use nu_plugin_inc::Inc;

fn main() {
    serve_plugin(&mut Inc::new(), CapnpSerializer {})
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}
