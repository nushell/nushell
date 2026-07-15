#![allow(clippy::unwrap_used)]

#[macro_use]
extern crate nu_test_support;
use nu_test_support::harness::main;

use nu_test_support::prelude::*;

// Because the color config closures can't modify the global environment, passing information out
// requires using the in memory sqlite database (`stor`) or job messaging as a queue.
#[test]
fn test_computable_style_closure_basic() -> Result {
    let mut tester = test();

    let code = r#"
        let id = job id

        $env.config = {
            color_config: {
                string: {|e| $e | job send $id; 'red' }
            }
        }
    "#;
    let () = tester.run(code)?;

    let () = tester.run("[bell book candle] | table | ignore")?;

    for e in ["bell", "book", "candle"] {
        tester.run("job recv --timeout 0sec").expect_value_eq(e)?;
    }

    Ok(())
}

#[test]
#[deps(NU)]
fn test_computable_style_closure_errors() -> Result {
    let child_code = "
        $env.config = {
            color_config: {
                string: {|e| $e + 2 }
            }
        }

        [bell] | table
    ";

    let code = "
        let child_code

        nu --no-config-file --commands $child_code | complete
    ";

    let result: CompleteResult = test().run_with_data(code, child_code)?;

    assert_contains("nu::shell::operator_incompatible_types", result.stderr);
    assert_contains("bell", result.stdout);

    Ok(())
}
