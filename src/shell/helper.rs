use crate::context::Context;
use crate::parser::hir::syntax_shape::{color_fallible_syntax, FlatShape, PipelineShape};
use crate::parser::hir::TokensIterator;
use crate::parser::nom_input;
use crate::parser::parse::token_tree::TokenNode;
use crate::{HasSpan, Spanned, SpannedItem, Tag, Tagged, Text};
use ansi_term::Color;
use log::{log_enabled, trace};
use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use std::borrow::Cow::{self, Owned};

pub(crate) struct Helper {
    context: Context,
    pub colored_prompt: String,
}

impl Helper {
    pub(crate) fn new(context: Context) -> Helper {
        Helper {
            context,
            colored_prompt: String::new(),
        }
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

impl Hinter for Helper {
    fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<String> {
        self.context.shell_manager.hint(line, pos, ctx)
    }
}

impl Highlighter for Helper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        use std::borrow::Cow::Borrowed;

        if default {
            Borrowed(&self.colored_prompt)
        } else {
            Borrowed(prompt)
        }
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
                let expand_context = self.context.expand_context(&text);

                #[cfg(not(coloring_in_tokens))]
                let shapes = {
                    let mut shapes = vec![];
                    color_fallible_syntax(
                        &PipelineShape,
                        &mut tokens,
                        &expand_context,
                        &mut shapes,
                    )
                    .unwrap();
                    shapes
                };

                #[cfg(coloring_in_tokens)]
                let shapes = {
                    // We just constructed a token list that only contains a pipeline, so it can't fail
                    color_fallible_syntax(&PipelineShape, &mut tokens, &expand_context).unwrap();
                    tokens.with_color_tracer(|_, tracer| tracer.finish());

                    tokens.state().shapes()
                };

                trace!(target: "nu::color_syntax", "{:#?}", tokens.color_tracer());

                if log_enabled!(target: "nu::color_syntax", log::Level::Debug) {
                    outln!("");
                    ptree::print_tree(&tokens.color_tracer().clone().print(Text::from(line)))
                        .unwrap();
                    outln!("");
                }

                for shape in shapes {
                    let styled = paint_flat_shape(&shape, line);
                    out.push_str(&styled);
                }

                Cow::Owned(out)
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

fn paint_flat_shape(flat_shape: &Spanned<FlatShape>, line: &str) -> String {
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
