#[allow(deprecated)]
use nu_engine::{command_prelude::*, env::current_dir};
use std::path::PathBuf;

#[derive(Clone)]
pub struct Mktemp;

impl Command for Mktemp {
    fn name(&self) -> &str {
        "mktemp"
    }

    fn description(&self) -> &str {
        "Create temporary files or directories using uutils/coreutils mktemp."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "create",
            "directory",
            "file",
            "folder",
            "temporary",
            "coreutils",
        ]
    }

    fn signature(&self) -> Signature {
        Signature::build("mktemp")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .optional(
                "template",
                SyntaxShape::String,
                "Optional pattern from which the name of the file or directory is derived. Must contain at least three 'X's in last component.",
            )
            .named("suffix", SyntaxShape::String, "Append suffix to template; must not contain a slash.", None)
            .named("tmpdir-path", SyntaxShape::Filepath, "Interpret TEMPLATE relative to tmpdir-path. If tmpdir-path is not set use $TMPDIR", Some('p'))
            .switch("tmpdir", "Interpret TEMPLATE relative to the system temporary directory.", Some('t'))
            .switch("directory", "Create a directory instead of a file.", Some('d'))
            .category(Category::FileSystem)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Make a temporary file with the given suffix in the current working directory.",
                example: "mktemp --suffix .txt",
                result: Some(Value::test_string("<WORKING_DIR>/tmp.lekjbhelyx.txt")),
            },
            Example {
                description: "Make a temporary file named testfile.XXX with the 'X's as random characters in the current working directory.",
                example: "mktemp testfile.XXX",
                result: Some(Value::test_string("<WORKING_DIR>/testfile.4kh")),
            },
            Example {
                description: "Make a temporary file with a template in the system temp directory.",
                example: "mktemp -t testfile.XXX",
                result: Some(Value::test_string("/tmp/testfile.4kh")),
            },
            Example {
                description: "Make a temporary directory with randomly generated name in the temporary directory.",
                example: "mktemp -d",
                result: Some(Value::test_string("/tmp/tmp.NMw9fJr8K0")),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let template = call
            .rest(engine_state, stack, 0)?
            .first()
            .cloned()
            .map(|i: Spanned<String>| i.item)
            .unwrap_or("tmp.XXXXXXXXXX".to_string()); // same as default in coreutils
        let directory = call.has_flag(engine_state, stack, "directory")?;
        let suffix = call.get_flag(engine_state, stack, "suffix")?;
        let tmpdir = call.has_flag(engine_state, stack, "tmpdir")?;
        let tmpdir_path = call
            .get_flag(engine_state, stack, "tmpdir-path")?
            .map(|i: Spanned<PathBuf>| i.item);

        let tmpdir = if tmpdir_path.is_some() {
            tmpdir_path
        } else if directory || tmpdir {
            Some(std::env::temp_dir())
        } else {
            #[allow(deprecated)]
            Some(current_dir(engine_state, stack)?)
        };

        let options = uu_mktemp::Options {
            directory,
            dry_run: false,
            quiet: false,
            suffix,
            template,
            tmpdir,
            treat_as_template: true,
        };

        let res = match uu_mktemp::mktemp(&options) {
            Ok(res) => res
                .into_os_string()
                .into_string()
                .map_err(|_| ShellError::NonUtf8 { span })?,
            Err(e) => {
                return Err(ShellError::GenericError {
                    error: format!("{e}"),
                    msg: format!("{e}"),
                    span: None,
                    help: None,
                    inner: vec![],
                });
            }
        };
        Ok(PipelineData::value(Value::string(res, span), None))
    }
}
