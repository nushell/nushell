use nu_plugin::serve_plugin;
use nu_plugin_xpath::Xpath;

fn main() {
    serve_plugin(&mut Xpath::new());
}
