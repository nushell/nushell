use tabled::settings::style::{HorizontalLine, Style};
use tabled::settings::themes::Theme;

#[derive(Debug, Clone)]
pub struct TableTheme {
    theme: Theme,
    full_theme: Theme,
    has_inner: bool,
}

impl TableTheme {
    pub fn new(theme: impl Into<Theme>, full_theme: impl Into<Theme>, has_inner: bool) -> Self {
        Self {
            theme: theme.into(),
            full_theme: full_theme.into(),
            has_inner,
        }
    }

    pub fn basic() -> TableTheme {
        Self::new(Style::ascii(), Style::ascii(), true)
    }

    pub fn thin() -> TableTheme {
        Self::new(Style::modern(), Style::modern(), true)
    }

    pub fn light() -> TableTheme {
        let mut theme = Theme::from_style(Style::blank());
        theme.insert_horizontal_line(1, HorizontalLine::new('─').intersection('─'));

        Self::new(theme, Style::modern(), true)
    }

    pub fn psql() -> TableTheme {
        Self::new(Style::psql(), Style::psql(), true)
    }

    pub fn markdown() -> TableTheme {
        Self::new(Style::markdown(), Style::markdown(), true)
    }

    pub fn dots() -> TableTheme {
        let theme = Style::dots().remove_horizontal();

        Self::new(theme, Style::dots(), true)
    }

    pub fn restructured() -> TableTheme {
        Self::new(
            Style::re_structured_text(),
            Style::re_structured_text(),
            true,
        )
    }

    pub fn ascii_rounded() -> TableTheme {
        Self::new(Style::ascii_rounded(), Style::ascii_rounded(), true)
    }

    pub fn basic_compact() -> TableTheme {
        let theme = Style::ascii().remove_horizontal();

        Self::new(theme, Style::ascii(), true)
    }

    pub fn compact() -> TableTheme {
        let hline = HorizontalLine::inherit(Style::modern().remove_left().remove_right());
        let theme = Style::modern()
            .remove_left()
            .remove_right()
            .remove_horizontal()
            .horizontals([(1, hline)]);

        Self::new(theme, Style::modern(), true)
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

        Self::new(theme, full_theme, true)
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

        Self::new(theme, Style::extended(), true)
    }

    pub fn rounded() -> TableTheme {
        let full = Style::modern()
            .corner_top_left('╭')
            .corner_top_right('╮')
            .corner_bottom_left('╰')
            .corner_bottom_right('╯');

        Self::new(Style::rounded(), full, true)
    }

    pub fn reinforced() -> TableTheme {
        let full = Style::modern()
            .corner_top_left('┏')
            .corner_top_right('┓')
            .corner_bottom_left('┗')
            .corner_bottom_right('┛');

        Self::new(full.clone().remove_horizontal(), full, true)
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

        Self::new(theme, full, true)
    }

    pub fn none() -> TableTheme {
        Self::new(Style::blank(), Style::blank(), true)
    }

    pub fn has_top(&self) -> bool {
        self.theme.borders_has_top()
    }

    pub fn has_left(&self) -> bool {
        self.theme.borders_has_left()
    }

    pub fn has_right(&self) -> bool {
        self.theme.borders_has_right()
    }

    pub fn has_inner(&self) -> bool {
        self.has_inner
    }

    pub fn has_horizontals(&self) -> bool {
        self.full_theme.get_borders().has_horizontal()
    }

    pub fn get_theme_full(&self) -> Theme {
        self.full_theme.clone()
    }

    pub fn get_theme(&self) -> Theme {
        self.theme.clone()
    }
}
