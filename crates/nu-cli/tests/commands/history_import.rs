use std::path::Path;

use nu_protocol::{Config, HistoryConfig, HistoryFileFormat};
use nu_test_support::prelude::*;
use nu_test_support::tester::NuTester;
use reedline::{
    FileBackedHistory, History, HistoryItem, HistoryItemId, ReedlineError, SearchQuery,
    SqliteBackedHistory,
};
use rstest::rstest;

trait NuTesterHistoryExt {
    fn with_history(self, config_home: impl AsRef<Path>, format: HistoryFileFormat) -> Self;
}

impl NuTesterHistoryExt for NuTester {
    fn with_history(mut self, config_home: impl AsRef<Path>, format: HistoryFileFormat) -> Self {
        let config_home = config_home.as_ref().to_path_buf();
        std::fs::create_dir_all(&config_home).unwrap();

        self.engine_state.config_dirs.config_home = config_home;
        self.engine_state.set_config(Config {
            history: HistoryConfig {
                file_format: format,
                ..Default::default()
            },
            ..Default::default()
        });
        self.engine_state.generate_nu_constant();
        self
    }
}

fn open_backend(
    config_home: &Path,
    format: HistoryFileFormat,
) -> Result<Box<dyn History>, ReedlineError> {
    fn boxed(be: impl History + 'static) -> Box<dyn History> {
        Box::new(be)
    }

    match format {
        HistoryFileFormat::Plaintext => FileBackedHistory::with_file(
            100,
            config_home.join(HistoryFileFormat::Plaintext.default_file_name()),
        )
        .map(boxed),
        HistoryFileFormat::Sqlite => SqliteBackedHistory::with_file(
            config_home.join(HistoryFileFormat::Sqlite.default_file_name()),
            None,
            None,
        )
        .map(boxed),
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

fn commands_only(items: &[HistoryItem]) -> impl Iterator<Item = &str> {
    items
        .iter()
        .map(|item| item.command_line.as_str())
}

#[test]
fn history_import_pipe_string() -> Result {
    Playground::setup("history_import_pipe_string", |dirs, _| {
        let config_home = dirs.test().join("nushell").to_std_path_buf();

        let () = test()
            .with_history(&config_home, HistoryFileFormat::Plaintext)
            .run("echo bar | history import")?;

        let got =
            query_all(&*open_backend(&config_home, HistoryFileFormat::Plaintext).unwrap()).unwrap();
        let want_history = vec![HistoryItem {
            id: Some(HistoryItemId::new(0)),
            command_line: "bar".to_string(),
            ..HistoryItem::EMPTY
        }];

        assert_eq!(commands_only(&got), commands_only(&want_history));
        assert_eq!(got, want_history);
        Ok(())
    })
}

#[test]
fn history_import_pipe_record() -> Result {
    Playground::setup("history_import_pipe_record", |dirs, _| {
        let config_home = dirs.test().join("nushell").to_std_path_buf();

        let () = test()
            .with_history(&config_home, HistoryFileFormat::Sqlite)
            .run("[[cwd command]; [/tmp some_command]] | history import")?;

        let got =
            query_all(&*open_backend(&config_home, HistoryFileFormat::Sqlite).unwrap()).unwrap();
        let want_history = vec![HistoryItem {
            id: Some(HistoryItemId::new(1)),
            command_line: "some_command".to_string(),
            cwd: Some("/tmp".to_string()),
            ..HistoryItem::EMPTY
        }];

        assert_eq!(commands_only(&got), commands_only(&want_history));
        assert_eq!(got, want_history);
        Ok(())
    })
}

#[test]
fn to_empty_plaintext() -> Result {
    Playground::setup("history_import_to_empty_plaintext", |dirs, _| {
        let config_home = dirs.test().join("nushell").to_std_path_buf();
        save_all(
            &mut *open_backend(&config_home, HistoryFileFormat::Sqlite).unwrap(),
            vec![
                HistoryItem {
                    command_line: "foo".to_string(),
                    ..HistoryItem::EMPTY
                },
                HistoryItem {
                    command_line: "bar".to_string(),
                    ..HistoryItem::EMPTY
                },
            ],
        )
        .unwrap();

        let () = test()
            .with_history(&config_home, HistoryFileFormat::Plaintext)
            .run("history import")?;

        let got =
            query_all(&*open_backend(&config_home, HistoryFileFormat::Plaintext).unwrap()).unwrap();
        let want_history = vec![
            HistoryItem {
                id: Some(HistoryItemId::new(0)),
                command_line: "foo".to_string(),
                ..HistoryItem::EMPTY
            },
            HistoryItem {
                id: Some(HistoryItemId::new(1)),
                command_line: "bar".to_string(),
                ..HistoryItem::EMPTY
            },
        ];

        assert_eq!(commands_only(&got), commands_only(&want_history));
        assert_eq!(got, want_history);
        Ok(())
    })
}

#[test]
fn to_empty_sqlite() -> Result {
    Playground::setup("history_import_to_empty_sqlite", |dirs, _| {
        let config_home = dirs.test().join("nushell").to_std_path_buf();
        save_all(
            &mut *open_backend(&config_home, HistoryFileFormat::Plaintext).unwrap(),
            vec![
                HistoryItem {
                    command_line: "foo".to_string(),
                    ..HistoryItem::EMPTY
                },
                HistoryItem {
                    command_line: "bar".to_string(),
                    ..HistoryItem::EMPTY
                },
            ],
        )
        .unwrap();

        let () = test()
            .with_history(&config_home, HistoryFileFormat::Sqlite)
            .run("history import")?;

        let got =
            query_all(&*open_backend(&config_home, HistoryFileFormat::Sqlite).unwrap()).unwrap();
        let want_history = vec![
            HistoryItem {
                id: Some(HistoryItemId::new(1)),
                command_line: "foo".to_string(),
                ..HistoryItem::EMPTY
            },
            HistoryItem {
                id: Some(HistoryItemId::new(2)),
                command_line: "bar".to_string(),
                ..HistoryItem::EMPTY
            },
        ];

        assert_eq!(commands_only(&got), commands_only(&want_history));
        assert_eq!(got, want_history);
        Ok(())
    })
}

#[rstest]
#[case::plaintext(HistoryFileFormat::Plaintext)]
#[case::sqlite(HistoryFileFormat::Sqlite)]
fn to_existing(#[case] dst_format: HistoryFileFormat) -> Result {
    Playground::setup("history_import_to_existing", |dirs, _| {
        let config_home = dirs.test().join("nushell").to_std_path_buf();

        save_all(
            &mut *open_backend(&config_home, dst_format).unwrap(),
            vec![
                HistoryItem {
                    id: Some(HistoryItemId::new(0)),
                    command_line: "original-1".to_string(),
                    ..HistoryItem::EMPTY
                },
                HistoryItem {
                    id: Some(HistoryItemId::new(1)),
                    command_line: "original-2".to_string(),
                    ..HistoryItem::EMPTY
                },
            ],
        )
        .unwrap();

        let src_format = match dst_format {
            HistoryFileFormat::Plaintext => HistoryFileFormat::Sqlite,
            HistoryFileFormat::Sqlite => HistoryFileFormat::Plaintext,
        };
        save_all(
            &mut *open_backend(&config_home, src_format).unwrap(),
            vec![HistoryItem {
                id: Some(HistoryItemId::new(1)),
                command_line: "new".to_string(),
                ..HistoryItem::EMPTY
            }],
        )
        .unwrap();

        let () = test()
            .with_history(&config_home, dst_format)
            .run("history import")?;

        let got = query_all(&*open_backend(&config_home, dst_format).unwrap()).unwrap();
        let want_history = vec![
            HistoryItem {
                id: Some(HistoryItemId::new(0)),
                command_line: "original-1".to_string(),
                ..HistoryItem::EMPTY
            },
            HistoryItem {
                id: Some(HistoryItemId::new(1)),
                command_line: "original-2".to_string(),
                ..HistoryItem::EMPTY
            },
            HistoryItem {
                id: Some(HistoryItemId::new(2)),
                command_line: "new".to_string(),
                ..HistoryItem::EMPTY
            },
        ];

        assert_eq!(commands_only(&got), commands_only(&want_history));
        assert_eq!(got, want_history);
        Ok(())
    })
}
