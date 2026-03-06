use nu_test_support::{Outcome, nu};
use tempfile::TempDir;

struct Test {
    cfg_dir: TempDir,
}

const IMPORT_SINGLE_HISTORY_RECORD: &str = "[[command start_timestamp duration exit_status cwd]; ['echo hi' (date now) 30ms 0 /tmp]] | history import";
const IMPORT_THREE_HISTORY_RECORDS: &str = "[[command start_timestamp duration exit_status cwd]; ['echo one' (date now) 10ms 0 /tmp] ['echo two' (date now) 20ms 0 /tmp] ['echo three' (date now) 30ms 0 /tmp]] | history import";

impl Test {
    fn new() -> Self {
        let cfg_dir = tempfile::Builder::new()
            .prefix("history_output_test")
            .tempdir()
            .unwrap();
        std::fs::write(
            cfg_dir.path().join("env.nu"),
            "$env.config.history.file_format = 'sqlite'",
        )
        .unwrap();
        Self { cfg_dir }
    }

    fn nu(&self, cmd: impl AsRef<str>) -> Outcome {
        let env = [(
            "XDG_CONFIG_HOME".to_string(),
            self.cfg_dir.path().to_str().unwrap().to_string(),
        )];
        let env_config = self.cfg_dir.path().join("env.nu");
        nu!(envs: env, env_config: env_config, cmd.as_ref())
    }

    fn import_single_history_record(&self) -> Outcome {
        self.nu(IMPORT_SINGLE_HISTORY_RECORD)
    }

    fn import_single_history_record_and_assert_success(&self) {
        let import_result = self.import_single_history_record();
        assert!(import_result.status.success(), "{}", import_result.err);
    }
}

#[test]
fn sqlite_history_last_returns_date_for_start_timestamp() {
    let test = Test::new();
    test.import_single_history_record_and_assert_success();

    let actual = test.nu("history | last | get start_timestamp | describe");
    assert_eq!(actual.out, "datetime");
}

#[test]
fn sqlite_history_last_returns_duration_for_duration_column() {
    let test = Test::new();
    test.import_single_history_record_and_assert_success();

    let actual = test.nu("history | last | get duration | describe");
    assert_eq!(actual.out, "duration");
}

#[test]
fn sqlite_history_select_command_works() {
    let test = Test::new();
    test.import_single_history_record_and_assert_success();

    let actual = test.nu("history | select command | columns | first");
    assert!(actual.status.success(), "{}", actual.err);
    assert_eq!(actual.out, "command");
}

#[test]
fn sqlite_history_select_projection_preserves_order() {
    let test = Test::new();
    let import_result = test.nu(IMPORT_THREE_HISTORY_RECORDS);
    assert!(import_result.status.success(), "{}", import_result.err);

    let command_only = test.nu("history | where command =~ 'echo (one|two|three)' | select command | get command | to nuon");
    assert!(command_only.status.success(), "{}", command_only.err);

    let with_timestamp = test.nu("history | where command =~ 'echo (one|two|three)' | select start_timestamp command | get command | to nuon");
    assert!(with_timestamp.status.success(), "{}", with_timestamp.err);

    assert_eq!(command_only.out, with_timestamp.out);
}
