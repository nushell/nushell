use crate::evaluate::scope::Scope;
use crate::shell::palette::Palette;
use nu_ansi_term::{Color, Style};
use nu_parser::ParserScope;
use nu_protocol::hir::FlatShape;
use nu_source::Spanned;
use std::borrow::Cow;

// FIXME: find a good home, as nu-engine may be too core for styling
pub struct Painter {
    original: Vec<u8>,
    styles: Vec<Style>,
}

impl Painter {
    fn new(original: &str) -> Painter {
        let bytes: Vec<u8> = original.bytes().collect();
        let bytes_count = bytes.len();
        Painter {
            original: bytes,
            styles: vec![Color::White.normal(); bytes_count],
        }
    }

    pub fn paint_string<'l, P: Palette>(line: &'l str, scope: &Scope, palette: &P) -> Cow<'l, str> {
        scope.enter_scope();
        let (block, _) = nu_parser::parse(line, 0, scope);
        scope.exit_scope();

        let shapes = nu_parser::shapes(&block);
        let mut painter = Painter::new(line);

        for shape in shapes {
            painter.paint_shape(&shape, palette);
        }

        Cow::Owned(painter.into_string())
    }

    fn paint_shape<P: Palette>(&mut self, shape: &Spanned<FlatShape>, palette: &P) {
        palette
            .styles_for_shape(shape)
            .iter()
            .for_each(|x| self.paint(x));
    }

    fn paint(&mut self, styled_span: &Spanned<Style>) {
        for pos in styled_span.span.start()..styled_span.span.end() {
            self.styles[pos] = styled_span.item;
        }
    }

    fn into_string(self) -> String {
        let mut idx_start = 0;
        let mut idx_end = 1;

        if self.original.is_empty() {
            String::new()
        } else {
            let mut builder = String::new();

            let mut current_style = self.styles[0];

            while idx_end < self.styles.len() {
                if self.styles[idx_end] != current_style {
                    // Emit, as we changed styles
                    let intermediate = String::from_utf8_lossy(&self.original[idx_start..idx_end]);

                    builder.push_str(&format!("{}", current_style.paint(intermediate)));

                    current_style = self.styles[idx_end];
                    idx_start = idx_end;
                    idx_end += 1;
                } else {
                    idx_end += 1;
                }
            }

            let intermediate = String::from_utf8_lossy(&self.original[idx_start..idx_end]);
            builder.push_str(&format!("{}", current_style.paint(intermediate)));

            builder
        }
    }
}
