use crate::context::Context;
use ansi_term::Color;
use log::{log_enabled, trace};
use nu_parser::hir::syntax_shape::color_fallible_syntax;
use nu_parser::{FlatShape, PipelineShape, TokenNode, TokensIterator};
use nu_protocol::outln;
use nu_source::{nom_input, HasSpan, Spanned, Tag, Tagged, Text};
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
        match self.context.shell_manager.hint(line, pos, ctx) {
            Ok(output) => output,
            Err(e) => Some(format!("{}", e)),
        }
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
        let tokens = nu_parser::pipeline(nom_input(line));

        match tokens {
            Err(_) => Cow::Borrowed(line),
            Ok((_rest, v)) => {
                let mut out = String::new();
                let pipeline = match v.as_pipeline() {
                    Err(_) => return Cow::Borrowed(line),
                    Ok(v) => v,
                };

                let tokens = vec![TokenNode::Pipeline(pipeline)];
                let mut tokens = TokensIterator::all(&tokens[..], Text::from(line), v.span());

                let text = Text::from(line);
                match self.context.expand_context(&text) {
                    Ok(expand_context) => {
                        let shapes = {
                            // We just constructed a token list that only contains a pipeline, so it can't fail
                            if let Err(err) =
                                color_fallible_syntax(&PipelineShape, &mut tokens, &expand_context)
                            {
                                let error_msg = format!("{}", err);
                                return Cow::Owned(error_msg);
                            }
                            tokens.with_color_tracer(|_, tracer| tracer.finish());

                            tokens.state().shapes()
                        };

                        trace!(target: "nu::color_syntax", "{:#?}", tokens.color_tracer());

                        if log_enabled!(target: "nu::color_syntax", log::Level::Debug) {
                            outln!("");
                            let _ = ptree::print_tree(
                                &tokens.color_tracer().clone().print(Text::from(line)),
                            );
                            outln!("");
                        }

                        for shape in shapes {
                            let styled = paint_flat_shape(&shape, line);
                            out.push_str(&styled);
                        }

                        Cow::Owned(out)
                    }
                    Err(err) => {
                        let error_msg = format!("{}", err);
                        Cow::Owned(error_msg)
                    }
                }
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
        FlatShape::CompareOperator => Color::Yellow.normal(),
        FlatShape::DotDot => Color::Yellow.bold(),
        FlatShape::Dot => Color::White.normal(),
        FlatShape::InternalCommand => Color::Cyan.bold(),
        FlatShape::ExternalCommand => Color::Cyan.normal(),
        FlatShape::ExternalWord => Color::Green.bold(),
        FlatShape::BareMember => Color::Yellow.bold(),
        FlatShape::StringMember => Color::Yellow.bold(),
        FlatShape::String => Color::Green.normal(),
        FlatShape::Path => Color::Cyan.normal(),
        FlatShape::GlobPattern => Color::Cyan.bold(),
        FlatShape::Word => Color::Green.normal(),
        FlatShape::Pipe => Color::Purple.bold(),
        FlatShape::Flag => Color::Blue.bold(),
        FlatShape::ShorthandFlag => Color::Blue.bold(),
        FlatShape::Int => Color::Purple.bold(),
        FlatShape::Decimal => Color::Purple.bold(),
        FlatShape::Whitespace | FlatShape::Separator => Color::White.normal(),
        FlatShape::Comment => Color::Green.bold(),
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
