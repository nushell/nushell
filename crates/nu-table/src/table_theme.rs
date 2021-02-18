#[derive(Debug, Clone)]
pub struct TableTheme {
    pub top_left: char,
    pub middle_left: char,
    pub bottom_left: char,
    pub top_center: char,
    pub center: char,
    pub bottom_center: char,
    pub top_right: char,
    pub middle_right: char,
    pub bottom_right: char,
    pub top_horizontal: char,
    pub middle_horizontal: char,
    pub bottom_horizontal: char,
    pub left_vertical: char,
    pub center_vertical: char,
    pub right_vertical: char,

    pub separate_header: bool,
    pub separate_rows: bool,

    pub print_left_border: bool,
    pub print_right_border: bool,
    pub print_top_border: bool,
    pub print_bottom_border: bool,
}

impl TableTheme {
    #[allow(unused)]
    pub fn basic() -> TableTheme {
        TableTheme {
            top_left: '+',
            middle_left: '+',
            bottom_left: '+',
            top_center: '+',
            center: '+',
            bottom_center: '+',
            top_right: '+',
            middle_right: '+',
            bottom_right: '+',
            top_horizontal: '-',
            middle_horizontal: '-',
            bottom_horizontal: '-',
            left_vertical: '|',
            center_vertical: '|',
            right_vertical: '|',

            separate_header: true,
            separate_rows: true,

            print_left_border: true,
            print_right_border: true,
            print_top_border: true,
            print_bottom_border: true,
        }
    }

    #[allow(unused)]
    pub fn thin() -> TableTheme {
        TableTheme {
            top_left: '┌',
            middle_left: '├',
            bottom_left: '└',
            top_center: '┬',
            center: '┼',
            bottom_center: '┴',
            top_right: '┐',
            middle_right: '┤',
            bottom_right: '┘',

            top_horizontal: '─',
            middle_horizontal: '─',
            bottom_horizontal: '─',

            left_vertical: '│',
            center_vertical: '│',
            right_vertical: '│',

            separate_header: true,
            separate_rows: true,

            print_left_border: true,
            print_right_border: true,
            print_top_border: true,
            print_bottom_border: true,
        }
    }

    #[allow(unused)]
    pub fn light() -> TableTheme {
        TableTheme {
            top_left: ' ',
            middle_left: '─',
            bottom_left: ' ',
            top_center: ' ',
            center: '─',
            bottom_center: ' ',
            top_right: ' ',
            middle_right: '─',
            bottom_right: ' ',

            top_horizontal: ' ',
            middle_horizontal: '─',
            bottom_horizontal: ' ',

            left_vertical: ' ',
            center_vertical: ' ',
            right_vertical: ' ',

            separate_header: true,
            separate_rows: false,

            print_left_border: true,
            print_right_border: true,
            print_top_border: false,
            print_bottom_border: true,
        }
    }

    #[allow(unused)]
    pub fn compact() -> TableTheme {
        TableTheme {
            top_left: '─',
            middle_left: '─',
            bottom_left: '─',
            top_center: '┬',
            center: '┼',
            bottom_center: '┴',
            top_right: '─',
            middle_right: '─',
            bottom_right: '─',
            top_horizontal: '─',
            middle_horizontal: '─',
            bottom_horizontal: '─',

            left_vertical: ' ',
            center_vertical: '│',
            right_vertical: ' ',

            separate_header: true,
            separate_rows: false,

            print_left_border: false,
            print_right_border: false,
            print_top_border: true,
            print_bottom_border: true,
        }
    }

    #[allow(unused)]
    pub fn with_love() -> TableTheme {
        TableTheme {
            top_left: '❤',
            middle_left: '❤',
            bottom_left: '❤',
            top_center: '❤',
            center: '❤',
            bottom_center: '❤',
            top_right: '❤',
            middle_right: '❤',
            bottom_right: '❤',
            top_horizontal: '❤',
            middle_horizontal: '❤',
            bottom_horizontal: '❤',

            left_vertical: ' ',
            center_vertical: '❤',
            right_vertical: ' ',

            separate_header: true,
            separate_rows: false,

            print_left_border: false,
            print_right_border: false,
            print_top_border: true,
            print_bottom_border: true,
        }
    }

    #[allow(unused)]
    pub fn compact_double() -> TableTheme {
        TableTheme {
            top_left: '═',
            middle_left: '═',
            bottom_left: '═',
            top_center: '╦',
            center: '╬',
            bottom_center: '╩',
            top_right: '═',
            middle_right: '═',
            bottom_right: '═',
            top_horizontal: '═',
            middle_horizontal: '═',
            bottom_horizontal: '═',

            left_vertical: ' ',
            center_vertical: '║',
            right_vertical: ' ',

            separate_header: true,
            separate_rows: false,

            print_left_border: false,
            print_right_border: false,
            print_top_border: true,
            print_bottom_border: true,
        }
    }

    #[allow(unused)]
    pub fn rounded() -> TableTheme {
        TableTheme {
            top_left: '╭',
            middle_left: '├',
            bottom_left: '╰',
            top_center: '┬',
            center: '┼',
            bottom_center: '┴',
            top_right: '╮',
            middle_right: '┤',
            bottom_right: '╯',
            top_horizontal: '─',
            middle_horizontal: '─',
            bottom_horizontal: '─',

            left_vertical: '│',
            center_vertical: '│',
            right_vertical: '│',

            separate_header: true,
            separate_rows: false,

            print_left_border: true,
            print_right_border: true,
            print_top_border: true,
            print_bottom_border: true,
        }
    }

    #[allow(unused)]
    pub fn reinforced() -> TableTheme {
        TableTheme {
            top_left: '┏',
            middle_left: '├',
            bottom_left: '┗',
            top_center: '┬',
            center: '┼',
            bottom_center: '┴',
            top_right: '┓',
            middle_right: '┤',
            bottom_right: '┛',
            top_horizontal: '─',
            middle_horizontal: '─',
            bottom_horizontal: '─',

            left_vertical: '│',
            center_vertical: '│',
            right_vertical: '│',

            separate_header: true,
            separate_rows: false,

            print_left_border: true,
            print_right_border: true,
            print_top_border: true,
            print_bottom_border: true,
        }
    }

    #[allow(unused)]
    pub fn heavy() -> TableTheme {
        TableTheme {
            top_left: '┏',
            middle_left: '┣',
            bottom_left: '┗',
            top_center: '┳',
            center: '╋',
            bottom_center: '┻',
            top_right: '┓',
            middle_right: '┫',
            bottom_right: '┛',
            top_horizontal: '━',
            middle_horizontal: '━',
            bottom_horizontal: '━',

            left_vertical: '┃',
            center_vertical: '┃',
            right_vertical: '┃',

            separate_header: true,
            separate_rows: false,

            print_left_border: true,
            print_right_border: true,
            print_top_border: true,
            print_bottom_border: true,
        }
    }
    #[allow(unused)]
    pub fn none() -> TableTheme {
        TableTheme {
            top_left: ' ',
            middle_left: ' ',
            bottom_left: ' ',
            top_center: ' ',
            center: ' ',
            bottom_center: ' ',
            top_right: ' ',
            middle_right: ' ',
            bottom_right: ' ',

            top_horizontal: ' ',
            middle_horizontal: ' ',
            bottom_horizontal: ' ',

            left_vertical: ' ',
            center_vertical: ' ',
            right_vertical: ' ',

            separate_header: false,
            separate_rows: false,

            print_left_border: false,
            print_right_border: false,
            print_top_border: false,
            print_bottom_border: false,
        }
    }
}
