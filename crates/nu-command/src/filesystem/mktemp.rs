use nu_engine::env::current_dir;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};

use std::path::PathBuf;

#[derive(Clone)]
pub struct Mktemp;

impl Command for Mktemp {
    fn name(&self) -> &str {
        "mktemp"
    }

    fn usage(&self) -> &str {
        "Create temporary files or directories using uutils/coreutils mktemp. TEMPLATE must contain at least 3 consecutive 'X's in last component."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "coreutils",
            "create",
            "directory",
            "file",
            "folder",
            "temporary",
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

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let template: String = call
            .rest(engine_state, stack, 0)?
            .get(0)
            .cloned()
            .map(|i: Spanned<String>| i.item)
            .unwrap_or("tmp.XXXXXXXXXX".to_string()); // same as default in coreutils

        let directory = call.has_flag("directory");
        let suffix: Option<String> = call
            .get_flag(engine_state, stack, "suffix")?
            .map(|i: Spanned<String>| i.item);
        let tmpdir = call.has_flag("tmpdir");
        let tmpdir_path: Option<PathBuf> = call
            .get_flag(engine_state, stack, "tmpdir-path")?
            .map(|i: Spanned<PathBuf>| i.item);

        let tmpdir = if tmpdir_path.is_some() {
            tmpdir_path
        } else if directory || tmpdir {
            Some(std::env::temp_dir())
        } else {
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
                .map_err(|e| ShellError::IOErrorSpanned(e.to_string_lossy().to_string(), span))?,
            Err(e) => {
                return Err(ShellError::GenericError(
                    format!("{}", e),
                    format!("{}", e),
                    None,
                    None,
                    Vec::new(),
                ));
            }
        };
        Ok(PipelineData::Value(Value::string(res, span), None))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Make a temporary file with the given suffix in the current working directory.",
                example: "mktemp --suffix .txt",
                result: Some(Value::string("<WORKING_DIR>/tmp.lekjbhelyx.txt", Span::new(0, 0))),
            },
            Example {
                description: "Make a temporary file named testfile.XXX with the 'X's as random characters in the current working directory. If a template is provided, it must end in at least three 'X's.",
                example: "mktemp testfile.XXX",
                result: Some(Value::string("<WORKING_DIR>/testfile.4kh", Span::new(0, 0))),
            },
            Example {
                description: "Make a temporary file with a template in the system temp directory.",
                example: "mktemp -t testfile.XXX",
                result: Some(Value::string("/tmp/testfile.4kh", Span::new(0, 0))),
            },
            Example {
                description: "Make a temporary directory with randomly generated name in the temporary directory.",
                example: "mktemp -d",
                result: Some(Value::string("/tmp/tmp.NMw9fJr8K0", Span::new(0, 0))),
            },
        ]
    }
}
