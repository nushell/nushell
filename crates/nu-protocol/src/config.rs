use crate::{ShellError, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const ANIMATE_PROMPT_DEFAULT: bool = false;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub filesize_metric: bool,
    pub table_mode: String,
    pub use_ls_colors: bool,
    pub color_config: HashMap<String, String>,
    pub use_grid_icons: bool,
    pub footer_mode: FooterMode,
    pub animate_prompt: bool,
    pub float_precision: i64,
    pub filesize_format: String,
    pub use_ansi_coloring: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            filesize_metric: false,
            table_mode: "rounded".into(),
            use_ls_colors: true,
            color_config: HashMap::new(),
            use_grid_icons: false,
            footer_mode: FooterMode::Never,
            animate_prompt: ANIMATE_PROMPT_DEFAULT,
            float_precision: 4,
            filesize_format: "auto".into(),
            use_ansi_coloring: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum FooterMode {
    /// Never show the footer
    Never,
    /// Always show the footer
    Always,
    /// Only show the footer if there are more than RowCount rows
    RowCount(u64),
    /// Calculate the screen height, calculate row count, if display will be bigger than screen, add the footer
    Auto,
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
                    let (cols, inner_vals) = value.as_record()?;
                    let mut hm = HashMap::new();
                    for (k, v) in cols.iter().zip(inner_vals) {
                        match &v {
                            Value::Record {
                                cols: inner_cols,
                                vals: inner_vals,
                                span: _,
                            } => {
                                // make a string from our config.color_config section that
                                // looks like this: { fg: "#rrggbb" bg: "#rrggbb" attr: "abc", }
                                // the real key here was to have quotes around the values but not
                                // require them around the keys.

                                // maybe there's a better way to generate this but i'm not sure
                                // what it is.
                                let key = k.to_string();
                                let mut val: String = inner_cols
                                    .iter()
                                    .zip(inner_vals)
                                    .map(|(x, y)| {
                                        let clony = y.clone();
                                        format!("{}: \"{}\" ", x, clony.into_string(", ", &config))
                                    })
                                    .collect();
                                // now insert the braces at the front and the back to fake the json string
                                val.insert(0, '{');
                                val.push('}');
                                hm.insert(key, val);
                            }
                            _ => {
                                hm.insert(k.to_string(), v.as_string()?);
                            }
                        }
                    }
                    config.color_config = hm;
                }
                "use_grid_icons" => {
                    config.use_grid_icons = value.as_bool()?;
                }
                "footer_mode" => {
                    let val_str = value.as_string()?.to_lowercase();
                    config.footer_mode = match val_str.as_ref() {
                        "auto" => FooterMode::Auto,
                        "never" => FooterMode::Never,
                        "always" => FooterMode::Always,
                        _ => match &val_str.parse::<u64>() {
                            Ok(number) => FooterMode::RowCount(*number),
                            _ => FooterMode::Never,
                        },
                    };
                }
                "animate_prompt" => {
                    config.animate_prompt = value.as_bool()?;
                }
                "float_precision" => {
                    config.float_precision = value.as_integer()?;
                }
                "use_ansi_coloring" => {
                    config.use_ansi_coloring = value.as_bool()?;
                }
                "filesize_format" => {
                    config.filesize_format = value.as_string()?.to_lowercase();
                }
                _ => {}
            }
        }

        Ok(config)
    }
}
