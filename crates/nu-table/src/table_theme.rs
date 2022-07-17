use tabled::{papergrid::Line, style::RawStyle, Style};

#[derive(Debug, Clone)]
pub struct TableTheme {
    pub(crate) theme: RawStyle,
}

impl TableTheme {
    pub fn basic() -> TableTheme {
        Self {
            theme: Style::ascii().into(),
        }
    }

    pub fn thin() -> TableTheme {
        Self {
            theme: Style::modern().into(),
        }
    }

    pub fn light() -> TableTheme {
        Self {
            theme: Style::blank().lines([(1, Line::short('─', '─'))]).into(),
        }
    }

    pub fn compact() -> TableTheme {
        Self {
            theme: Style::modern()
                .off_left()
                .off_right()
                .off_horizontal()
                .lines([(1, Style::modern().get_horizontal().left(None).right(None))])
                .into(),
        }
    }

    pub fn with_love() -> TableTheme {
        Self {
            theme: Style::empty()
                .top('❤')
                .bottom('❤')
                .vertical('❤')
                .lines([(1, Line::short('❤', '❤'))])
                .into(),
        }
    }

    pub fn compact_double() -> TableTheme {
        Self {
            theme: Style::extended()
                .off_left()
                .off_right()
                .off_horizontal()
                .lines([(1, Style::extended().get_horizontal().left(None).right(None))])
                .into(),
        }
    }

    pub fn rounded() -> TableTheme {
        Self {
            theme: Style::rounded().into(),
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
                .lines([(1, Line::full('━', '╋', '┣', '┫'))])
                .into(),
        }
    }

    pub fn none() -> TableTheme {
        Self {
            theme: Style::blank().into(),
        }
    }
}
