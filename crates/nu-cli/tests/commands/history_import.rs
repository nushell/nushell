use nu_test_support::{nu, Outcome};
use reedline::{
    FileBackedHistory, History, HistoryItem, HistoryItemId, ReedlineError, SearchQuery,
    SqliteBackedHistory,
};
use tempfile::TempDir;

struct Test {
    cfg_dir: TempDir,
}

impl Test {
    fn new(history_format: &'static str) -> Self {
        let cfg_dir = tempfile::Builder::new()
            .prefix("history_import_test")
            .tempdir()
            .unwrap();
        // Assigning to $env.config.history.file_format seems to work only in startup
        // configuration.
        std::fs::write(
            cfg_dir.path().join("env.nu"),
            format!("$env.config.history.file_format = {history_format:?}"),
        )
        .unwrap();
        Self { cfg_dir }
    }

    fn nu(&self, cmd: &'static str) -> Outcome {
        let env = [(
            "XDG_CONFIG_HOME".to_string(),
            self.cfg_dir.path().to_str().unwrap().to_string(),
        )];
        let env_config = self.cfg_dir.path().join("env.nu");
        nu!(envs: env, env_config: env_config, cmd)
    }

    fn open_plaintext(&self) -> Result<FileBackedHistory, ReedlineError> {
        FileBackedHistory::with_file(100, self.cfg_dir.path().join("nushell").join("history.txt"))
    }

    fn open_sqlite(&self) -> Result<SqliteBackedHistory, ReedlineError> {
        SqliteBackedHistory::with_file(
            self.cfg_dir.path().join("nushell").join("history.sqlite3"),
            None,
            None,
        )
    }
}

fn query_all(history: impl History) -> Result<Vec<HistoryItem>, ReedlineError> {
    history.search(SearchQuery::everything(
        reedline::SearchDirection::Forward,
        None,
    ))
}

fn save_all(mut history: impl History, items: Vec<HistoryItem>) -> Result<(), ReedlineError> {
    for item in items {
        history.save(item)?;
    }
    Ok(())
}

const EMPTY_ITEM: HistoryItem = HistoryItem {
    command_line: String::new(),
    id: None,
    start_timestamp: None,
    session_id: None,
    hostname: None,
    cwd: None,
    duration: None,
    exit_status: None,
    more_info: None,
};

#[ignore]
#[test]
fn history_import_pipe_string() {
    let test = Test::new("plaintext");
    let outcome = test.nu("echo bar | history import");

    assert!(outcome.status.success());
    assert_eq!(
        query_all(test.open_plaintext().unwrap()).unwrap(),
        vec![HistoryItem {
            id: Some(HistoryItemId::new(0)),
            command_line: "bar".to_string(),
            ..EMPTY_ITEM
        }]
    );
}

#[ignore]
#[test]
fn history_import_pipe_record() {
    let test = Test::new("sqlite");
    let outcome = test.nu("[[item_id command]; [42 some_command]] | history import");

    assert!(outcome.status.success());
    assert_eq!(
        query_all(test.open_sqlite().unwrap()).unwrap(),
        vec![HistoryItem {
            id: Some(HistoryItemId::new(42)),
            command_line: "some_command".to_string(),
            ..EMPTY_ITEM
        }]
    );
}

#[ignore]
#[test]
fn history_import_plain_to_sqlite() {
    let test = Test::new("sqlite");
    save_all(
        test.open_plaintext().unwrap(),
        vec![
            HistoryItem {
                id: Some(HistoryItemId::new(0)),
                command_line: "foo".to_string(),
                ..EMPTY_ITEM
            },
            HistoryItem {
                id: Some(HistoryItemId::new(1)),
                command_line: "bar".to_string(),
                ..EMPTY_ITEM
            },
        ],
    )
    .unwrap();

    let outcome = test.nu("history import");
    assert!(outcome.status.success());
    assert_eq!(
        query_all(test.open_sqlite().unwrap()).unwrap(),
        vec![
            HistoryItem {
                id: Some(HistoryItemId::new(0)),
                command_line: "foo".to_string(),
                ..EMPTY_ITEM
            },
            HistoryItem {
                id: Some(HistoryItemId::new(1)),
                command_line: "bar".to_string(),
                ..EMPTY_ITEM
            }
        ]
    );
}

#[ignore]
#[test]
fn history_import_sqlite_to_plain() {
    let test = Test::new("plaintext");
    save_all(
        test.open_sqlite().unwrap(),
        vec![
            HistoryItem {
                id: Some(HistoryItemId::new(0)),
                command_line: "foo".to_string(),
                hostname: Some("host".to_string()),
                ..EMPTY_ITEM
            },
            HistoryItem {
                id: Some(HistoryItemId::new(1)),
                command_line: "bar".to_string(),
                cwd: Some("/home/test".to_string()),
                ..EMPTY_ITEM
            },
        ],
    )
    .unwrap();

    let outcome = test.nu("history import");
    assert!(outcome.status.success());
    assert_eq!(
        query_all(test.open_plaintext().unwrap()).unwrap(),
        vec![
            HistoryItem {
                id: Some(HistoryItemId::new(0)),
                command_line: "foo".to_string(),
                ..EMPTY_ITEM
            },
            HistoryItem {
                id: Some(HistoryItemId::new(1)),
                command_line: "bar".to_string(),
                ..EMPTY_ITEM
            },
        ]
    );
}
