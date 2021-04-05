#[cfg(test)]
mod test {
    use nu_test_support::fs::Stub::FileWithContent;
    use serial_test::serial;

    use nu_engine::{filesystem::filesystem_shell::FilesystemShellMode, script::run_script};
    use nu_protocol::{NuScript, RunScriptOptions};
    use nu_test_support::playground::Playground;

    use nu_command::commands::{
        Append, Autoenv, AutoenvTrust, AutoenvUnTrust, Autoview, BuildString, Cd, Def, Each, Echo,
        First, Get, Keep, Last, Let, Ls, Nth, RunExternalCommand, Select, StrCollect, Touch, Wrap,
    };
    use nu_engine::basic_evaluation_context;
    use nu_engine::{whole_stream_command, EvaluationContext};

    pub fn get_test_context() -> EvaluationContext {
        let base_context = basic_evaluation_context(FilesystemShellMode::Cli)
            .expect("Could not create test context");

        base_context.add_commands(vec![
            // Minimal restricted commands to aid in testing
            whole_stream_command(Echo {}),
            whole_stream_command(Def {}),
            whole_stream_command(Autoview {}),
            whole_stream_command(RunExternalCommand { interactive: true }),
            whole_stream_command(Ls {}),
            whole_stream_command(Autoenv {}),
            whole_stream_command(AutoenvTrust {}),
            whole_stream_command(AutoenvUnTrust {}),
            whole_stream_command(Touch {}),
            whole_stream_command(Cd {}),
            whole_stream_command(Append {}),
            whole_stream_command(BuildString {}),
            whole_stream_command(First {}),
            whole_stream_command(Get {}),
            whole_stream_command(Keep {}),
            whole_stream_command(Each {}),
            whole_stream_command(Last {}),
            whole_stream_command(Nth {}),
            whole_stream_command(Let {}),
            whole_stream_command(Select),
            whole_stream_command(StrCollect),
            whole_stream_command(Wrap),
        ]);

        base_context
    }

    #[test]
    #[serial]
    fn sourcing_script_executing_cd_changes_path() {
        Playground::setup("run_script_test_1", |dirs, sandbox| {
            assert!(std::env::set_current_dir(dirs.test()).is_ok());
            sandbox.mkdir("foo");
            let code = "cd foo";
            let context = get_test_context();
            let options = RunScriptOptions::default()
                .source_script(true)
                .with_cwd(dirs.test.clone());
            let script = NuScript::Content(code.to_string());

            futures::executor::block_on(run_script(script, &options, &context));
            assert!(context.shell_manager.path().ends_with("foo"));
        });
    }

    #[test]
    #[serial]
    fn sourcing_script_picks_up_defs() {
        Playground::setup("run_script_test_2", |dirs, sandbox| {
            assert!(std::env::set_current_dir(dirs.test()).is_ok());
            sandbox.mkdir("foo");
            let code = "def hi [] {}";
            let context = get_test_context();
            let options = RunScriptOptions::default().source_script(true);
            let script = NuScript::Content(code.to_string());

            futures::executor::block_on(run_script(script, &options, &context));
            assert!(context.scope.has_custom_command("hi"));
        });
    }

    #[test]
    #[serial]
    fn not_sourcing_script_does_not_leave_defs_in_scope() {
        Playground::setup("run_script_test_3", |dirs, sandbox| {
            assert!(std::env::set_current_dir(dirs.test()).is_ok());
            sandbox.mkdir("foo");
            let code = "def hi [] {}";
            let context = get_test_context();
            let options = RunScriptOptions::default();
            let script = NuScript::Content(code.to_string());

            futures::executor::block_on(run_script(script, &options, &context));
            assert!(!context.scope.has_custom_command("hi"));
        });
    }

    #[test]
    #[serial]
    fn not_sourcing_script_executing_cd_does_not_change_path() {
        Playground::setup("run_script_test_4", |dirs, sandbox| {
            assert!(std::env::set_current_dir(dirs.test()).is_ok());
            sandbox.mkdir("foo");
            let code = "cd foo";
            let context = get_test_context();
            let options = RunScriptOptions::default().with_cwd(dirs.test.clone());
            let script = NuScript::Content(code.to_string());

            futures::executor::block_on(run_script(script, &options, &context));
            assert!(!context.shell_manager.path().ends_with("foo"));
        });
    }

    #[test]
    #[serial]
    fn running_script_with_cwd_works() {
        Playground::setup("run_script_test_5", |dirs, sandbox| {
            // sandbox.within("foo");
            sandbox.mkdir("bar");
            assert!(std::env::set_current_dir(dirs.test()).is_ok());
            let code = "touch cuz";
            let context = get_test_context();
            let options = RunScriptOptions::default().with_cwd(dirs.test.join("bar"));
            // .source_script(true);
            let script = NuScript::Content(code.to_string());

            futures::executor::block_on(run_script(script, &options, &context));

            assert!(dirs.test.join("bar/cuz").exists());
        });
    }

    #[test]
    #[serial]
    fn run_script_picks_up_nu_env_in_cli_mode() {
        Playground::setup("run_script_test_6", |dirs, sandbox| {
            assert!(std::env::set_current_dir(dirs.test()).is_ok());
            sandbox.mkdir("foo");
            sandbox.with_files(vec![FileWithContent(
                "foo/.nu-env",
                r#"
                startup = ["touch bar"]
            "#,
            )]);

            let code = "autoenv trust foo; cd foo; autoenv untrust .";
            let context = get_test_context();
            let options = RunScriptOptions::default()
                .cli_mode(true)
                .with_cwd(dirs.test.clone());

            let script = NuScript::Content(code.to_string());

            futures::executor::block_on(run_script(script, &options, &context));

            assert!(dirs.test.join("foo").join("bar").exists());
            assert!(!dirs.test.join("bar").exists());
        });
    }

    #[test]
    #[serial]
    fn run_script_does_not_pick_up_nu_env_if_set_into_nu_env_dir_by_with_cwd() {
        //TODO I am not sure, what the most desired behaviour for this case would be
        // Currently we don't load the .nu-env in this case
        // However this slightly differs from the "normal cli" interaction (where
        // the nu-env in the dir you start gets sourced...)
        //
        // But automatically loading nu-env's can be surprising...
        Playground::setup("run_script_test_7", |dirs, sandbox| {
            assert!(std::env::set_current_dir(dirs.test()).is_ok());
            sandbox.with_files(vec![FileWithContent(
                ".nu-env",
                r#"
                startup = ["touch bar"]
            "#,
            )]);
            let trust = "autoenv trust .";
            let code = "echo hi";
            let untrust = "autoenv untrust .";
            let context = get_test_context();
            let options = RunScriptOptions::default()
                .cli_mode(true)
                .with_cwd(dirs.test.clone());
            let trust = NuScript::Content(trust.to_string());
            let script = NuScript::Content(code.to_string());
            let untrust = NuScript::Content(untrust.to_string());

            futures::executor::block_on(run_script(trust, &options, &context));
            futures::executor::block_on(run_script(script, &options, &context));
            futures::executor::block_on(run_script(untrust, &options, &context));

            assert!(!dirs.test.join("bar").exists());
        });
    }
}
