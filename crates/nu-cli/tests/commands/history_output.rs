use std::path::Path;

use nu_protocol::{Config, HistoryConfig, HistoryFileFormat};
use nu_test_support::prelude::*;
use nu_test_support::tester::NuTester;
use reedline::{History, HistoryItem, SqliteBackedHistory};

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
fn sqlite_history_last_returns_date_for_start_timestamp() -> Result {
    Playground::setup("sqlite_history_last_returns_date", |dirs, _| {
        let config_home = dirs.test().join("nushell").to_std_path_buf();
        let mut tester = test().with_sqlite_history(config_home);
        let () = tester.run(IMPORT_SINGLE_HISTORY_RECORD)?;

        tester
            .run("history | last | get start_timestamp | describe")
            .expect_value_eq("datetime")
    })
}

#[test]
fn sqlite_history_last_returns_duration_for_duration_column() -> Result {
    Playground::setup("sqlite_history_last_returns_duration", |dirs, _| {
        let config_home = dirs.test().join("nushell").to_std_path_buf();
        let mut tester = test().with_sqlite_history(config_home);
        let () = tester.run(IMPORT_SINGLE_HISTORY_RECORD)?;

        tester
            .run("history | last | get duration | describe")
            .expect_value_eq("duration")
    })
}

#[test]
fn sqlite_history_select_command_works() -> Result {
    Playground::setup("sqlite_history_select_command_works", |dirs, _| {
        let config_home = dirs.test().join("nushell").to_std_path_buf();
        let mut tester = test().with_sqlite_history(config_home);
        let () = tester.run(IMPORT_SINGLE_HISTORY_RECORD)?;

        tester
            .run("history | select command | columns | first")
            .expect_value_eq("command")
    })
}

#[test]
fn sqlite_history_select_projection_preserves_order() -> Result {
    Playground::setup("sqlite_history_select_projection_order", |dirs, _| {
        let config_home = dirs.test().join("nushell").to_std_path_buf();
        let mut tester = test().with_sqlite_history(config_home);
        let () = tester.run(IMPORT_THREE_HISTORY_RECORDS)?;

        let command_only: Vec<String> = tester.run(
            "history | where command =~ 'echo (one|two|three)' | select command | get command",
        )?;

        let with_timestamp: Vec<String> = tester.run(
            "history | where command =~ 'echo (one|two|three)' | select start_timestamp command | get command",
        )?;

        assert_eq!(command_only, with_timestamp);
        Ok(())
    })
}

fn sqlite_history_path(config_home: &Path) -> std::path::PathBuf {
    config_home.join("history.sqlite3")
}

#[test]
fn sqlite_history_clear_removes_all_entries() -> Result {
    Playground::setup("sqlite_history_clear_removes_all_entries", |dirs, _| {
        let config_home = dirs.test().join("nushell").to_std_path_buf();
        let mut tester = test().with_sqlite_history(&config_home);
        let () = tester.run(IMPORT_THREE_HISTORY_RECORDS)?;

        let () = tester.run("history --clear")?;

        tester.run("history | length").expect_value_eq(0)
    })
}

#[test]
fn sqlite_history_clear_keeps_database_usable() -> Result {
    Playground::setup("sqlite_history_clear_keeps_database_usable", |dirs, _| {
        let config_home = dirs.test().join("nushell").to_std_path_buf();
        let history_path = sqlite_history_path(&config_home);
        let mut tester = test().with_sqlite_history(&config_home);
        let () = tester.run(IMPORT_THREE_HISTORY_RECORDS)?;

        // Stands in for the REPL, which keeps its own connection to the history
        // database open while `history --clear` runs.
        let mut live = SqliteBackedHistory::with_file(history_path.clone(), None, None)
            .expect("open live history connection");

        let () = tester.run("history --clear")?;

        // The live connection must still write to the same database.
        live.save(HistoryItem::from_command_line("echo after clear"))
            .expect("save through live connection");

        // A new session must be able to open the database and add entries.
        let mut fresh = SqliteBackedHistory::with_file(history_path, None, None)
            .expect("reopen history after clear");
        fresh
            .save(HistoryItem::from_command_line("echo fresh session"))
            .expect("save through fresh connection");

        tester
            .run("history | get command")
            .expect_value_eq(["echo after clear", "echo fresh session"])
    })
}
