use ansi_term::{Color, Style};
use nu_protocol::hir::FlatShape;
use nu_source::{Span, Spanned};

pub trait Palette {
    fn styles_for_shape(&self, shape: &Spanned<FlatShape>) -> Vec<Spanned<Style>>;
}

pub struct DefaultPalette {}

impl Palette for DefaultPalette {
    fn styles_for_shape(&self, shape: &Spanned<FlatShape>) -> Vec<Spanned<Style>> {
        match &shape.item {
            FlatShape::OpenDelimiter(_) => single_style_span(Color::White.normal(), shape.span),
            FlatShape::CloseDelimiter(_) => single_style_span(Color::White.normal(), shape.span),
            FlatShape::ItVariable | FlatShape::Keyword => {
                single_style_span(Color::Purple.bold(), shape.span)
            }
            FlatShape::Variable | FlatShape::Identifier => {
                single_style_span(Color::Purple.normal(), shape.span)
            }
            FlatShape::Type => single_style_span(Color::Blue.bold(), shape.span),
            FlatShape::Operator => single_style_span(Color::Yellow.normal(), shape.span),
            FlatShape::DotDot => single_style_span(Color::Yellow.bold(), shape.span),
            FlatShape::Dot => single_style_span(Style::new().fg(Color::White), shape.span),
            FlatShape::InternalCommand => single_style_span(Color::Cyan.bold(), shape.span),
            FlatShape::ExternalCommand => single_style_span(Color::Cyan.normal(), shape.span),
            FlatShape::ExternalWord => single_style_span(Color::Green.bold(), shape.span),
            FlatShape::BareMember => single_style_span(Color::Yellow.bold(), shape.span),
            FlatShape::StringMember => single_style_span(Color::Yellow.bold(), shape.span),
            FlatShape::String => single_style_span(Color::Green.normal(), shape.span),
            FlatShape::Path => single_style_span(Color::Cyan.normal(), shape.span),
            FlatShape::GlobPattern => single_style_span(Color::Cyan.bold(), shape.span),
            FlatShape::Word => single_style_span(Color::Green.normal(), shape.span),
            FlatShape::Pipe => single_style_span(Color::Purple.bold(), shape.span),
            FlatShape::Flag => single_style_span(Color::Blue.bold(), shape.span),
            FlatShape::ShorthandFlag => single_style_span(Color::Blue.bold(), shape.span),
            FlatShape::Int => single_style_span(Color::Purple.bold(), shape.span),
            FlatShape::Decimal => single_style_span(Color::Purple.bold(), shape.span),
            FlatShape::Whitespace | FlatShape::Separator => {
                single_style_span(Color::White.normal(), shape.span)
            }
            FlatShape::Comment => single_style_span(Color::Green.bold(), shape.span),
            FlatShape::Garbage => {
                single_style_span(Style::new().fg(Color::White).on(Color::Red), shape.span)
            }
            FlatShape::Size { number, unit } => vec![
                Spanned::<Style> {
                    span: *number,
                    item: Color::Purple.bold(),
                },
                Spanned::<Style> {
                    span: *unit,
                    item: Color::Cyan.bold(),
                },
            ],
        }
    }
}

fn single_style_span(style: Style, span: Span) -> Vec<Spanned<Style>> {
    vec![Spanned::<Style> { span, item: style }]
}
