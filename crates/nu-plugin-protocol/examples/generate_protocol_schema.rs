//! Regenerate `protocol_schema/plugin_protocol.schema.json`.
//!
//! ```text
//! cargo run -p nu-plugin-protocol --example generate_protocol_schema --features schema
//! ```
//!
//! The schema is envelope-oriented documentation only; many nested payloads are free-form JSON.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(feature = "schema"))]
    {
        eprintln!(
            "error: rebuild with `--features schema` to generate the protocol schema document"
        );
        std::process::exit(1);
    }

    #[cfg(feature = "schema")]
    {
        use std::{env, fs, path::PathBuf};

        let schema = nu_plugin_protocol::schema::plugin_protocol_schema_pretty()?;

        let output_path = match env::args().nth(1) {
            Some(path) => PathBuf::from(path),
            None => {
                let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                path.push("protocol_schema");
                path.push("plugin_protocol.schema.json");
                path
            }
        };

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&output_path, schema)?;
        println!("Wrote {}", output_path.display());
        Ok(())
    }
}
