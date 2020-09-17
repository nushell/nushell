use crate::shell::palette::Palette;
use ansi_term::{Color, Style};
use nu_parser::SignatureRegistry;
use nu_protocol::hir::FlatShape;
use nu_source::Spanned;
use std::borrow::Cow;

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

    pub fn paint_string<'l, P: Palette>(
        line: &'l str,
        registry: &dyn SignatureRegistry,
        palette: &P,
    ) -> Cow<'l, str> {
        let lite_block = nu_parser::lite_parse(line, 0);

        match lite_block {
            Err(_) => Cow::Borrowed(line),
            Ok(lb) => {
                let classified = nu_parser::classify_block(&lb, registry);

                let shapes = nu_parser::shapes(&classified.block);
                let mut painter = Painter::new(line);

                for shape in shapes {
                    painter.paint_shape(&shape, palette);
                }

                Cow::Owned(painter.into_string())
            }
        }
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
