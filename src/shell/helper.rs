use crate::context::Context;
use crate::parser::hir::syntax_shape::{color_fallible_syntax, FlatShape, PipelineShape};
use crate::parser::hir::TokensIterator;
use crate::parser::nom_input;
use crate::parser::parse::token_tree::TokenNode;
use crate::{Span, Spanned, SpannedItem, Tag, Tagged, Text};
use ansi_term::Color;
use log::trace;
use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use std::borrow::Cow::{self, Owned};

pub(crate) struct Helper {
    context: Context,
}

impl Helper {
    pub(crate) fn new(context: Context) -> Helper {
        Helper { context }
    }
}

impl Completer for Helper {
    type Candidate = rustyline::completion::Pair;
    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<rustyline::completion::Pair>), ReadlineError> {
        self.context.shell_manager.complete(line, pos, ctx)
    }
}

/*
impl Completer for Helper {
    type Candidate = rustyline::completion::Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<rustyline::completion::Pair>), ReadlineError> {
        let result = self.helper.complete(line, pos, ctx);

        result.map(|(x, y)| (x, y.iter().map(|z| z.into()).collect()))
    }
}
*/

impl Hinter for Helper {
    fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<String> {
        self.context.shell_manager.hint(line, pos, ctx)
    }
}

impl Highlighter for Helper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(&'s self, prompt: &'p str, _: bool) -> Cow<'b, str> {
        Owned("\x1b[32m".to_owned() + &prompt[0..prompt.len() - 2] + "\x1b[m> ")
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        let tokens = crate::parser::pipeline(nom_input(line));

        match tokens {
            Err(_) => Cow::Borrowed(line),
            Ok((_rest, v)) => {
                let mut out = String::new();
                let pipeline = match v.as_pipeline() {
                    Err(_) => return Cow::Borrowed(line),
                    Ok(v) => v,
                };

                let tokens = vec![TokenNode::Pipeline(pipeline.clone().spanned(v.span()))];
                let mut tokens = TokensIterator::all(&tokens[..], v.span());

                let text = Text::from(line);
                let expand_context = self
                    .context
                    .expand_context(&text, Span::new(0, line.len() - 1));
                let mut shapes = vec![];

                // We just constructed a token list that only contains a pipeline, so it can't fail
                color_fallible_syntax(&PipelineShape, &mut tokens, &expand_context, &mut shapes)
                    .unwrap();

                trace!(target: "nu::shapes",
                    "SHAPES :: {:?}",
                    shapes.iter().map(|shape| shape.item).collect::<Vec<_>>()
                );

                for shape in shapes {
                    let styled = paint_flat_shape(shape, line);
                    out.push_str(&styled);
                }

                Cow::Owned(out)

                // loop {
                //     match iter.next() {
                //         None => {
                //             return Cow::Owned(out);
                //         }
                //         Some(token) => {
                //             let styled = paint_pipeline_element(&token, line);
                //             out.push_str(&styled.to_string());
                //         }
                //     }
                // }
            }
        }
    }

    fn highlight_char(&self, _line: &str, _pos: usize) -> bool {
        true
    }
}

#[allow(unused)]
fn vec_tag<T>(input: Vec<Tagged<T>>) -> Option<Tag> {
    let mut iter = input.iter();
    let first = iter.next()?.tag.clone();
    let last = iter.last();

    Some(match last {
        None => first,
        Some(last) => first.until(&last.tag),
    })
}

fn paint_flat_shape(flat_shape: Spanned<FlatShape>, line: &str) -> String {
    let style = match &flat_shape.item {
        FlatShape::OpenDelimiter(_) => Color::White.normal(),
        FlatShape::CloseDelimiter(_) => Color::White.normal(),
        FlatShape::ItVariable => Color::Purple.bold(),
        FlatShape::Variable => Color::Purple.normal(),
        FlatShape::Operator => Color::Yellow.normal(),
        FlatShape::Dot => Color::White.normal(),
        FlatShape::InternalCommand => Color::Cyan.bold(),
        FlatShape::ExternalCommand => Color::Cyan.normal(),
        FlatShape::ExternalWord => Color::Black.bold(),
        FlatShape::BareMember => Color::Yellow.bold(),
        FlatShape::StringMember => Color::Yellow.bold(),
        FlatShape::String => Color::Green.normal(),
        FlatShape::Path => Color::Cyan.normal(),
        FlatShape::GlobPattern => Color::Cyan.bold(),
        FlatShape::Word => Color::Green.normal(),
        FlatShape::Pipe => Color::Purple.bold(),
        FlatShape::Flag => Color::Black.bold(),
        FlatShape::ShorthandFlag => Color::Black.bold(),
        FlatShape::Int => Color::Purple.bold(),
        FlatShape::Decimal => Color::Purple.bold(),
        FlatShape::Whitespace => Color::White.normal(),
        FlatShape::Error => Color::Red.bold(),
        FlatShape::Size { number, unit } => {
            let number = number.slice(line);
            let unit = unit.slice(line);
            return format!(
                "{}{}",
                Color::Purple.bold().paint(number),
                Color::Cyan.bold().paint(unit)
            );
        }
    };

    let body = flat_shape.span.slice(line);
    style.paint(body).to_string()
}

impl rustyline::Helper for Helper {}
