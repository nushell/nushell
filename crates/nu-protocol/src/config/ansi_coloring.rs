use super::{ConfigErrors, ConfigPath, IntoValue, ShellError, UpdateFromValue, Value};
use crate::{self as nu_protocol, FromValue, engine::EngineState};
use serde::{Deserialize, Serialize};
use std::io::IsTerminal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, IntoValue, Serialize, Deserialize)]
pub enum UseAnsiColoring {
    #[default]
    Auto,
    True,
    False,
}

impl UseAnsiColoring {
    /// Determines whether ANSI colors should be used.
    ///
    /// This method evaluates the `UseAnsiColoring` setting and considers environment variables
    /// (`FORCE_COLOR`, `NO_COLOR`, and `CLICOLOR`) when the value is set to `Auto`.
    /// The configuration value (`UseAnsiColoring`) takes precedence over environment variables, as
    /// it is more direct and internally may be modified to override ANSI coloring behavior.
    ///
    /// Most users should have the default value `Auto` which allows the environment variables to
    /// control ANSI coloring.
    /// However, when explicitly set to `True` or `False`, the environment variables are ignored.
    ///
    /// Behavior based on `UseAnsiColoring`:
    /// - `True`: Forces ANSI colors to be enabled, ignoring terminal support and environment variables.
    /// - `False`: Disables ANSI colors completely.
    /// - `Auto`: Determines whether ANSI colors should be used based on environment variables and terminal support.
    ///
    /// When set to `Auto`, the following environment variables are checked in order:
    /// 1. `FORCE_COLOR`: If set, ANSI colors are always enabled, overriding all other settings.
    /// 2. `NO_COLOR`: If set, ANSI colors are disabled, overriding `CLICOLOR` and terminal checks.
    /// 3. `CLICOLOR`: If set, its value determines whether ANSI colors are enabled (`1` for enabled, `0` for disabled).
    ///
    /// If none of these variables are set, ANSI coloring is enabled only if the standard output is
    /// a terminal.
    ///
    /// By prioritizing the `UseAnsiColoring` value, we ensure predictable behavior and prevent
    /// conflicts with internal overrides that depend on this configuration.
    pub fn get(self, engine_state: &EngineState) -> bool {
        let is_terminal = match self {
            Self::Auto => std::io::stdout().is_terminal(),
            Self::True => return true,
            Self::False => return false,
        };

        let env_value = |env_name| {
            engine_state
                .get_env_var_insensitive(env_name)
                .map(|(_, v)| v)
                .and_then(|v| v.coerce_bool().ok())
                .unwrap_or(false)
        };

        if env_value("force_color") {
            return true;
        }

        if env_value("no_color") {
            return false;
        }

        if let Some((_, cli_color)) = engine_state.get_env_var_insensitive("clicolor")
            && let Ok(cli_color) = cli_color.coerce_bool()
        {
            return cli_color;
        }

        is_terminal
    }
}

impl From<bool> for UseAnsiColoring {
    fn from(value: bool) -> Self {
        match value {
            true => Self::True,
            false => Self::False,
        }
    }
}

impl FromValue for UseAnsiColoring {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        if let Ok(v) = v.as_bool() {
            return Ok(v.into());
        }

        #[derive(FromValue)]
        enum UseAnsiColoringString {
            Auto = 0,
            True = 1,
            False = 2,
        }

        Ok(match UseAnsiColoringString::from_value(v)? {
            UseAnsiColoringString::Auto => Self::Auto,
            UseAnsiColoringString::True => Self::True,
            UseAnsiColoringString::False => Self::False,
        })
    }
}

impl UpdateFromValue for UseAnsiColoring {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) {
        let Ok(value) = UseAnsiColoring::from_value(value.clone()) else {
            errors.type_mismatch(path, UseAnsiColoring::expected_type(), value);
            return;
        };

        *self = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::Config;

    fn set_env(engine_state: &mut EngineState, name: &str, value: bool) {
        engine_state.add_env_var(name.to_string(), Value::test_bool(value));
    }

    #[test]
    fn test_use_ansi_coloring_true() {
        let mut engine_state = EngineState::new();
        engine_state.config = Config {
            use_ansi_coloring: UseAnsiColoring::True,
            ..Default::default()
        }
        .into();

        // explicit `True` ignores environment variables
        assert!(
            engine_state
                .get_config()
                .use_ansi_coloring
                .get(&engine_state)
        );

        set_env(&mut engine_state, "clicolor", false);
        assert!(
            engine_state
                .get_config()
                .use_ansi_coloring
                .get(&engine_state)
        );
        set_env(&mut engine_state, "clicolor", true);
        assert!(
            engine_state
                .get_config()
                .use_ansi_coloring
                .get(&engine_state)
        );
        set_env(&mut engine_state, "no_color", true);
        assert!(
            engine_state
                .get_config()
                .use_ansi_coloring
                .get(&engine_state)
        );
        set_env(&mut engine_state, "force_color", true);
        assert!(
            engine_state
                .get_config()
                .use_ansi_coloring
                .get(&engine_state)
        );
    }

    #[test]
    fn test_use_ansi_coloring_false() {
        let mut engine_state = EngineState::new();
        engine_state.config = Config {
            use_ansi_coloring: UseAnsiColoring::False,
            ..Default::default()
        }
        .into();

        // explicit `False` ignores environment variables
        assert!(
            !engine_state
                .get_config()
                .use_ansi_coloring
                .get(&engine_state)
        );

        set_env(&mut engine_state, "clicolor", false);
        assert!(
            !engine_state
                .get_config()
                .use_ansi_coloring
                .get(&engine_state)
        );
        set_env(&mut engine_state, "clicolor", true);
        assert!(
            !engine_state
                .get_config()
                .use_ansi_coloring
                .get(&engine_state)
        );
        set_env(&mut engine_state, "no_color", true);
        assert!(
            !engine_state
                .get_config()
                .use_ansi_coloring
                .get(&engine_state)
        );
        set_env(&mut engine_state, "force_color", true);
        assert!(
            !engine_state
                .get_config()
                .use_ansi_coloring
                .get(&engine_state)
        );
    }

    #[test]
    fn test_use_ansi_coloring_auto() {
        let mut engine_state = EngineState::new();
        engine_state.config = Config {
            use_ansi_coloring: UseAnsiColoring::Auto,
            ..Default::default()
        }
        .into();

        // no environment variables, behavior depends on terminal state
        let is_terminal = std::io::stdout().is_terminal();
        assert_eq!(
            engine_state
                .get_config()
                .use_ansi_coloring
                .get(&engine_state),
            is_terminal
        );

        // `clicolor` determines ANSI behavior if no higher-priority variables are set
        set_env(&mut engine_state, "clicolor", true);
        assert!(
            engine_state
                .get_config()
                .use_ansi_coloring
                .get(&engine_state)
        );

        set_env(&mut engine_state, "clicolor", false);
        assert!(
            !engine_state
                .get_config()
                .use_ansi_coloring
                .get(&engine_state)
        );

        // `no_color` overrides `clicolor` and terminal state
        set_env(&mut engine_state, "no_color", true);
        assert!(
            !engine_state
                .get_config()
                .use_ansi_coloring
                .get(&engine_state)
        );

        // `force_color` overrides everything
        set_env(&mut engine_state, "force_color", true);
        assert!(
            engine_state
                .get_config()
                .use_ansi_coloring
                .get(&engine_state)
        );
    }
}
