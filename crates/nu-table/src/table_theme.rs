use tabled::{
    style::RawStyle,
    style::{HorizontalLine, Line, Style},
};

#[derive(Debug, Clone)]
pub struct TableTheme {
    pub(crate) theme: RawStyle,
    has_inner: bool,
}

impl TableTheme {
    pub fn basic() -> TableTheme {
        Self {
            theme: Style::ascii().into(),
            has_inner: true,
        }
    }

    pub fn thin() -> TableTheme {
        Self {
            theme: Style::modern().into(),
            has_inner: true,
        }
    }

    pub fn light() -> TableTheme {
        Self {
            theme: Style::blank()
                .horizontals([HorizontalLine::new(
                    1,
                    Line::new(Some('─'), Some('─'), None, None),
                )])
                .into(),

            has_inner: true,
        }
    }

    pub fn compact() -> TableTheme {
        Self {
            theme: Style::modern()
                .off_left()
                .off_right()
                .off_horizontal()
                .horizontals([HorizontalLine::new(1, Style::modern().get_horizontal())
                    .left(None)
                    .right(None)])
                .into(),
            has_inner: true,
        }
    }

    pub fn with_love() -> TableTheme {
        Self {
            theme: Style::empty()
                .top('❤')
                .bottom('❤')
                .vertical('❤')
                .horizontals([HorizontalLine::new(
                    1,
                    Line::new(Some('❤'), Some('❤'), None, None),
                )])
                .into(),
            has_inner: true,
        }
    }

    pub fn compact_double() -> TableTheme {
        Self {
            theme: Style::extended()
                .off_left()
                .off_right()
                .off_horizontal()
                .horizontals([HorizontalLine::new(1, Style::extended().get_horizontal())
                    .left(None)
                    .right(None)])
                .into(),
            has_inner: true,
        }
    }

    pub fn rounded() -> TableTheme {
        Self {
            theme: Style::rounded().into(),
            has_inner: true,
        }
    }

    pub fn reinforced() -> TableTheme {
        Self {
            theme: Style::modern()
                .top_left_corner('┏')
                .top_right_corner('┓')
                .bottom_left_corner('┗')
                .bottom_right_corner('┛')
                .off_horizontal()
                .into(),
            has_inner: true,
        }
    }

    pub fn heavy() -> TableTheme {
        Self {
            theme: Style::empty()
                .top('━')
                .bottom('━')
                .vertical('┃')
                .left('┃')
                .right('┃')
                .top_intersection('┳')
                .bottom_intersection('┻')
                .top_left_corner('┏')
                .top_right_corner('┓')
                .bottom_left_corner('┗')
                .bottom_right_corner('┛')
                .horizontals([HorizontalLine::new(1, Line::full('━', '╋', '┣', '┫'))])
                .into(),
            has_inner: true,
        }
    }

    pub fn none() -> TableTheme {
        Self {
            theme: Style::blank().into(),
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
}
