use nu_protocol::PipelineData;
use nu_test_support::prelude::*;

#[test]
fn metadata_access_is_streaming() -> Result {
    let code = "
        let id = job id

        for i in (
            seq 1 3 | metadata access {|md|
                each { job send $id; $in }
            }
        ) {
            $i | job send $id
        }

        generate {|x=null|
            try {{next: null, out: ( job recv --timeout 0sec )}}
        }
    ";

    test().run(code).expect_value_eq([1, 1, 2, 2, 3, 3])
}

#[test]
fn metadata_access_affect_caller_env() -> Result {
    let mut tester = test();
    tester.run("$env.FOO?").expect_value_eq(())?;

    let code = "
        seq 0 9
        | metadata access {|md|
            match ($env.FOO = true) { _ => {} }
        }
    ";

    let output = tester.run_raw(code)?;
    let PipelineData::ListStream(stream, _) = output.body else {
        panic!("Output must be a stream")
    };
    stream
        .into_value()
        .map_err(Error::from)
        .expect_value_eq((0..10).collect::<Vec<i64>>())?;

    tester.run("$env.FOO").expect_value_eq(true)
}
