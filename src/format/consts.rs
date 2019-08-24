use lazy_static::lazy_static;

use prettytable::format::{FormatBuilder, LinePosition, LineSeparator, TableFormat};
 
lazy_static! {
    pub(crate) static ref TABLE_FORMAT: TableFormat =
        FormatBuilder::new()
            .column_separator('│')
            .separator(LinePosition::Top, LineSeparator::new('━', '┯', ' ', ' '))
            .separator(LinePosition::Title, LineSeparator::new('─', '┼', ' ', ' '))
            .separator(LinePosition::Bottom, LineSeparator::new('━', '┷', ' ', ' '))
            .padding(1, 1)
            .build();
}
