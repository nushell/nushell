use crate::StyledString;

#[derive(Debug, Default, Clone)]
pub struct Data {
    records: Vec<Vec<StyledString>>,
    size: (usize, usize),
}

impl Data {
    pub fn new(count_rows: usize, count_columns: usize) -> Self {
        Self {
            records: Vec::with_capacity(count_rows),
            size: (count_rows, count_columns),
        }
    }
}
