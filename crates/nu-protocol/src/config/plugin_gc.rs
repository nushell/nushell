use super::{prelude::*, report_invalid_config_key, report_invalid_config_value};
use crate as nu_protocol;
use std::collections::HashMap;

/// Configures when plugins should be stopped if inactive
#[derive(Clone, Debug, Default, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginGcConfigs {
    /// The config to use for plugins not otherwise specified
    pub default: PluginGcConfig,
    /// Specific configs for plugins (by name)
    pub plugins: HashMap<String, PluginGcConfig>,
}

impl PluginGcConfigs {
    /// Get the plugin GC configuration for a specific plugin name. If not specified by name in the
    /// config, this is `default`.
    pub fn get(&self, plugin_name: &str) -> &PluginGcConfig {
        self.plugins.get(plugin_name).unwrap_or(&self.default)
    }
}

impl UpdateFromValue for PluginGcConfigs {
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
                "default" => self.default.update(val, path, errors),
                "plugins" => self.plugins.update(val, path, errors),
                _ => report_invalid_config_key(span, path, errors),
            }
        }
    }
}

/// Configures when a plugin should be stopped if inactive
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginGcConfig {
    /// True if the plugin should be stopped automatically
    pub enabled: bool,
    /// When to stop the plugin if not in use for this long (in nanoseconds)
    pub stop_after: i64,
}

impl Default for PluginGcConfig {
    fn default() -> Self {
        PluginGcConfig {
            enabled: true,
            stop_after: 10_000_000_000, // 10sec
        }
    }
}

impl IntoValue for PluginGcConfig {
    fn into_value(self, span: Span) -> Value {
        record! {
            "enabled" => self.enabled.into_value(span),
            "stop_after" => Value::duration(self.stop_after, span),
        }
        .into_value(span)
    }
}

impl UpdateFromValue for PluginGcConfig {
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
                "enabled" => self.enabled.update(val, path, errors),
                "stop_after" => {
                    if let Ok(val) = val.as_duration() {
                        if val >= 0 {
                            self.stop_after = val;
                        } else {
                            report_invalid_config_value(
                                "should be a non-negative duration",
                                span,
                                path,
                                errors,
                            );
                        }
                    } else {
                        report_invalid_config_value(
                            "should be a non-negative duration",
                            span,
                            path,
                            errors,
                        );
                    }
                }
                _ => report_invalid_config_key(span, path, errors),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::{record, Span};

    fn test_pair() -> (PluginGcConfigs, Value) {
        (
            PluginGcConfigs {
                default: PluginGcConfig {
                    enabled: true,
                    stop_after: 30_000_000_000,
                },
                plugins: [(
                    "my_plugin".to_owned(),
                    PluginGcConfig {
                        enabled: false,
                        stop_after: 0,
                    },
                )]
                .into_iter()
                .collect(),
            },
            Value::test_record(record! {
                "default" => Value::test_record(record! {
                    "enabled" => Value::test_bool(true),
                    "stop_after" => Value::test_duration(30_000_000_000),
                }),
                "plugins" => Value::test_record(record! {
                    "my_plugin" => Value::test_record(record! {
                        "enabled" => Value::test_bool(false),
                        "stop_after" => Value::test_duration(0),
                    }),
                }),
            }),
        )
    }

    #[test]
    fn update() {
        let (expected, input) = test_pair();
        let mut errors = vec![];
        let mut result = PluginGcConfigs::default();
        result.update(&input, &mut ConfigPath::new(), &mut errors);
        assert!(errors.is_empty(), "errors: {errors:#?}");
        assert_eq!(expected, result);
    }

    #[test]
    fn reconstruct() {
        let (input, expected) = test_pair();
        assert_eq!(expected, input.into_value(Span::test_data()));
    }
}
