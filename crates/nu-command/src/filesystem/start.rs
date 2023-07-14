use nu_engine::CallExt;
use nu_path::canonicalize_with;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type,
};
use std::path::Path;

#[derive(Clone)]
pub struct Start;

impl Command for Start {
    fn name(&self) -> &str {
        "start"
    }

    fn usage(&self) -> &str {
        "Open a folder, file or website in the default application or viewer."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["load", "folder", "directory", "run", "open"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("start")
            .input_output_types(vec![(Type::Nothing, Type::Any), (Type::String, Type::Any)])
            .required("path", SyntaxShape::String, "path to open")
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let path = call.req::<Spanned<String>>(engine_state, stack, 0)?;
        let path = Spanned {
            item: nu_utils::strip_ansi_string_unlikely(path.item),
            span: path.span,
        };
        let path_no_whitespace = &path.item.trim_end_matches(|x| matches!(x, '\x09'..='\x0d'));
        // only check if file exists in current current directory
        let file_path = Path::new(path_no_whitespace);
        if file_path.exists() {
            open::that(path_no_whitespace)?;
        } else if file_path.starts_with("https://") || file_path.starts_with("http://") {
            let url = url::Url::parse(&path.item).map_err(|_| {
                ShellError::GenericError(
                    format!("Cannot parse url: {}", &path.item),
                    "".to_string(),
                    Some(path.span),
                    Some("cannot parse".to_string()),
                    Vec::new(),
                )
            })?;
            open::that(url.as_str())?;
        } else {
            // try to distinguish between file not found and opening url without prefix
            if let Ok(path) =
                canonicalize_with(path_no_whitespace, std::env::current_dir()?.as_path())
            {
                open::that(path)?;
            } else {
                // open crate does not allow opening URL without prefix
                let path_with_prefix = Path::new("https://").join(&path.item);
                let common_domains = ["com", "net", "org", "edu", "sh"];
                if let Some(url) = path_with_prefix.to_str() {
                    let url = url::Url::parse(url).map_err(|_| {
                        ShellError::GenericError(
                            format!("Cannot parse url: {}", &path.item),
                            "".to_string(),
                            Some(path.span),
                            Some("cannot parse".to_string()),
                            Vec::new(),
                        )
                    })?;
                    if let Some(domain) = url.host() {
                        let domain = domain.to_string();
                        let ext = Path::new(&domain).extension().and_then(|s| s.to_str());
                        if let Some(url_ext) = ext {
                            if common_domains.contains(&url_ext) {
                                open::that(url.as_str())?;
                            }
                        }
                    }
                    return Err(ShellError::GenericError(
                        format!("Cannot find file or url: {}", &path.item),
                        "".to_string(),
                        Some(path.span),
                        Some("Use prefix https:// to disambiguate URLs from files".to_string()),
                        Vec::new(),
                    ));
                }
            };
        }
        Ok(PipelineData::Empty)
    }

    fn examples(&self) -> Vec<nu_protocol::Example> {
        vec![
            Example {
                description: "Open a text file with the default text editor",
                example: "start file.txt",
                result: None,
            },
            Example {
                description: "Open an image with the default image viewer",
                example: "start file.jpg",
                result: None,
            },
            Example {
                description: "Open the current directory with the default file manager",
                example: "start .",
                result: None,
            },
            Example {
                description: "Open a pdf with the default pdf viewer",
                example: "start file.pdf",
                result: None,
            },
            Example {
                description: "Open a website with default browser",
                example: "start https://www.nushell.sh",
                result: None,
            },
        ]
    }
}
