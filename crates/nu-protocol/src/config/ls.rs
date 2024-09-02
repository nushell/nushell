use super::{prelude::*, report_invalid_config_key, report_invalid_config_value};
use crate as nu_protocol;

#[derive(Clone, Copy, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct LsConfig {
    pub use_ls_colors: bool,
    pub clickable_links: bool,
}

impl Default for LsConfig {
    fn default() -> Self {
        Self {
            use_ls_colors: true,
            clickable_links: true,
        }
    }
}

impl UpdateFromValue for LsConfig {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut Vec<ShellError>,
    ) {
        let span = value.span();
        let Value::Record { val: record, .. } = value else {
            report_invalid_config_value("should be a record", span, path, errors);
            return;
        };

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            let span = val.span();
            match col.as_str() {
                "use_ls_colors" => self.use_ls_colors.update(val, path, errors),
                "clickable_links" => self.clickable_links.update(val, path, errors),
                _ => report_invalid_config_key(span, path, errors),
            }
        }
    }
}
