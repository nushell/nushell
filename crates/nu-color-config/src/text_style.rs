use nu_ansi_term::{Color, Style};

#[derive(Debug, Clone, Copy)]
pub enum Alignment {
    Center,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub struct TextStyle {
    pub alignment: Alignment,
    pub color_style: Option<Style>,
}

impl TextStyle {
    pub fn new() -> TextStyle {
        TextStyle {
            alignment: Alignment::Left,
            color_style: Some(Style::default()),
        }
    }

    pub fn bold(&self, bool_value: Option<bool>) -> TextStyle {
        let bv = bool_value.unwrap_or(false);

        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                is_bold: bv,
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn is_bold(&self) -> bool {
        self.color_style.unwrap_or_default().is_bold
    }

    pub fn dimmed(&self) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                is_dimmed: true,
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn is_dimmed(&self) -> bool {
        self.color_style.unwrap_or_default().is_dimmed
    }

    pub fn italic(&self) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                is_italic: true,
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn is_italic(&self) -> bool {
        self.color_style.unwrap_or_default().is_italic
    }

    pub fn underline(&self) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                is_underline: true,
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn is_underline(&self) -> bool {
        self.color_style.unwrap_or_default().is_underline
    }

    pub fn blink(&self) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                is_blink: true,
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn is_blink(&self) -> bool {
        self.color_style.unwrap_or_default().is_blink
    }

    pub fn reverse(&self) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                is_reverse: true,
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn is_reverse(&self) -> bool {
        self.color_style.unwrap_or_default().is_reverse
    }

    pub fn hidden(&self) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                is_hidden: true,
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn is_hidden(&self) -> bool {
        self.color_style.unwrap_or_default().is_hidden
    }

    pub fn strikethrough(&self) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                is_strikethrough: true,
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn is_strikethrough(&self) -> bool {
        self.color_style.unwrap_or_default().is_strikethrough
    }

    pub fn fg(&self, foreground: Color) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                foreground: Some(foreground),
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn on(&self, background: Color) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                background: Some(background),
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn bg(&self, background: Color) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                background: Some(background),
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn alignment(&self, align: Alignment) -> TextStyle {
        TextStyle {
            alignment: align,
            color_style: self.color_style,
        }
    }

    pub fn style(&self, style: Style) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                foreground: style.foreground,
                background: style.background,
                is_bold: style.is_bold,
                is_dimmed: style.is_dimmed,
                is_italic: style.is_italic,
                is_underline: style.is_underline,
                is_blink: style.is_blink,
                is_reverse: style.is_reverse,
                is_hidden: style.is_hidden,
                is_strikethrough: style.is_strikethrough,
                prefix_with_reset: false,
            }),
        }
    }

    pub fn basic_center() -> TextStyle {
        TextStyle::new()
            .alignment(Alignment::Center)
            .style(Style::default())
    }

    pub fn basic_right() -> TextStyle {
        TextStyle::new()
            .alignment(Alignment::Right)
            .style(Style::default())
    }

    pub fn basic_left() -> TextStyle {
        TextStyle::new()
            .alignment(Alignment::Left)
            .style(Style::default())
    }

    pub fn default_header() -> TextStyle {
        TextStyle::new()
            .alignment(Alignment::Center)
            .fg(Color::Green)
            .bold(Some(true))
    }

    pub fn default_field() -> TextStyle {
        TextStyle::new().fg(Color::Green).bold(Some(true))
    }

    pub fn with_attributes(bo: bool, al: Alignment, co: Color) -> TextStyle {
        TextStyle::new().alignment(al).fg(co).bold(Some(bo))
    }

    pub fn with_style(al: Alignment, style: Style) -> TextStyle {
        TextStyle::new().alignment(al).style(Style {
            foreground: style.foreground,
            background: style.background,
            is_bold: style.is_bold,
            is_dimmed: style.is_dimmed,
            is_italic: style.is_italic,
            is_underline: style.is_underline,
            is_blink: style.is_blink,
            is_reverse: style.is_reverse,
            is_hidden: style.is_hidden,
            is_strikethrough: style.is_strikethrough,
            prefix_with_reset: false,
        })
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use nu_ansi_term::Style;

    #[test]
    fn test_is_bold() {
        let text_style = TextStyle {
            alignment: Alignment::Left,
            color_style: Some(Style {
                is_bold: true,
                ..Default::default()
            }),
        };
        assert!(text_style.is_bold());
    }

    #[test]
    fn test_dimmed() {
        let text_style = TextStyle {
            alignment: Alignment::Left,
            color_style: Some(Style {
                ..Default::default()
            }),
        };
        let dimmed_style = text_style.dimmed();
        assert!(dimmed_style.is_dimmed());
    }

    #[test]
    fn test_is_dimmed() {
        let text_style = TextStyle {
            alignment: Alignment::Left,
            color_style: Some(Style {
                is_dimmed: true,
                ..Default::default()
            }),
        };
        assert!(text_style.is_dimmed());
    }

    #[test]
    fn test_italic() {
        let text_style = TextStyle {
            alignment: Alignment::Left,
            color_style: Some(Style {
                ..Default::default()
            }),
        };
        let italic_style = text_style.italic();
        assert!(italic_style.is_italic());
    }

    #[test]
    fn test_is_italic() {
        let text_style = TextStyle {
            alignment: Alignment::Left,
            color_style: Some(Style {
                is_italic: true,
                ..Default::default()
            }),
        };
        assert!(text_style.is_italic());
    }

    #[test]
    fn test_underline() {
        let text_style = TextStyle {
            alignment: Alignment::Left,
            color_style: Some(Style {
                ..Default::default()
            }),
        };
        let underline_style = text_style.underline();
        assert!(underline_style.is_underline());
    }

    #[test]
    fn test_is_underline() {
        let text_style = TextStyle {
            alignment: Alignment::Left,
            color_style: Some(Style {
                is_underline: true,
                ..Default::default()
            }),
        };
        assert!(text_style.is_underline());
    }

    #[test]
    fn test_blink() {
        let text_style = TextStyle {
            alignment: Alignment::Left,
            color_style: Some(Style {
                ..Default::default()
            }),
        };
        let blink_style = text_style.blink();
        assert!(blink_style.is_blink());
    }

    #[test]
    fn test_is_blink() {
        let text_style = TextStyle {
            alignment: Alignment::Left,
            color_style: Some(Style {
                is_blink: true,
                ..Default::default()
            }),
        };
        assert!(text_style.is_blink());
    }

    #[test]
    fn test_reverse() {
        let text_style = TextStyle {
            alignment: Alignment::Left,
            color_style: Some(Style {
                ..Default::default()
            }),
        };
        let reverse_style = text_style.reverse();
        assert!(reverse_style.is_reverse());
    }

    #[test]
    fn test_is_reverse() {
        let text_style = TextStyle {
            alignment: Alignment::Left,
            color_style: Some(Style {
                is_reverse: true,
                ..Default::default()
            }),
        };
        assert!(text_style.is_reverse());
    }

    #[test]
    fn test_hidden() {
        let text_style = TextStyle {
            alignment: Alignment::Left,
            color_style: Some(Style {
                ..Default::default()
            }),
        };
        let hidden_style = text_style.hidden();
        assert!(hidden_style.is_hidden());
    }

    #[test]
    fn test_is_hidden() {
        let text_style = TextStyle {
            alignment: Alignment::Left,
            color_style: Some(Style {
                is_hidden: true,
                ..Default::default()
            }),
        };
        assert!(text_style.is_hidden());
    }

    #[test]
    fn test_strikethrough() {
        let text_style = TextStyle {
            alignment: Alignment::Left,
            color_style: Some(Style {
                ..Default::default()
            }),
        };
        let strikethrough_style = text_style.strikethrough();
        assert!(strikethrough_style.is_strikethrough());
    }

    #[test]
    fn test_is_strikethrough() {
        let text_style = TextStyle {
            alignment: Alignment::Left,
            color_style: Some(Style {
                is_strikethrough: true,
                ..Default::default()
            }),
        };
        assert!(text_style.is_strikethrough());
    }
}
