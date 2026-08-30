use std::path::Path;

use nu_protocol::{Config, HistoryConfig, HistoryFileFormat};
use nu_test_support::prelude::*;
use nu_test_support::tester::NuTester;

const IMPORT_SINGLE_HISTORY_RECORD: &str = "[[command start_timestamp duration exit_status cwd]; ['echo hi' (date now) 30ms 0 /tmp]] | history import";
const IMPORT_THREE_HISTORY_RECORDS: &str = "[[command start_timestamp duration exit_status cwd]; ['echo one' (date now) 10ms 0 /tmp] ['echo two' (date now) 20ms 0 /tmp] ['echo three' (date now) 30ms 0 /tmp]] | history import";

trait NuTesterHistoryExt {
    fn with_sqlite_history(self, config_home: impl AsRef<Path>) -> Self;
}

impl NuTesterHistoryExt for NuTester {
    fn with_sqlite_history(mut self, config_home: impl AsRef<Path>) -> Self {
        let config_home = config_home.as_ref().to_path_buf();
        std::fs::create_dir_all(&config_home).unwrap();

        self.engine_state.config_dirs.config_home = config_home;
        self.engine_state.set_config(Config {
            history: HistoryConfig {
                file_format: HistoryFileFormat::Sqlite,
                ..Default::default()
            },
            ..Default::default()
        });
        self.engine_state.generate_nu_constant();
        self
    }
}

#[test]
fn sqlite_history_last_returns_date_for_start_timestamp(playground: Playground) -> Result {
    let config_home = playground.path().join("nushell");
    let mut tester = test().with_sqlite_history(config_home);
    let () = tester.run(IMPORT_SINGLE_HISTORY_RECORD)?;

    tester
        .run("history | last | get start_timestamp | describe")
        .expect_value_eq("datetime")
}

#[test]
fn sqlite_history_last_returns_duration_for_duration_column(playground: Playground) -> Result {
    let config_home = playground.path().join("nushell");
    let mut tester = test().with_sqlite_history(config_home);
    let () = tester.run(IMPORT_SINGLE_HISTORY_RECORD)?;

    tester
        .run("history | last | get duration | describe")
        .expect_value_eq("duration")
}

#[test]
fn sqlite_history_select_command_works(playground: Playground) -> Result {
    let config_home = playground.path().join("nushell");
    let mut tester = test().with_sqlite_history(config_home);
    let () = tester.run(IMPORT_SINGLE_HISTORY_RECORD)?;

    tester
        .run("history | select command | columns | first")
        .expect_value_eq("command")
}

#[test]
fn sqlite_history_select_projection_preserves_order(playground: Playground) -> Result {
    let config_home = playground.path().join("nushell");
    let mut tester = test().with_sqlite_history(config_home);
    let () = tester.run(IMPORT_THREE_HISTORY_RECORDS)?;

    let command_only: Vec<String> = tester
        .run("history | where command =~ 'echo (one|two|three)' | select command | get command")?;

    let with_timestamp: Vec<String> = tester.run(
        "history | where command =~ 'echo (one|two|three)' | select start_timestamp command | get command",
    )?;

    assert_eq!(command_only, with_timestamp);
    Ok(())
}
