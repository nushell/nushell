use tabled::Style;

#[derive(Debug, Clone)]
pub struct TableTheme {
    pub(crate) theme: tabled::style::StyleConfig,
    pub(crate) is_left_set: bool,
    pub(crate) is_right_set: bool,
}

impl TableTheme {
    pub fn basic() -> TableTheme {
        Self {
            theme: Style::ascii().into(),
            is_left_set: true,
            is_right_set: true,
        }
    }

    pub fn thin() -> TableTheme {
        Self {
            theme: Style::modern().into(),
            is_left_set: true,
            is_right_set: true,
        }
    }

    pub fn light() -> TableTheme {
        Self {
            theme: Style::blank().header('─').into(),
            is_left_set: false,
            is_right_set: false,
        }
    }

    pub fn compact() -> TableTheme {
        Self {
            theme: Style::modern()
                .left_off()
                .right_off()
                .horizontal_off()
                .into(),
            is_left_set: false,
            is_right_set: false,
        }
    }

    pub fn with_love() -> TableTheme {
        Self {
            theme: Style::psql()
                .header('❤')
                .top('❤')
                .bottom('❤')
                .vertical('❤')
                .into(),
            is_left_set: false,
            is_right_set: false,
        }
    }

    pub fn compact_double() -> TableTheme {
        Self {
            theme: Style::psql()
                .header('═')
                .top('═')
                .bottom('═')
                .vertical('║')
                .top_intersection('╦')
                .bottom_intersection('╩')
                .header_intersection('╬')
                .into(),
            is_left_set: false,
            is_right_set: false,
        }
    }

    pub fn rounded() -> TableTheme {
        Self {
            theme: Style::rounded().into(),
            is_left_set: true,
            is_right_set: true,
        }
    }

    pub fn reinforced() -> TableTheme {
        Self {
            theme: Style::modern()
                .top_left_corner('┏')
                .top_right_corner('┓')
                .bottom_left_corner('┗')
                .bottom_right_corner('┛')
                .horizontal_off()
                .into(),
            is_left_set: true,
            is_right_set: true,
        }
    }

    pub fn heavy() -> TableTheme {
        Self {
            theme: Style::modern()
                .header('━')
                .top('━')
                .bottom('━')
                .vertical('┃')
                .left('┃')
                .right('┃')
                .left_intersection('┣')
                .right_intersection('┫')
                .bottom_intersection('┻')
                .top_intersection('┳')
                .top_left_corner('┏')
                .top_right_corner('┓')
                .bottom_left_corner('┗')
                .bottom_right_corner('┛')
                .header_intersection('╋')
                .horizontal_off()
                .into(),
            is_left_set: true,
            is_right_set: true,
        }
    }

    pub fn none() -> TableTheme {
        Self {
            theme: Style::blank().into(),
            is_left_set: false,
            is_right_set: false,
        }
    }
}
