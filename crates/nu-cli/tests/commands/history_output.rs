use nu_test_support::{Outcome, nu};
use tempfile::TempDir;

struct Test {
    cfg_dir: TempDir,
}

const IMPORT_SINGLE_HISTORY_RECORD: &str = "[[command start_timestamp duration exit_status cwd]; [echo (date now) 30ms 0 /tmp]] | history import";

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
}

#[test]
fn sqlite_history_last_returns_date_for_start_timestamp() {
    let test = Test::new();

    let import_result = test.import_single_history_record();
    assert!(import_result.status.success(), "{}", import_result.err);

    let actual = test.nu("history | last | get start_timestamp | describe");
    assert_eq!(actual.out, "datetime");
}

#[test]
fn sqlite_history_last_returns_duration_for_duration_column() {
    let test = Test::new();

    let import_result = test.import_single_history_record();
    assert!(import_result.status.success(), "{}", import_result.err);

    let actual = test.nu("history | last | get duration | describe");
    assert_eq!(actual.out, "duration");
}
