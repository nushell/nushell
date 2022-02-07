mod host;
mod path;
mod query;
mod scheme;
mod url_;

use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{EngineState, Stack},
    PipelineData, ShellError, Span, Value,
};
use url::{self};

pub use self::host::SubCommand as UrlHost;
pub use self::path::SubCommand as UrlPath;
pub use self::query::SubCommand as UrlQuery;
pub use self::scheme::SubCommand as UrlScheme;
pub use url_::Url;

fn handle_value<F>(action: &F, v: &Value, span: Span) -> Value
where
    F: Fn(&url::Url) -> &str + Send + 'static,
{
    let a = |url| Value::String {
        val: action(url).to_string(),
        span,
    };

    match v {
        Value::String { val: s, .. } => {
            let s = s.trim();

            match url::Url::parse(s) {
                Ok(url) => a(&url),
                Err(_) => Value::String {
                    val: "".to_string(),
                    span,
                },
            }
        }
        other => {
            let span = other.span();
            match span {
                Ok(s) => {
                    let got = format!("Expected a string, got {} instead", other.get_type());
                    Value::Error {
                        error: ShellError::UnsupportedInput(got, s),
                    }
                }
                Err(e) => Value::Error { error: e },
            }
        }
    }
}

fn operator<F>(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
    action: &'static F,
) -> Result<PipelineData, ShellError>
where
    F: Fn(&url::Url) -> &str + Send + Sync + 'static,
{
    let span = call.head;
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;

    input.map(
        move |v| {
            if column_paths.is_empty() {
                handle_value(&action, &v, span)
            } else {
                let mut ret = v;

                for path in &column_paths {
                    let r = ret.update_cell_path(
                        &path.members,
                        Box::new(move |old| handle_value(&action, old, span)),
                    );
                    if let Err(error) = r {
                        return Value::Error { error };
                    }
                }

                ret
            }
        },
        engine_state.ctrlc.clone(),
    )
}
