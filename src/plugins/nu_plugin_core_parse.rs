use nu_plugin::serve_plugin;
use nu_plugin_parse::Parse;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    serve_plugin(&mut Parse::new()?);
    Ok(())
}
