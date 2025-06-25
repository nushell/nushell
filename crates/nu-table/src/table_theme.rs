use tabled::settings::{
    style::{HorizontalLine, Style},
    themes::Theme,
};

#[derive(Debug, Clone)]
pub struct TableTheme {
    base: Theme,
    full: Theme,
}

impl TableTheme {
    fn new(base: impl Into<Theme>, full: impl Into<Theme>) -> Self {
        Self {
            base: base.into(),
            full: full.into(),
        }
    }

    pub fn basic() -> TableTheme {
        Self::new(Style::ascii(), Style::ascii())
    }

    pub fn thin() -> TableTheme {
        Self::new(Style::modern(), Style::modern())
    }

    pub fn light() -> TableTheme {
        let mut theme = Theme::from_style(Style::blank());
        theme.insert_horizontal_line(1, HorizontalLine::new('─').intersection('─'));

        Self::new(theme, Style::modern())
    }

    pub fn psql() -> TableTheme {
        Self::new(Style::psql(), Style::psql())
    }

    pub fn markdown() -> TableTheme {
        Self::new(Style::markdown(), Style::markdown())
    }

    pub fn dots() -> TableTheme {
        let theme = Style::dots().remove_horizontal();

        Self::new(theme, Style::dots())
    }

    pub fn restructured() -> TableTheme {
        Self::new(Style::re_structured_text(), Style::re_structured_text())
    }

    pub fn ascii_rounded() -> TableTheme {
        Self::new(Style::ascii_rounded(), Style::ascii_rounded())
    }

    pub fn basic_compact() -> TableTheme {
        let theme = Style::ascii().remove_horizontal();

        Self::new(theme, Style::ascii())
    }

    pub fn compact() -> TableTheme {
        let hline = HorizontalLine::inherit(Style::modern().remove_left().remove_right());
        let theme = Style::modern()
            .remove_left()
            .remove_right()
            .remove_horizontal()
            .horizontals([(1, hline)]);

        Self::new(theme, Style::modern())
    }

    pub fn with_love() -> TableTheme {
        let theme = Style::empty()
            .top('❤')
            .bottom('❤')
            .vertical('❤')
            .horizontals([(1, HorizontalLine::new('❤').intersection('❤'))]);

        let full_theme = Style::empty()
            .top('❤')
            .bottom('❤')
            .vertical('❤')
            .horizontal('❤')
            .left('❤')
            .right('❤')
            .intersection_top('❤')
            .corner_top_left('❤')
            .corner_top_right('❤')
            .intersection_bottom('❤')
            .corner_bottom_left('❤')
            .corner_bottom_right('❤')
            .intersection_right('❤')
            .intersection_left('❤')
            .intersection('❤');

        Self::new(theme, full_theme)
    }

    pub fn compact_double() -> TableTheme {
        let hline = HorizontalLine::inherit(Style::extended())
            .remove_left()
            .remove_right();
        let theme = Style::extended()
            .remove_left()
            .remove_right()
            .remove_horizontal()
            .horizontals([(1, hline)]);

        Self::new(theme, Style::extended())
    }

    pub fn rounded() -> TableTheme {
        let full = Style::modern()
            .corner_top_left('╭')
            .corner_top_right('╮')
            .corner_bottom_left('╰')
            .corner_bottom_right('╯');

        Self::new(Style::rounded(), full)
    }

    pub fn reinforced() -> TableTheme {
        let full = Style::modern()
            .corner_top_left('┏')
            .corner_top_right('┓')
            .corner_bottom_left('┗')
            .corner_bottom_right('┛');

        Self::new(full.clone().remove_horizontal(), full)
    }

    pub fn heavy() -> TableTheme {
        let theme = Style::empty()
            .top('━')
            .bottom('━')
            .vertical('┃')
            .left('┃')
            .right('┃')
            .intersection_top('┳')
            .intersection_bottom('┻')
            .corner_top_left('┏')
            .corner_top_right('┓')
            .corner_bottom_left('┗')
            .corner_bottom_right('┛')
            .horizontals([(1, HorizontalLine::full('━', '╋', '┣', '┫'))]);
        let full = theme
            .clone()
            .remove_horizontals()
            .horizontal('━')
            .intersection_left('┣')
            .intersection_right('┫')
            .intersection('╋');

        Self::new(theme, full)
    }

    pub fn single() -> TableTheme {
        let full = Style::modern()
            .corner_top_left('┌')
            .corner_top_right('┐')
            .corner_bottom_left('└')
            .corner_bottom_right('┘');

        Self::new(Style::sharp(), full)
    }

    pub fn double() -> TableTheme {
        let hline = HorizontalLine::inherit(Style::extended());
        let theme = Style::extended()
            .remove_horizontal()
            .horizontals([(1, hline)]);

        Self::new(theme, Style::extended())
    }

    pub fn none() -> TableTheme {
        Self::new(Style::blank(), Style::blank())
    }

    pub fn as_full(&self) -> &Theme {
        &self.full
    }

    pub fn as_base(&self) -> &Theme {
        &self.base
    }
}
