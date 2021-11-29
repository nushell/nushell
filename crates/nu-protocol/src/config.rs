use crate::{ShellError, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub filesize_metric: bool,
    pub table_mode: String,
    pub use_ls_colors: bool,
    pub color_config: HashMap<String, String>,
    pub use_grid_icons: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            filesize_metric: false,
            table_mode: "rounded".into(),
            use_ls_colors: true,
            color_config: HashMap::new(),
            use_grid_icons: false,
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
                "color_config" => {
                    let (cols, vals) = value.as_record()?;
                    let mut hm = HashMap::new();
                    for (k, v) in cols.iter().zip(vals) {
                        hm.insert(k.to_string(), v.as_string().unwrap());
                    }
                    config.color_config = hm;
                }
                "use_grid_icons" => {
                    config.use_grid_icons = value.as_bool()?;
                }
                _ => {}
            }
        }

        Ok(config)
    }
}
