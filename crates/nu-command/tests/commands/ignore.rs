use nu_test_support::prelude::*;
use std::fs;

#[test]
fn ignore_still_causes_stream_to_be_consumed_fully() -> Result {
    Playground::setup("ignore_consumes_stream", |dirs, _| {
        let code = "
            [foo bar]
            | each {|val| $val | save --append output.txt; $val}
            | ignore
        ";

        let () = test().cwd(dirs.test()).run(code)?;
        let file_content = fs::read_to_string(dirs.test().join("output.txt")).unwrap();
        assert_eq!(file_content, "foobar");
        Ok(())
    })
}
