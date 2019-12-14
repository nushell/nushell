#[macro_use]
pub mod macros;

pub mod commands;
pub mod hir;
pub mod parse;
pub mod parse_command;

pub use crate::commands::classified::{
    external::ExternalCommand, internal::InternalCommand, ClassifiedCommand, ClassifiedPipeline,
};
pub use crate::hir::syntax_shape::flat_shape::{FlatShape, ShapeResult};
pub use crate::hir::syntax_shape::{ExpandContext, ExpandSyntax, PipelineShape, SignatureRegistry};
pub use crate::hir::tokens_iterator::TokensIterator;
pub use crate::parse::files::Files;
pub use crate::parse::flag::Flag;
pub use crate::parse::operator::{CompareOperator, EvaluationOperator};
pub use crate::parse::parser::Number;
pub use crate::parse::parser::{module, pipeline};
pub use crate::parse::token_tree::{Delimiter, SpannedToken, Token};
pub use crate::parse::token_tree_builder::TokenTreeBuilder;

use log::log_enabled;
use nu_errors::ShellError;
use nu_protocol::{errln, outln};
use nu_source::{nom_input, HasSpan, Text};

pub fn pipeline_shapes(line: &str, expand_context: ExpandContext) -> Vec<ShapeResult> {
    let tokens = parse_pipeline(line);

    match tokens {
        Err(_) => vec![],
        Ok(v) => {
            let pipeline = match v.as_pipeline() {
                Err(_) => return vec![],
                Ok(v) => v,
            };

            let tokens = vec![Token::Pipeline(pipeline.clone()).into_spanned(v.span())];
            let mut tokens = TokensIterator::new(&tokens[..], expand_context, v.span());

            let shapes = {
                // We just constructed a token list that only contains a pipeline, so it can't fail
                let result = tokens.expand_infallible(PipelineShape);

                if let Some(failure) = result.failed {
                    errln!(
                        "BUG: PipelineShape didn't find a pipeline :: {:#?}",
                        failure
                    );
                }

                tokens.finish_tracer();

                tokens.state().shapes()
            };

            if log_enabled!(target: "nu::expand_syntax", log::Level::Debug) {
                outln!("");
                ptree::print_tree(&tokens.expand_tracer().clone().print(Text::from(line))).unwrap();
                outln!("");
            }

            shapes.clone()
        }
    }
}

pub fn parse_pipeline(input: &str) -> Result<SpannedToken, ShellError> {
    let _ = pretty_env_logger::try_init();

    match pipeline(nom_input(input)) {
        Ok((_rest, val)) => Ok(val),
        Err(err) => Err(ShellError::parse_error(err)),
    }
}

pub use parse_pipeline as parse;

pub fn parse_script(input: &str) -> Result<SpannedToken, ShellError> {
    let _ = pretty_env_logger::try_init();

    match module(nom_input(input)) {
        Ok((_rest, val)) => Ok(val),
        Err(err) => Err(ShellError::parse_error(err)),
    }
}
