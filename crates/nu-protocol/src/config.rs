use serde::{Deserialize, Serialize};

use crate::{ShellError, Value};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub filesize_metric: bool,
    pub table_mode: String,
    pub use_ls_colors: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            filesize_metric: false,
            table_mode: "rounded".into(),
            use_ls_colors: true,
        }
    }
}

impl Value {
    pub fn into_config(self) -> Result<Config, ShellError> {
        let v = self.as_record()?;

        let mut config = Config::default();

        for (key, value) in v.0.iter().zip(v.1) {
            match key.as_str() {
                "filesize_metric" => {
                    config.filesize_metric = value.as_bool()?;
                }
                "table_mode" => {
                    config.table_mode = value.as_string()?;
                }
                "use_ls_colors" => {
                    config.use_ls_colors = value.as_bool()?;
                }
                _ => {}
            }
        }

        Ok(config)
    }
}
