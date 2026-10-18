use super::util::read_pickle;
use nu_engine::command_prelude::*;
use nu_parser::pickle;
use nu_path::expand_path_with;
use nu_protocol::shell_error::{generic::GenericError, io::IoError};
use std::{
    collections::HashSet,
    path::{Component, Path, PathBuf},
};

#[derive(Clone)]
pub struct Depickle;

impl Command for Depickle {
    fn name(&self) -> &str {
        "depickle"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("pickle", SyntaxShape::Filepath, "The pickle to read.")
            .named(
                "output-dir",
                SyntaxShape::Directory,
                "Where to write the files, instead of the current directory.",
                Some('o'),
            )
            .switch("force", "Overwrite files that already exist.", Some('f'))
            .input_output_type(Type::Nothing, Type::table())
            .category(Category::Debug)
    }

    fn description(&self) -> &str {
        "Write the source files a pickle carries back out, for inspection."
    }

    fn extra_description(&self) -> &str {
        "A pickle made by `pickle` carries the script and every file it `use`s or \
`source`s. They're written with their paths relative to the script's directory. A `..` in a \
path becomes `_up`, so nothing is written outside the output directory. The pickle has to come \
from the same nushell version and commit."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["nupkl", "unpickle", "extract", "source"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let (_, contents) = read_pickle(engine_state, stack, call)?;
        let output_dir: Option<Spanned<String>> =
            call.get_flag(engine_state, stack, "output-dir")?;
        let force = call.has_flag(engine_state, stack, "force")?;

        let cwd = engine_state.cwd(Some(stack))?;
        let dir = match output_dir {
            Some(dir) => expand_path_with(dir.item, &cwd, true),
            None => cwd.into(),
        };
        let files: Vec<(PathBuf, Vec<u8>)> = pickle::sources(&contents)?
            .into_iter()
            .map(|(name, content)| (dir.join(output_path(&name)), content))
            .collect();

        // Check everything before writing anything. A path with `..` and one with a real `_up`
        // can end up the same.
        let mut paths = HashSet::new();
        if let Some((path, _)) = files.iter().find(|(path, _)| !paths.insert(path)) {
            return Err(ShellError::Generic(GenericError::new(
                format!("{} would be written twice", path.display()),
                "the pickle carries two files that end up with this path",
                head,
            )));
        }
        if !force && let Some((path, _)) = files.iter().find(|(path, _)| path.exists()) {
            return Err(ShellError::Generic(
                GenericError::new(
                    format!("{} already exists", path.display()),
                    "the pickle carries a file with this path",
                    head,
                )
                .with_help("use --force to overwrite it, or --output-dir to write elsewhere"),
            ));
        }

        let mut written = vec![];
        for (path, content) in files {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|err| IoError::new(err, head, parent.to_path_buf()))?;
            }
            std::fs::write(&path, &content).map_err(|err| IoError::new(err, head, path.clone()))?;
            written.push(Value::record(
                record! {
                    "path" => Value::string(path.to_string_lossy(), head),
                    "size" => Value::filesize(content.len() as i64, head),
                },
                head,
            ));
        }
        Ok(Value::list(written, head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Write the sources of `script.nupkl` into the current directory.",
                example: "depickle script.nupkl",
                result: None,
            },
            Example {
                description: "Write them into `sources`, replacing files that are there.",
                example: "depickle script.nupkl --output-dir sources --force",
                result: None,
            },
        ]
    }
}

/// Where a pickled source file goes, relative to the output directory. Only plain path parts are
/// kept: `..` becomes `_up`, and roots and drive prefixes are dropped, so a pickle can't make
/// `depickle` write outside the output directory.
fn output_path(name: &str) -> PathBuf {
    name.split(['/', '\\'])
        .filter_map(|part| match part {
            "" | "." => None,
            ".." => Some("_up".into()),
            part => match Path::new(part).components().next() {
                Some(Component::Normal(part)) => Some(PathBuf::from(part)),
                _ => None,
            },
        })
        .collect()
}
