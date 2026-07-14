#![expect(clippy::test_attr_in_doctest)]

use nu_protocol::FromValue;

/// Output of the `complete` command as a struct.
///
/// Working with external processes in tests often require inspecting all of `stdin`, `stdout` and
/// the exit code separately.
/// This is easy to achieve using `complete`:
/// ```
/// # #[macro_use] extern crate nu_test_support;
/// use nu_protocol::Record;
/// use nu_test_support::prelude::*;
///
/// #[test]
/// #[deps(NU)]
/// fn run_non-existent_script() -> Result {
/// #     unimplemented!()
/// # }
/// #
/// # fn main() -> Result {
///     let result: Record = test().run("nu non-existent-script.nu | complete")?;
///     let stdout = result["stdout"].as_str()?;
///     let stderr = result["stdrr"].as_str()?;
///     let exit_code = result["exit_code"].as_int()?;
///
///     assert_ne!(exit_code, 0);
///     assert_contains("nu::shell::io::file_not_found", stderr);
///
///     Ok(())
/// }
/// ```
///
/// This type exists to avoid repetition and to make it more convenient to work with `complete`:
/// ```
/// # #[macro_use] extern crate nu_test_support;
/// use nu_test_support::prelude::*;
///
/// #[test]
/// #[deps(NU)]
/// fn run_non-existent_script() -> Result {
/// #     unimplemented!()
/// # }
/// #
/// # fn main() -> Result {
///     let result: CompleteResult = test().run("nu non-existent-script.nu | complete")?;
///
///     assert_ne!(result.exit_code, 0);
///     assert_contains("nu::shell::io::file_not_found", result.stderr);
///
///     Ok(())
/// }
/// ```
#[derive(FromValue)]
pub struct CompleteResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i64,
}
