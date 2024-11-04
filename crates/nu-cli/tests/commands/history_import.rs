use nu_protocol::HistoryFileFormat;
use nu_test_support::{nu, Outcome};
use reedline::{
    FileBackedHistory, History, HistoryItem, HistoryItemId, ReedlineError, SearchQuery,
    SqliteBackedHistory,
};
use rstest::rstest;
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

    fn nu(&self, cmd: impl AsRef<str>) -> Outcome {
        let env = [(
            "XDG_CONFIG_HOME".to_string(),
            self.cfg_dir.path().to_str().unwrap().to_string(),
        )];
        let env_config = self.cfg_dir.path().join("env.nu");
        nu!(envs: env, env_config: env_config, cmd.as_ref())
    }

    fn open_plaintext(&self) -> Result<FileBackedHistory, ReedlineError> {
        FileBackedHistory::with_file(
            100,
            self.cfg_dir
                .path()
                .join("nushell")
                .join(HistoryFileFormat::Plaintext.default_file_name()),
        )
    }

    fn open_sqlite(&self) -> Result<SqliteBackedHistory, ReedlineError> {
        SqliteBackedHistory::with_file(
            self.cfg_dir
                .path()
                .join("nushell")
                .join(HistoryFileFormat::Sqlite.default_file_name()),
            None,
            None,
        )
    }

    fn open_backend(&self, format: HistoryFileFormat) -> Result<Box<dyn History>, ReedlineError> {
        fn boxed(be: impl History + 'static) -> Box<dyn History> {
            Box::new(be)
        }
        use HistoryFileFormat::*;
        match format {
            Plaintext => self.open_plaintext().map(boxed),
            Sqlite => self.open_sqlite().map(boxed),
        }
    }
}

enum HistorySource {
    Vec(Vec<HistoryItem>),
    Command(&'static str),
}

struct TestCase {
    dst_format: HistoryFileFormat,
    dst_history: Vec<HistoryItem>,
    src_history: HistorySource,
    want_history: Vec<HistoryItem>,
}

const EMPTY_TEST_CASE: TestCase = TestCase {
    dst_format: HistoryFileFormat::Plaintext,
    dst_history: Vec::new(),
    src_history: HistorySource::Vec(Vec::new()),
    want_history: Vec::new(),
};

impl TestCase {
    fn run(self) {
        use HistoryFileFormat::*;
        let test = Test::new(match self.dst_format {
            Plaintext => "plaintext",
            Sqlite => "sqlite",
        });
        save_all(
            &mut *test.open_backend(self.dst_format).unwrap(),
            self.dst_history,
        )
        .unwrap();

        let outcome = match self.src_history {
            HistorySource::Vec(src_history) => {
                let src_format = match self.dst_format {
                    Plaintext => Sqlite,
                    Sqlite => Plaintext,
                };
                save_all(&mut *test.open_backend(src_format).unwrap(), src_history).unwrap();
                test.nu("history import")
            }
            HistorySource::Command(cmd) => {
                let mut cmd = cmd.to_string();
                cmd.push_str(" | history import");
                test.nu(cmd)
            }
        };
        assert!(outcome.status.success());
        let got = query_all(&*test.open_backend(self.dst_format).unwrap()).unwrap();

        // Compare just the commands first, for readability.
        fn commands_only(items: &[HistoryItem]) -> Vec<&str> {
            items
                .iter()
                .map(|item| item.command_line.as_str())
                .collect()
        }
        assert_eq!(commands_only(&got), commands_only(&self.want_history));
        // If commands match, compare full items.
        assert_eq!(got, self.want_history);
    }
}

fn query_all(history: &dyn History) -> Result<Vec<HistoryItem>, ReedlineError> {
    history.search(SearchQuery::everything(
        reedline::SearchDirection::Forward,
        None,
    ))
}

fn save_all(history: &mut dyn History, items: Vec<HistoryItem>) -> Result<(), ReedlineError> {
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

#[test]
fn history_import_pipe_string() {
    TestCase {
        dst_format: HistoryFileFormat::Plaintext,
        src_history: HistorySource::Command("echo bar"),
        want_history: vec![HistoryItem {
            id: Some(HistoryItemId::new(0)),
            command_line: "bar".to_string(),
            ..EMPTY_ITEM
        }],
        ..EMPTY_TEST_CASE
    }
    .run();
}

#[test]
fn history_import_pipe_record() {
    TestCase {
        dst_format: HistoryFileFormat::Sqlite,
        src_history: HistorySource::Command("[[cwd command]; [/tmp some_command]]"),
        want_history: vec![HistoryItem {
            id: Some(HistoryItemId::new(1)),
            command_line: "some_command".to_string(),
            cwd: Some("/tmp".to_string()),
            ..EMPTY_ITEM
        }],
        ..EMPTY_TEST_CASE
    }
    .run();
}

#[test]
fn to_empty_plaintext() {
    TestCase {
        dst_format: HistoryFileFormat::Plaintext,
        src_history: HistorySource::Vec(vec![
            HistoryItem {
                command_line: "foo".to_string(),
                ..EMPTY_ITEM
            },
            HistoryItem {
                command_line: "bar".to_string(),
                ..EMPTY_ITEM
            },
        ]),
        want_history: vec![
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
        ..EMPTY_TEST_CASE
    }
    .run()
}

#[test]
fn to_empty_sqlite() {
    TestCase {
        dst_format: HistoryFileFormat::Sqlite,
        src_history: HistorySource::Vec(vec![
            HistoryItem {
                command_line: "foo".to_string(),
                ..EMPTY_ITEM
            },
            HistoryItem {
                command_line: "bar".to_string(),
                ..EMPTY_ITEM
            },
        ]),
        want_history: vec![
            HistoryItem {
                id: Some(HistoryItemId::new(1)),
                command_line: "foo".to_string(),
                ..EMPTY_ITEM
            },
            HistoryItem {
                id: Some(HistoryItemId::new(2)),
                command_line: "bar".to_string(),
                ..EMPTY_ITEM
            },
        ],
        ..EMPTY_TEST_CASE
    }
    .run()
}

#[rstest]
#[case::plaintext(HistoryFileFormat::Plaintext)]
#[case::sqlite(HistoryFileFormat::Sqlite)]
fn to_existing(#[case] dst_format: HistoryFileFormat) {
    TestCase {
        dst_format,
        dst_history: vec![
            HistoryItem {
                id: Some(HistoryItemId::new(0)),
                command_line: "original-1".to_string(),
                ..EMPTY_ITEM
            },
            HistoryItem {
                id: Some(HistoryItemId::new(1)),
                command_line: "original-2".to_string(),
                ..EMPTY_ITEM
            },
        ],
        src_history: HistorySource::Vec(vec![HistoryItem {
            id: Some(HistoryItemId::new(1)),
            command_line: "new".to_string(),
            ..EMPTY_ITEM
        }]),
        want_history: vec![
            HistoryItem {
                id: Some(HistoryItemId::new(0)),
                command_line: "original-1".to_string(),
                ..EMPTY_ITEM
            },
            HistoryItem {
                id: Some(HistoryItemId::new(1)),
                command_line: "original-2".to_string(),
                ..EMPTY_ITEM
            },
            HistoryItem {
                id: Some(HistoryItemId::new(2)),
                command_line: "new".to_string(),
                ..EMPTY_ITEM
            },
        ],
    }
    .run()
}
