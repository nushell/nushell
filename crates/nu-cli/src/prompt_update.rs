use crate::NushellPrompt;
use log::trace;
use nu_cmd_base::prompt::{
    PROMPT_COMMAND, PROMPT_COMMAND_RIGHT, PROMPT_INDICATOR, PROMPT_INDICATOR_VI_INSERT,
    PROMPT_INDICATOR_VI_NORMAL, PROMPT_MULTILINE_INDICATOR, TRANSIENT_PROMPT_COMMAND,
    TRANSIENT_PROMPT_COMMAND_RIGHT, TRANSIENT_PROMPT_INDICATOR,
    TRANSIENT_PROMPT_INDICATOR_VI_INSERT, TRANSIENT_PROMPT_INDICATOR_VI_NORMAL,
    TRANSIENT_PROMPT_MULTILINE_INDICATOR, render, resolve_prompt_source,
};
use nu_protocol::{
    Config,
    engine::{EngineState, PromptContents, Stack},
};
use reedline::Prompt;
use std::sync::Arc;

// ────────────────────────────────────────────────────────────────────────────────
// OSC 133 / OSC 633 COMMAND EXECUTION MARKERS
// ────────────────────────────────────────────────────────────────────────────────
// These escape sequences are used by the shell to mark command execution boundaries.
// Note: A/B/P markers for prompts are now handled by reedline.

// Command execution markers (C = pre-exec, D = post-exec with exit code)
pub(crate) const PRE_EXECUTION_MARKER: &str = "\x1b]133;C\x1b\\";
pub(crate) const POST_EXECUTION_MARKER_PREFIX: &str = "\x1b]133;D;";
pub(crate) const POST_EXECUTION_MARKER_SUFFIX: &str = "\x1b\\";

// VS Code specific markers (OSC 633)
pub(crate) const VSCODE_PRE_EXECUTION_MARKER: &str = "\x1b]633;C\x1b\\";
pub(crate) const VSCODE_POST_EXECUTION_MARKER_PREFIX: &str = "\x1b]633;D;";
pub(crate) const VSCODE_POST_EXECUTION_MARKER_SUFFIX: &str = "\x1b\\";
pub(crate) const VSCODE_COMMANDLINE_MARKER_PREFIX: &str = "\x1b]633;E;";
pub(crate) const VSCODE_COMMANDLINE_MARKER_SUFFIX: &str = "\x1b\\";
pub(crate) const VSCODE_CWD_PROPERTY_MARKER_PREFIX: &str = "\x1b]633;P;Cwd=";
pub(crate) const VSCODE_CWD_PROPERTY_MARKER_SUFFIX: &str = "\x1b\\";

// Reset terminal application mode sequence
pub(crate) const RESET_APPLICATION_MODE: &str = "\x1b[?1l";

/// Re-evaluate the prompt and install it as the per-cycle baseline. This
/// overwrites anything a background job pushed during the previous cycle; the
/// pushes this cycle's `PromptState::begin_update` kept stay on top.
pub fn update_prompt(config: &Config, engine_state: &EngineState, stack: &Stack) {
    let new_contents = build_prompt_contents(config, engine_state, stack);

    // reedline handles semantic markers itself.
    engine_state.prompt_state.set_contents(new_contents);

    trace!("update_prompt {}:{}:{}", file!(), line!(), column!());
}

fn build_prompt_contents(
    config: &Config,
    engine_state: &EngineState,
    stack: &Stack,
) -> PromptContents {
    let prompt = &config.prompt;
    let source = |env_var, configured| {
        resolve_prompt_source(env_var, configured, config, engine_state, stack)
    };

    // Fixed order: any of these may hold a closure, so reshuffling them would
    // reorder side effects. An indicator that fails to render is `None`, which
    // `NushellPrompt` draws as the built-in one.
    let left = source(PROMPT_COMMAND, prompt.left.as_ref());
    let right = source(PROMPT_COMMAND_RIGHT, prompt.right.as_ref());
    let indicator = source(PROMPT_INDICATOR, Some(&prompt.indicator));
    let vi_insert = source(PROMPT_INDICATOR_VI_INSERT, Some(&prompt.vi_insert));
    let vi_normal = source(PROMPT_INDICATOR_VI_NORMAL, Some(&prompt.vi_normal));
    // Config-only: vi visual used to reuse the vi normal indicator.
    let vi_visual = render(&prompt.vi_visual, config, engine_state, stack);
    let multiline = source(PROMPT_MULTILINE_INDICATOR, Some(&prompt.multiline));

    PromptContents {
        left: left.map(Arc::from),
        // Reset the color on an empty right prompt, so the left prompt's
        // styling cannot bleed across the line.
        right: right.map(|right| {
            if right.is_empty() {
                Arc::from("\x1b[0m")
            } else {
                Arc::from(right)
            }
        }),
        indicator: indicator.map(Arc::from),
        vi_insert: vi_insert.map(Arc::from),
        vi_normal: vi_normal.map(Arc::from),
        vi_visual: vi_visual.map(Arc::from),
        multiline: multiline.map(Arc::from),
        render_right_on_last_line: prompt.render_right_on_last_line,
    }
}

/// Construct the transient prompt based on the normal nu_prompt.
/// Note: Transient prompts do NOT emit semantic markers since they replace
/// the actual prompt after command execution (which already has markers).
///
/// The transient prompt is drawn only once the line is submitted, which can
/// be long after this function runs. Rather than freezing a snapshot of the
/// baseline now, we resolve only the `TRANSIENT_PROMPT_*` overrides here and
/// read the baseline live at render time, so a background job's late
/// `commandline set-prompt` pushes still show up.
pub(crate) fn make_transient_prompt(
    config: &Config,
    engine_state: &EngineState,
    stack: &Stack,
) -> Box<dyn Prompt> {
    let transient = &config.prompt.transient;
    let source = |env_var, configured| {
        resolve_prompt_source(env_var, configured, config, engine_state, stack)
    };

    // Same fixed order as the live prompt, for the same reason. `None` here
    // means "keep the live value".
    let left = source(TRANSIENT_PROMPT_COMMAND, transient.left.as_ref());
    let right = source(TRANSIENT_PROMPT_COMMAND_RIGHT, transient.right.as_ref());
    let indicator = source(TRANSIENT_PROMPT_INDICATOR, transient.indicator.as_ref());
    let vi_insert = source(
        TRANSIENT_PROMPT_INDICATOR_VI_INSERT,
        transient.vi_insert.as_ref(),
    );
    let vi_normal = source(
        TRANSIENT_PROMPT_INDICATOR_VI_NORMAL,
        transient.vi_normal.as_ref(),
    );
    // Config-only, like its live counterpart.
    let vi_visual = transient
        .vi_visual
        .as_ref()
        .and_then(|value| render(value, config, engine_state, stack));
    let multiline = source(
        TRANSIENT_PROMPT_MULTILINE_INDICATOR,
        transient.multiline.as_ref(),
    );

    let overrides = PromptContents {
        left: left.map(Arc::from),
        right: right.map(Arc::from),
        indicator: indicator.map(Arc::from),
        vi_insert: vi_insert.map(Arc::from),
        vi_normal: vi_normal.map(Arc::from),
        vi_visual: vi_visual.map(Arc::from),
        multiline: multiline.map(Arc::from),
        // Falls back to the live baseline via `overridden_by`.
        render_right_on_last_line: false,
    };

    Box::new(NushellPrompt::transient(
        engine_state.prompt_state.clone(),
        overrides,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::{Span, Value};

    #[test]
    fn update_prompt_does_not_embed_osc_markers() {
        let mut config = Config::default();
        config.shell_integration.osc133 = true;

        let engine_state = EngineState::new();
        let mut stack = Stack::new();
        stack.add_env_var(
            PROMPT_COMMAND.into(),
            Value::string("test", Span::test_data()),
        );

        update_prompt(&config, &engine_state, &stack);

        let nu_prompt = NushellPrompt::shared(engine_state.prompt_state.clone());
        assert_eq!(nu_prompt.render_prompt_left(), "test");
    }

    #[test]
    fn transient_prompt_override_still_wins_over_the_live_baseline() {
        use nu_protocol::{JobId, engine::PromptSegment};

        let config = Config::default();
        let engine_state = EngineState::new();
        let mut stack = Stack::new();
        stack.add_env_var(
            TRANSIENT_PROMPT_INDICATOR.into(),
            Value::string("transient> ", Span::test_data()),
        );

        let transient_prompt = make_transient_prompt(&config, &engine_state, &stack);

        // The configured TRANSIENT_PROMPT_INDICATOR beats a later live change.
        engine_state
            .prompt_state
            .set(PromptSegment::Indicator, "live> ", JobId::new(1));

        assert_eq!(
            transient_prompt.render_prompt_indicator(reedline::PromptEditMode::Emacs),
            "transient> "
        );
    }

    /// Renders the emacs indicator from `config`, with `env` applied first.
    fn rendered_indicator(config: &Config, env: &[(&str, &str)]) -> String {
        let engine_state = EngineState::new();
        let mut stack = Stack::new();
        for (name, val) in env {
            stack.add_env_var((*name).into(), Value::string(*val, Span::test_data()));
        }

        update_prompt(config, &engine_state, &stack);

        NushellPrompt::shared(engine_state.prompt_state.clone())
            .render_prompt_indicator(reedline::PromptEditMode::Emacs)
            .to_string()
    }

    #[test]
    fn indicator_comes_from_config_without_env_var() {
        let mut config = Config::default();
        config.prompt.indicator = Value::test_string("config> ");

        assert_eq!(rendered_indicator(&config, &[]), "config> ");
    }

    #[test]
    fn legacy_env_var_takes_precedence_over_config() {
        let mut config = Config::default();
        config.prompt.indicator = Value::test_string("config> ");

        assert_eq!(
            rendered_indicator(&config, &[(PROMPT_INDICATOR, "env> ")]),
            "env> "
        );
    }

    /// Renders the left prompt from `config`, with `env` applied first.
    fn rendered_left(config: &Config, env: &[(&str, &str)]) -> String {
        let engine_state = EngineState::new();
        let mut stack = Stack::new();
        for (name, val) in env {
            stack.add_env_var((*name).into(), Value::string(*val, Span::test_data()));
        }

        update_prompt(config, &engine_state, &stack);

        NushellPrompt::shared(engine_state.prompt_state.clone())
            .render_prompt_left()
            .to_string()
    }

    #[test]
    fn left_comes_from_config_without_env_var() {
        let mut config = Config::default();
        config.prompt.left = Some(Value::string("config> ", Span::test_data()));

        assert_eq!(rendered_left(&config, &[]), "config> ");
    }

    #[test]
    fn legacy_prompt_command_takes_precedence_over_config() {
        let mut config = Config::default();
        config.prompt.left = Some(Value::string("config> ", Span::test_data()));

        assert_eq!(
            rendered_left(&config, &[(PROMPT_COMMAND, "env> ")]),
            "env> "
        );
    }

    #[test]
    fn an_unrenderable_env_var_falls_through_to_config() {
        // Matches how the indicators behave: a variable that is set but has no
        // rendering is treated as if it were unset, rather than blanking the
        // segment.
        let mut config = Config::default();
        config.prompt.left = Some(Value::string("config> ", Span::test_data()));

        let engine_state = EngineState::new();
        let mut stack = Stack::new();
        stack.add_env_var(PROMPT_COMMAND.into(), Value::test_int(1));

        update_prompt(&config, &engine_state, &stack);

        assert_eq!(
            NushellPrompt::shared(engine_state.prompt_state.clone()).render_prompt_left(),
            "config> "
        );
    }

    #[test]
    fn vi_visual_indicator_comes_from_config() {
        // `vi_visual` is config-only, so the vi normal environment variable
        // must not leak into visual mode the way it used to.
        use reedline::{PromptEditMode, PromptViMode};

        let mut config = Config::default();
        config.prompt.vi_visual = Value::test_string("visual> ");

        let engine_state = EngineState::new();
        let mut stack = Stack::new();
        stack.add_env_var(
            PROMPT_INDICATOR_VI_NORMAL.into(),
            Value::string("normal> ", Span::test_data()),
        );

        update_prompt(&config, &engine_state, &stack);
        let prompt = NushellPrompt::shared(engine_state.prompt_state.clone());

        assert_eq!(
            prompt.render_prompt_indicator(PromptEditMode::Vi(PromptViMode::Visual)),
            "visual> "
        );
        assert_eq!(
            prompt.render_prompt_indicator(PromptEditMode::Vi(PromptViMode::Normal)),
            "normal> "
        );
    }

    /// Renders the transient emacs indicator, given a live baseline pushed
    /// after the transient prompt was built.
    fn rendered_transient_indicator(config: &Config, env: &[(&str, &str)]) -> String {
        use nu_protocol::{JobId, engine::PromptSegment};

        let engine_state = EngineState::new();
        let mut stack = Stack::new();
        for (name, val) in env {
            stack.add_env_var((*name).into(), Value::string(*val, Span::test_data()));
        }

        let transient = make_transient_prompt(config, &engine_state, &stack);
        engine_state
            .prompt_state
            .set(PromptSegment::Indicator, "live> ", JobId::new(1));

        transient
            .render_prompt_indicator(reedline::PromptEditMode::Emacs)
            .to_string()
    }

    #[test]
    fn transient_indicator_comes_from_config_without_env_var() {
        let mut config = Config::default();
        config.prompt.transient.indicator = Some(Value::test_string("transient> "));

        assert_eq!(rendered_transient_indicator(&config, &[]), "transient> ");
    }

    #[test]
    fn legacy_transient_env_var_takes_precedence_over_config() {
        let mut config = Config::default();
        config.prompt.transient.indicator = Some(Value::test_string("config> "));

        assert_eq!(
            rendered_transient_indicator(&config, &[(TRANSIENT_PROMPT_INDICATOR, "env> ")]),
            "env> "
        );
    }

    #[test]
    fn null_transient_indicator_keeps_the_live_one() {
        // The default: nothing configured for the indicator, so the transient
        // prompt shows whatever the live prompt ended up with.
        let config = Config::default();
        assert_eq!(config.prompt.transient.indicator, None);

        assert_eq!(rendered_transient_indicator(&config, &[]), "live> ");
    }

    #[test]
    fn configured_indicators_accept_closures() -> nu_test_support::Result {
        use reedline::{PromptEditMode, PromptViMode};

        // What `$env.PROMPT_INDICATOR` and friends always took, e.g. an
        // indicator colored by the last exit code.
        let mut tester = nu_test_support::test();
        let () = tester.run(
            r#"
            $env.config.prompt.indicator = {|| "closure> " }
            $env.config.prompt.vi_visual = {|| "visual> " }
            $env.config.prompt.multiline = {|| "closure::: " }
            $env.config.prompt.transient.indicator = {|| "transient> " }
            $env.config.prompt.vi_insert = {|| error make {msg: "boom"} }
            $env.config.prompt.transient.vi_visual = {|| "tvisual> " }
            "#,
        )?;
        tester.engine_state.merge_env(&mut tester.stack)?;
        let config = tester.engine_state.get_config().clone();

        update_prompt(&config, &tester.engine_state, &tester.stack);
        let prompt = NushellPrompt::shared(tester.engine_state.prompt_state.clone());
        assert_eq!(
            prompt.render_prompt_indicator(PromptEditMode::Emacs),
            "closure> "
        );
        assert_eq!(
            prompt.render_prompt_indicator(PromptEditMode::Vi(PromptViMode::Visual)),
            "visual> "
        );
        assert_eq!(prompt.render_prompt_multiline_indicator(), "closure::: ");
        // A closure that fails reports its error and leaves the built-in
        // indicator in place, rather than blanking the slot.
        assert_eq!(
            prompt.render_prompt_indicator(PromptEditMode::Vi(PromptViMode::Insert)),
            "> "
        );

        let transient = make_transient_prompt(&config, &tester.engine_state, &tester.stack);
        assert_eq!(
            transient.render_prompt_indicator(PromptEditMode::Emacs),
            "transient> "
        );
        // `vi_visual` has no environment variable, so it renders on its own
        // path instead of through `resolve_prompt_source`.
        assert_eq!(
            transient.render_prompt_indicator(PromptEditMode::Vi(PromptViMode::Visual)),
            "tvisual> "
        );
        Ok(())
    }
}
