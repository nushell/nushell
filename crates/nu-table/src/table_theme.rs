use tabled::settings::style::{HorizontalLine, Line, RawStyle, Style};

#[derive(Debug, Clone)]
pub struct TableTheme {
    theme: RawStyle,
    full_theme: RawStyle,
    has_inner: bool,
}

impl TableTheme {
    pub fn basic() -> TableTheme {
        Self {
            theme: Style::ascii().into(),
            full_theme: Style::ascii().into(),
            has_inner: true,
        }
    }

    pub fn thin() -> TableTheme {
        Self {
            theme: Style::modern().into(),
            full_theme: Style::modern().into(),
            has_inner: true,
        }
    }

    pub fn light() -> TableTheme {
        let theme = Style::blank()
            .horizontals([HorizontalLine::new(
                1,
                Line::new(Some('─'), Some('─'), None, None),
            )])
            .into();
        Self {
            theme,
            full_theme: Style::modern().into(),
            has_inner: true,
        }
    }

    pub fn psql() -> TableTheme {
        Self {
            theme: Style::psql().into(),
            full_theme: Style::psql().into(),
            has_inner: true,
        }
    }

    pub fn markdown() -> TableTheme {
        Self {
            theme: Style::markdown().into(),
            full_theme: Style::markdown().into(),
            has_inner: true,
        }
    }

    pub fn dots() -> TableTheme {
        let theme = Style::dots().remove_horizontal().into();
        Self {
            theme,
            full_theme: Style::dots().into(),
            has_inner: true,
        }
    }

    pub fn restructured() -> TableTheme {
        Self {
            theme: Style::re_structured_text().into(),
            full_theme: Style::re_structured_text().into(),
            has_inner: true,
        }
    }

    pub fn ascii_rounded() -> TableTheme {
        Self {
            theme: Style::ascii_rounded().into(),
            full_theme: Style::ascii_rounded().into(),
            has_inner: true,
        }
    }

    pub fn basic_compact() -> TableTheme {
        let theme = Style::ascii().remove_horizontal().into();
        Self {
            theme,
            full_theme: Style::ascii().into(),
            has_inner: true,
        }
    }

    pub fn compact() -> TableTheme {
        let theme = Style::modern()
            .remove_left()
            .remove_right()
            .remove_horizontal()
            .horizontals([HorizontalLine::new(1, Style::modern().get_horizontal())
                .left(None)
                .right(None)])
            .into();
        Self {
            theme,
            full_theme: Style::modern().into(),
            has_inner: true,
        }
    }

    pub fn with_love() -> TableTheme {
        let theme = Style::empty()
            .top('❤')
            .bottom('❤')
            .vertical('❤')
            .horizontals([HorizontalLine::new(
                1,
                Line::new(Some('❤'), Some('❤'), None, None),
            )]);

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

        Self {
            theme: theme.into(),
            full_theme: full_theme.into(),
            has_inner: true,
        }
    }

    pub fn compact_double() -> TableTheme {
        let theme = Style::extended()
            .remove_left()
            .remove_right()
            .remove_horizontal()
            .horizontals([HorizontalLine::new(1, Style::extended().get_horizontal())
                .left(None)
                .right(None)])
            .into();
        Self {
            theme,
            full_theme: Style::extended().into(),
            has_inner: true,
        }
    }

    pub fn rounded() -> TableTheme {
        Self {
            theme: Style::rounded().into(),
            full_theme: Style::modern()
                .corner_top_left('╭')
                .corner_top_right('╮')
                .corner_bottom_left('╰')
                .corner_bottom_right('╯')
                .into(),
            has_inner: true,
        }
    }

    pub fn reinforced() -> TableTheme {
        let full_theme = Style::modern()
            .corner_top_left('┏')
            .corner_top_right('┓')
            .corner_bottom_left('┗')
            .corner_bottom_right('┛');
        Self {
            theme: full_theme.clone().remove_horizontal().into(),
            full_theme: full_theme.into(),
            has_inner: true,
        }
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
            .horizontals([HorizontalLine::new(1, Line::full('━', '╋', '┣', '┫'))]);
        let full_theme = theme
            .clone()
            .remove_horizontals()
            .horizontal('━')
            .intersection_left('┣')
            .intersection_right('┫')
            .intersection('╋');
        Self {
            theme: theme.into(),
            full_theme: full_theme.into(),
            has_inner: true,
        }
    }

    pub fn none() -> TableTheme {
        Self {
            theme: Style::blank().into(),
            full_theme: Style::blank().into(),
            has_inner: true,
        }
    }

    pub fn has_top_line(&self) -> bool {
        self.theme.get_top().is_some()
            || self.theme.get_top_intersection().is_some()
            || self.theme.get_top_left().is_some()
            || self.theme.get_top_right().is_some()
    }

    pub fn has_left(&self) -> bool {
        self.theme.get_left().is_some()
            || self.theme.get_left_intersection().is_some()
            || self.theme.get_top_left().is_some()
            || self.theme.get_bottom_left().is_some()
    }

    pub fn has_right(&self) -> bool {
        self.theme.get_right().is_some()
            || self.theme.get_right_intersection().is_some()
            || self.theme.get_top_right().is_some()
            || self.theme.get_bottom_right().is_some()
    }

    pub fn has_inner(&self) -> bool {
        self.has_inner
    }

    pub fn has_horizontals(&self) -> bool {
        self.full_theme.get_borders().has_horizontal()
    }

    pub fn get_theme_full(&self) -> RawStyle {
        self.full_theme.clone()
    }

    pub fn get_theme(&self) -> RawStyle {
        self.theme.clone()
    }
}
