use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{record, ShellError, Span, Value};

use super::helper::{
    process_bool_config, report_invalid_key, report_invalid_value, ReconstructVal,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginGcConfigs {
    /// The config to use for plugins not otherwise specified
    pub default: PluginGcConfig,
    /// Specific configs for plugins (by name)
    pub plugins: HashMap<String, PluginGcConfig>,
}

impl PluginGcConfigs {
    pub(super) fn process(
        &mut self,
        path: &[&str],
        value: &mut Value,
        errors: &mut Vec<ShellError>,
    ) {
        if let Value::Record { val, .. } = value {
            val.retain_mut(|key, value| {
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
            })
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
        val.retain_mut(|key, value| {
            if matches!(value, Value::Record { .. }) {
                plugins.entry(key.to_owned()).or_default().process(
                    &join_path(path, &[key]),
                    value,
                    errors,
                );
                true
            } else {
                report_invalid_value("should be a record", value.span(), errors);
                false
            }
        })
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginGcConfig {
    /// True if the plugin should be removed automatically
    pub enabled: bool,
    /// When to remove the plugin if not in use for this long (in nanoseconds)
    pub remove_after: i64,
}

impl Default for PluginGcConfig {
    fn default() -> Self {
        PluginGcConfig {
            enabled: true,
            remove_after: 10_000_000_000, // 10sec
        }
    }
}

impl PluginGcConfig {
    fn process(&mut self, path: &[&str], value: &mut Value, errors: &mut Vec<ShellError>) {
        if let Value::Record { val, .. } = value {
            val.retain_mut(|key, value| {
                let span = value.span();
                match key {
                    "enabled" => process_bool_config(value, errors, &mut self.enabled),
                    "remove_after" => match value {
                        Value::Duration { val, .. } => {
                            if *val >= 0 {
                                self.remove_after = *val;
                            } else {
                                report_invalid_value("must not be negative", span, errors);
                                *val = self.remove_after;
                            }
                        }
                        _ => {
                            report_invalid_value("should be a duration", span, errors);
                            *value = Value::duration(self.remove_after, span);
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
                "remove_after" => Value::duration(self.remove_after, span),
            },
            span,
        )
    }
}

fn join_path<'a>(a: &[&'a str], b: &[&'a str]) -> Vec<&'a str> {
    a.iter().copied().chain(b.iter().copied()).collect()
}
