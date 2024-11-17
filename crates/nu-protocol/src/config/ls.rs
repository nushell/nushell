use super::prelude::*;
use crate as nu_protocol;

#[derive(Clone, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct LsConfig {
    pub use_ls_colors: bool,
    pub clickable_links: bool,
    pub sort_by: Vec<LsConfigSortConfig>,
}

#[derive(Clone, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct LsConfigSortConfig {
    pub column: String,
    pub reverse: bool,
    pub ignore_case: bool,
    pub natural: bool,
}

impl UpdateFromValue for Vec<LsConfigSortConfig> {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) {
        if let Ok(value_list) = value.as_list() {
            *self = value_list
                .iter()
                .filter_map(|value_record| {
                    let Value::Record { val: record, .. } = value_record else {
                        errors.type_mismatch(path, Type::record(), value_record);
                        return None;
                    };

                    let mut ls_config_sort_config = LsConfigSortConfig::default();

                    for (col, val) in record.iter() {
                        let path = &mut path.push(col);
                        match col.as_str() {
                            "column" => ls_config_sort_config.column.update(val, path, errors),
                            "reverse" => ls_config_sort_config.reverse.update(val, path, errors),
                            "ignore_case" => {
                                ls_config_sort_config.ignore_case.update(val, path, errors)
                            }
                            "natural" => ls_config_sort_config.natural.update(val, path, errors),
                            _ => errors.unknown_option(path, val),
                        }
                    }

                    if ls_config_sort_config.column.is_empty() {
                        errors.missing_column(path, "column", value_record.span());
                    }

                    Some(ls_config_sort_config)
                })
                .collect();
        } else {
            errors.type_mismatch(path, Type::list(Type::record()), value);
        }
    }
}

impl Default for LsConfig {
    fn default() -> Self {
        Self {
            use_ls_colors: true,
            clickable_links: true,
            sort_by: vec![LsConfigSortConfig {
                column: "name".to_string(),
                reverse: false,
                ignore_case: true,
                natural: true,
            }],
        }
    }
}

impl Default for LsConfigSortConfig {
    fn default() -> Self {
        Self {
            column: "".to_string(),
            reverse: false,
            ignore_case: false,
            natural: false,
        }
    }
}

impl UpdateFromValue for LsConfig {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) {
        let Value::Record { val: record, .. } = value else {
            errors.type_mismatch(path, Type::record(), value);
            return;
        };

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            match col.as_str() {
                "use_ls_colors" => self.use_ls_colors.update(val, path, errors),
                "clickable_links" => self.clickable_links.update(val, path, errors),
                "sort_by" => self.sort_by.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}
