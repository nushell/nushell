use std::{io::Write, path::PathBuf};

use chrono::{DateTime, FixedOffset, NaiveDateTime, Offset};
use nu_protocol::{ast::PathMember, Record, Span, Value};
use nu_test_support::{
    fs::{line_ending, Stub},
    nu, pipeline,
    playground::{Dirs, Playground},
};
use rand::{
    distributions::{Alphanumeric, DistString, Standard},
    prelude::Distribution,
    rngs::StdRng,
    Rng, SeedableRng,
};

#[test]
fn into_sqlite_schema() {
    Playground::setup("schema", |dirs, _| {
        let testdb = make_sqlite_db(
            &dirs,
            r#"[
                [somebool, someint, somefloat, somefilesize, someduration, somedate, somestring, somebinary];
                [true, 1, 2.0, 1kb, 1sec, "2023-09-10 11:30:00", "foo", ("binary" | into binary)],
                [false, 2, 3.0, 2mb, 4wk, "2020-09-10 12:30:00", "bar", ("wut" | into binary)],
            ]"#,
        );

        let conn = rusqlite::Connection::open(testdb).unwrap();
        let mut stmt = conn.prepare("SELECT * FROM pragma_table_info(?1)").unwrap();

        let actual_rows: Vec<_> = stmt
            .query_and_then(["main"], |row| -> rusqlite::Result<_, rusqlite::Error> {
                let name: String = row.get("name").unwrap();
                let col_type: String = row.get("type").unwrap();
                Ok((name, col_type))
            })
            .unwrap()
            .map(|row| row.unwrap())
            .collect();

        let expected_rows = vec![
            ("somebool".into(), "BOOLEAN".into()),
            ("someint".into(), "INTEGER".into()),
            ("somefloat".into(), "REAL".into()),
            ("somefilesize".into(), "INTEGER".into()),
            ("someduration".into(), "BIGINT".into()),
            ("somedate".into(), "TEXT".into()),
            ("somestring".into(), "TEXT".into()),
            ("somebinary".into(), "BLOB".into()),
        ];

        assert_eq!(expected_rows, actual_rows);
    });
}

#[test]
fn into_sqlite_values() {
    Playground::setup("values", |dirs, _| {
        insert_test_rows(
            &dirs,
            r#"[
                [somebool, someint, somefloat, somefilesize, someduration, somedate, somestring, somebinary];
                [true, 1, 2.0, 1kb, 1sec, "2023-09-10T11:30:00-00:00", "foo", ("binary" | into binary)],
                [false, 2, 3.0, 2mb, 4wk, "2020-09-10T12:30:00-00:00", "bar", ("wut" | into binary)],
            ]"#,
            None,
            vec![
                TestRow(
                    true,
                    1,
                    2.0,
                    1000,
                    1000000000,
                    DateTime::parse_from_rfc3339("2023-09-10T11:30:00-00:00").unwrap(),
                    "foo".into(),
                    b"binary".to_vec(),
                ),
                TestRow(
                    false,
                    2,
                    3.0,
                    2000000,
                    2419200000000000,
                    DateTime::parse_from_rfc3339("2020-09-10T12:30:00-00:00").unwrap(),
                    "bar".into(),
                    b"wut".to_vec(),
                ),
            ],
        );
    });
}

/// Opening a preexisting database should append to it
#[test]
fn into_sqlite_existing_db_append() {
    Playground::setup("existing_db_append", |dirs, _| {
        // create a new DB with only one row
        insert_test_rows(
            &dirs,
            r#"[
                [somebool, someint, somefloat, somefilesize, someduration, somedate, somestring, somebinary];
                [true, 1, 2.0, 1kb, 1sec, "2023-09-10T11:30:00-00:00", "foo", ("binary" | into binary)],
            ]"#,
            None,
            vec![TestRow(
                true,
                1,
                2.0,
                1000,
                1000000000,
                DateTime::parse_from_rfc3339("2023-09-10T11:30:00-00:00").unwrap(),
                "foo".into(),
                b"binary".to_vec(),
            )],
        );

        // open the same DB again and write one row
        insert_test_rows(
            &dirs,
            r#"[
                [somebool, someint, somefloat, somefilesize, someduration, somedate, somestring, somebinary];
                [false, 2, 3.0, 2mb, 4wk, "2020-09-10T12:30:00-00:00", "bar", ("wut" | into binary)],
            ]"#,
            None,
            // it should have both rows
            vec![
                TestRow(
                    true,
                    1,
                    2.0,
                    1000,
                    1000000000,
                    DateTime::parse_from_rfc3339("2023-09-10T11:30:00-00:00").unwrap(),
                    "foo".into(),
                    b"binary".to_vec(),
                ),
                TestRow(
                    false,
                    2,
                    3.0,
                    2000000,
                    2419200000000000,
                    DateTime::parse_from_rfc3339("2020-09-10T12:30:00-00:00").unwrap(),
                    "bar".into(),
                    b"wut".to_vec(),
                ),
            ],
        );
    });
}

/// Test inserting a good number of randomly generated rows to test an actual
/// streaming pipeline instead of a simple value
#[test]
fn into_sqlite_big_insert() {
    Playground::setup("big_insert", |dirs, playground| {
        const NUM_ROWS: usize = 10_000;
        const NUON_FILE_NAME: &str = "data.nuon";

        let nuon_path = dirs.test().join(NUON_FILE_NAME);

        playground.with_files(vec![Stub::EmptyFile(&nuon_path.to_string_lossy())]);

        let mut expected_rows = Vec::new();
        let mut nuon_file = std::fs::OpenOptions::new()
            .write(true)
            .open(&nuon_path)
            .unwrap();

        // write the header
        for row in std::iter::repeat_with(TestRow::random).take(NUM_ROWS) {
            let mut value: Value = row.clone().into();

            // HACK: Convert to a string to get around this: https://github.com/nushell/nushell/issues/9186
            value
                .upsert_cell_path(
                    &[PathMember::String {
                        val: "somedate".into(),
                        span: Span::unknown(),
                        optional: false,
                    }],
                    Box::new(|dateval| Value::string(dateval.as_string().unwrap(), dateval.span())),
                )
                .unwrap();

            let nuon = nu_command::value_to_string(&value, Span::unknown(), 0, None).unwrap()
                + &line_ending();

            nuon_file.write_all(nuon.as_bytes()).unwrap();
            expected_rows.push(row);
        }

        insert_test_rows(
            &dirs,
            &format!(
                "open --raw {} | lines | each {{ from nuon }}",
                nuon_path.to_string_lossy()
            ),
            None,
            expected_rows,
        );
    });
}

/// empty in, empty out
#[test]
fn into_sqlite_empty() {
    Playground::setup("empty", |dirs, _| {
        insert_test_rows(&dirs, r#"[]"#, Some("SELECT * FROM sqlite_schema;"), vec![]);
    });
}

#[derive(Debug, PartialEq, Clone)]
struct TestRow(
    bool,
    i64,
    f64,
    i64,
    i64,
    chrono::DateTime<chrono::FixedOffset>,
    std::string::String,
    std::vec::Vec<u8>,
);

impl TestRow {
    pub fn random() -> Self {
        StdRng::from_entropy().sample(Standard)
    }
}

impl From<TestRow> for Value {
    fn from(row: TestRow) -> Self {
        Value::record(
            Record::from_iter(vec![
                ("somebool".into(), Value::bool(row.0, Span::unknown())),
                ("someint".into(), Value::int(row.1, Span::unknown())),
                ("somefloat".into(), Value::float(row.2, Span::unknown())),
                (
                    "somefilesize".into(),
                    Value::filesize(row.3, Span::unknown()),
                ),
                (
                    "someduration".into(),
                    Value::duration(row.4, Span::unknown()),
                ),
                ("somedate".into(), Value::date(row.5, Span::unknown())),
                ("somestring".into(), Value::string(row.6, Span::unknown())),
                ("somebinary".into(), Value::binary(row.7, Span::unknown())),
            ]),
            Span::unknown(),
        )
    }
}

impl<'r> TryFrom<&rusqlite::Row<'r>> for TestRow {
    type Error = rusqlite::Error;

    fn try_from(row: &rusqlite::Row) -> Result<Self, Self::Error> {
        let somebool: bool = row.get("somebool").unwrap();
        let someint: i64 = row.get("someint").unwrap();
        let somefloat: f64 = row.get("somefloat").unwrap();
        let somefilesize: i64 = row.get("somefilesize").unwrap();
        let someduration: i64 = row.get("someduration").unwrap();
        let somedate: DateTime<FixedOffset> = row.get("somedate").unwrap();
        let somestring: String = row.get("somestring").unwrap();
        let somebinary: Vec<u8> = row.get("somebinary").unwrap();

        Ok(TestRow(
            somebool,
            someint,
            somefloat,
            somefilesize,
            someduration,
            somedate,
            somestring,
            somebinary,
        ))
    }
}

impl Distribution<TestRow> for Standard {
    fn sample<R>(&self, rng: &mut R) -> TestRow
    where
        R: rand::Rng + ?Sized,
    {
        let naive_dt =
            NaiveDateTime::from_timestamp_millis(rng.gen_range(0..2324252554000)).unwrap();
        let dt = DateTime::from_naive_utc_and_offset(naive_dt, chrono::Utc.fix());
        let rand_string = Alphanumeric.sample_string(rng, 10);

        // limit the size of the numbers to work around
        // https://github.com/nushell/nushell/issues/10612
        let filesize = rng.gen_range(-1024..=1024);
        let duration = rng.gen_range(-1024..=1024);

        TestRow(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            filesize,
            duration,
            dt,
            rand_string,
            rng.gen::<u64>().to_be_bytes().to_vec(),
        )
    }
}

fn make_sqlite_db(dirs: &Dirs, nu_table: &str) -> PathBuf {
    let testdir = dirs.test();
    let testdb_path =
        testdir.join(testdir.file_name().unwrap().to_str().unwrap().to_owned() + ".db");
    let testdb = testdb_path.to_str().unwrap();

    let nucmd = nu!(
        cwd: testdir,
        pipeline(&format!("{nu_table} | into sqlite {testdb}"))
    );

    assert!(nucmd.status.success());
    testdb_path
}

fn insert_test_rows(dirs: &Dirs, nu_table: &str, sql_query: Option<&str>, expected: Vec<TestRow>) {
    let sql_query = sql_query.unwrap_or("SELECT * FROM main;");
    let testdb = make_sqlite_db(dirs, nu_table);

    let conn = rusqlite::Connection::open(testdb).unwrap();
    let mut stmt = conn.prepare(sql_query).unwrap();

    let actual_rows: Vec<_> = stmt
        .query_and_then([], |row| TestRow::try_from(row))
        .unwrap()
        .map(|row| row.unwrap())
        .collect();

    assert_eq!(expected, actual_rows);
}
