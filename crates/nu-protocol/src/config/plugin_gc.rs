use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{record, ShellError, Span, Value};

use super::helper::{
    process_bool_config, report_invalid_key, report_invalid_value, ReconstructVal,
};

/// Configures when plugins should be stopped if inactive
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
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

    pub(super) fn process(
        &mut self,
        path: &[&str],
        value: &mut Value,
        errors: &mut Vec<ShellError>,
    ) {
        if let Value::Record { val, .. } = value {
            // Handle resets to default if keys are missing
            if !val.contains("default") {
                self.default = PluginGcConfig::default();
            }
            if !val.contains("plugins") {
                self.plugins = HashMap::new();
            }

            val.to_mut().retain_mut(|key, value| {
                let span = value.span();
                match key {
                    "default" => {
                        self.default
                            .process(&join_path(path, &["default"]), value, errors)
                    }
                    "plugins" => process_plugins(
                        &join_path(path, &["plugins"]),
                        value,
                        errors,
                        &mut self.plugins,
                    ),
                    _ => {
                        report_invalid_key(&join_path(path, &[key]), span, errors);
                        return false;
                    }
                }
                true
            });
        } else {
            report_invalid_value("should be a record", value.span(), errors);
            *value = self.reconstruct_value(value.span());
        }
    }
}

impl ReconstructVal for PluginGcConfigs {
    fn reconstruct_value(&self, span: Span) -> Value {
        Value::record(
            record! {
                "default" => self.default.reconstruct_value(span),
                "plugins" => reconstruct_plugins(&self.plugins, span),
            },
            span,
        )
    }
}

fn process_plugins(
    path: &[&str],
    value: &mut Value,
    errors: &mut Vec<ShellError>,
    plugins: &mut HashMap<String, PluginGcConfig>,
) {
    if let Value::Record { val, .. } = value {
        // Remove any plugin configs that aren't in the value
        plugins.retain(|key, _| val.contains(key));

        val.to_mut().retain_mut(|key, value| {
            if matches!(value, Value::Record { .. }) {
                plugins.entry(key.to_owned()).or_default().process(
                    &join_path(path, &[key]),
                    value,
                    errors,
                );
                true
            } else {
                report_invalid_value("should be a record", value.span(), errors);
                if let Some(conf) = plugins.get(key) {
                    // Reconstruct the value if it existed before
                    *value = conf.reconstruct_value(value.span());
                    true
                } else {
                    // Remove it if it didn't
                    false
                }
            }
        });
    }
}

fn reconstruct_plugins(plugins: &HashMap<String, PluginGcConfig>, span: Span) -> Value {
    Value::record(
        plugins
            .iter()
            .map(|(key, val)| (key.to_owned(), val.reconstruct_value(span)))
            .collect(),
        span,
    )
}

/// Configures when a plugin should be stopped if inactive
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl PluginGcConfig {
    fn process(&mut self, path: &[&str], value: &mut Value, errors: &mut Vec<ShellError>) {
        if let Value::Record { val, .. } = value {
            // Handle resets to default if keys are missing
            if !val.contains("enabled") {
                self.enabled = PluginGcConfig::default().enabled;
            }
            if !val.contains("stop_after") {
                self.stop_after = PluginGcConfig::default().stop_after;
            }

            val.to_mut().retain_mut(|key, value| {
                let span = value.span();
                match key {
                    "enabled" => process_bool_config(value, errors, &mut self.enabled),
                    "stop_after" => match value {
                        Value::Duration { val, .. } => {
                            if *val >= 0 {
                                self.stop_after = *val;
                            } else {
                                report_invalid_value("must not be negative", span, errors);
                                *val = self.stop_after;
                            }
                        }
                        _ => {
                            report_invalid_value("should be a duration", span, errors);
                            *value = Value::duration(self.stop_after, span);
                        }
                    },
                    _ => {
                        report_invalid_key(&join_path(path, &[key]), span, errors);
                        return false;
                    }
                }
                true
            })
        } else {
            report_invalid_value("should be a record", value.span(), errors);
            *value = self.reconstruct_value(value.span());
        }
    }
}

impl ReconstructVal for PluginGcConfig {
    fn reconstruct_value(&self, span: Span) -> Value {
        Value::record(
            record! {
                "enabled" => Value::bool(self.enabled, span),
                "stop_after" => Value::duration(self.stop_after, span),
            },
            span,
        )
    }
}

fn join_path<'a>(a: &[&'a str], b: &[&'a str]) -> Vec<&'a str> {
    a.iter().copied().chain(b.iter().copied()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn process() {
        let (expected, mut input) = test_pair();
        let mut errors = vec![];
        let mut result = PluginGcConfigs::default();
        result.process(&[], &mut input, &mut errors);
        assert!(errors.is_empty(), "errors: {errors:#?}");
        assert_eq!(expected, result);
    }

    #[test]
    fn reconstruct() {
        let (input, expected) = test_pair();
        assert_eq!(expected, input.reconstruct_value(Span::test_data()));
    }
}
