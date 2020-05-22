use crate::theme::Theme;
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

pub struct ThemedPallet {
    theme: Theme,
}

impl Palette for ThemedPallet {
    fn styles_for_shape(&self, shape: &Spanned<FlatShape>) -> Vec<Spanned<Style>> {
        match &shape.item {
            FlatShape::OpenDelimiter(_) => {
                single_style_span(self.theme.open_delimiter.normal(), shape.span)
            }
            FlatShape::CloseDelimiter(_) => {
                single_style_span(self.theme.close_delimiter.normal(), shape.span)
            }
            FlatShape::ItVariable => single_style_span(self.theme.it_variable.bold(), shape.span),
            FlatShape::Keyword => single_style_span(self.theme.keyword.bold(), shape.span),
            FlatShape::Variable => single_style_span(self.theme.variable.normal(), shape.span),
            FlatShape::Identifier => single_style_span(self.theme.identifier.normal(), shape.span),
            FlatShape::Type => single_style_span(self.theme.r#type.bold(), shape.span),
            FlatShape::Operator => single_style_span(self.theme.operator.normal(), shape.span),
            FlatShape::DotDot => single_style_span(self.theme.dot_dot.bold(), shape.span),
            FlatShape::Dot => single_style_span(Style::new().fg(self.theme.dot), shape.span),
            FlatShape::InternalCommand => {
                single_style_span(self.theme.internal_command.bold(), shape.span)
            }
            FlatShape::ExternalCommand => {
                single_style_span(self.theme.external_command.normal(), shape.span)
            }
            FlatShape::ExternalWord => {
                single_style_span(self.theme.external_word.bold(), shape.span)
            }
            FlatShape::BareMember => single_style_span(self.theme.bare_member.bold(), shape.span),
            FlatShape::StringMember => {
                single_style_span(self.theme.string_member.bold(), shape.span)
            }
            FlatShape::String => single_style_span(self.theme.string.normal(), shape.span),
            FlatShape::Path => single_style_span(self.theme.path.normal(), shape.span),
            FlatShape::GlobPattern => single_style_span(self.theme.glob_pattern.bold(), shape.span),
            FlatShape::Word => single_style_span(self.theme.word.normal(), shape.span),
            FlatShape::Pipe => single_style_span(self.theme.pipe.bold(), shape.span),
            FlatShape::Flag => single_style_span(self.theme.flag.bold(), shape.span),
            FlatShape::ShorthandFlag => {
                single_style_span(self.theme.shorthand_flag.bold(), shape.span)
            }
            FlatShape::Int => single_style_span(self.theme.int.bold(), shape.span),
            FlatShape::Decimal => single_style_span(self.theme.decimal.bold(), shape.span),
            FlatShape::Whitespace => single_style_span(self.theme.whitespace.normal(), shape.span),
            FlatShape::Separator => single_style_span(self.theme.separator.normal(), shape.span),
            FlatShape::Comment => single_style_span(self.theme.comment.bold(), shape.span),
            FlatShape::Garbage => single_style_span(
                Style::new().fg(self.theme.garbage).on(Color::Red),
                shape.span,
            ),
            FlatShape::Size { number, unit } => vec![
                Spanned::<Style> {
                    span: *number,
                    item: self.theme.size_number.bold(),
                },
                Spanned::<Style> {
                    span: *unit,
                    item: self.theme.size_unit.bold(),
                },
            ],
        }
    }
}

fn single_style_span(style: Style, span: Span) -> Vec<Spanned<Style>> {
    vec![Spanned::<Style> { span, item: style }]
}
