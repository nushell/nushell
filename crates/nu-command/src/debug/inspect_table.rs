// note: Seems like could be simplified
//       IMHO: it shall not take 300+ lines :)
// TODO: Simplify
// NOTE: Pool table could be used?
// FIXME: `inspect` wrapping produces too much new lines with small terminal

use self::global_horizontal_char::SetHorizontalChar;
use nu_protocol::Value;
use nu_protocol::engine::EngineState;
use nu_table::{string_width, string_wrap};
use tabled::{
    Table,
    grid::config::ColoredConfig,
    settings::{Style, peaker::Priority, width::Wrap},
};

pub fn build_table(
    engine_state: &EngineState,
    value: Value,
    description: String,
    termsize: usize,
) -> String {
    let (head, mut data) = util::collect_input(engine_state, value);
    let count_columns = head.len();
    data.insert(0, head);

    let mut desc = description;
    let mut desc_width = string_width(&desc);
    let mut desc_table_width = get_total_width_2_column_table(11, desc_width);

    let cfg = Table::default().with(Style::modern()).get_config().clone();
    let mut widths = get_data_widths(&data, count_columns);
    truncate_data(&mut data, &mut widths, &cfg, termsize);

    let val_table_width = get_total_width2(&widths, &cfg);
    if val_table_width < desc_table_width {
        increase_widths(&mut widths, desc_table_width - val_table_width);
        increase_data_width(&mut data, &widths);
    }

    if val_table_width > desc_table_width {
        desc_width += val_table_width - desc_table_width;
        increase_string_width(&mut desc, desc_width);
    }

    if desc_table_width > termsize {
        let delete_width = desc_table_width - termsize;
        if delete_width >= desc_width {
            // we can't fit in a description; we consider it's no point in showing then?
            return String::new();
        }

        desc_width -= delete_width;
        desc = string_wrap(&desc, desc_width, false);
        desc_table_width = termsize;
    }

    add_padding_to_widths(&mut widths);

    let width = val_table_width.max(desc_table_width).min(termsize);

    let mut desc_table = Table::from_iter([[String::from("description"), desc]]);
    desc_table.with(Style::rounded().remove_bottom().remove_horizontals());

    let mut val_table = Table::from_iter(data);
    val_table.get_dimension_mut().set_widths(widths);
    val_table.with(Style::rounded().corner_top_left('├').corner_top_right('┤'));
    val_table.with((
        Wrap::new(width).priority(Priority::max(true)),
        SetHorizontalChar::new('┼', '┴', 11 + 2 + 1),
    ));

    format!("{desc_table}\n{val_table}")
}

fn get_data_widths(data: &[Vec<String>], count_columns: usize) -> Vec<usize> {
    let mut widths = vec![0; count_columns];
    for row in data {
        for col in 0..count_columns {
            let text = &row[col];
            let width = string_width(text);
            widths[col] = std::cmp::max(widths[col], width);
        }
    }

    widths
}

fn add_padding_to_widths(widths: &mut [usize]) {
    for width in widths {
        *width += 2;
    }
}

fn increase_widths(widths: &mut [usize], need: usize) {
    let all = need / widths.len();
    let mut rest = need - all * widths.len();

    for width in widths {
        *width += all;

        if rest > 0 {
            *width += 1;
            rest -= 1;
        }
    }
}

fn increase_data_width(data: &mut Vec<Vec<String>>, widths: &[usize]) {
    for row in data {
        for (col, max_width) in widths.iter().enumerate() {
            let text = &mut row[col];
            increase_string_width(text, *max_width);
        }
    }
}

fn increase_string_width(text: &mut String, total: usize) {
    let width = string_width(text);
    let rest = total - width;

    if rest > 0 {
        text.extend(std::iter::repeat_n(' ', rest));
    }
}

fn get_total_width_2_column_table(col1: usize, col2: usize) -> usize {
    const PAD: usize = 1;
    const SPLIT_LINE: usize = 1;
    SPLIT_LINE + PAD + col1 + PAD + SPLIT_LINE + PAD + col2 + PAD + SPLIT_LINE
}

fn truncate_data(
    data: &mut Vec<Vec<String>>,
    widths: &mut Vec<usize>,
    cfg: &ColoredConfig,
    expected_width: usize,
) {
    const SPLIT_LINE_WIDTH: usize = 1;
    const PAD: usize = 2;

    let total_width = get_total_width2(widths, cfg);
    if total_width <= expected_width {
        return;
    }

    let mut width = 0;
    let mut peak_count = 0;
    for column_width in widths.iter() {
        let next_width = width + *column_width + SPLIT_LINE_WIDTH + PAD;
        if next_width >= expected_width {
            break;
        }

        width = next_width;
        peak_count += 1;
    }

    debug_assert!(peak_count < widths.len());

    let left_space = expected_width - width;
    let has_space_for_truncation_column = left_space > PAD;
    if !has_space_for_truncation_column {
        peak_count = peak_count.saturating_sub(1);
    }

    remove_columns(data, peak_count);
    widths.drain(peak_count..);
    push_empty_column(data);
    widths.push(1);
}

fn remove_columns(data: &mut Vec<Vec<String>>, peak_count: usize) {
    if peak_count == 0 {
        for row in data {
            row.clear();
        }
    } else {
        for row in data {
            row.drain(peak_count..);
        }
    }
}

fn get_total_width2(widths: &[usize], cfg: &ColoredConfig) -> usize {
    let pad = 2;
    let total = widths.iter().sum::<usize>() + pad * widths.len();
    let countv = cfg.count_vertical(widths.len());
    let margin = cfg.get_margin();

    total + countv + margin.left.size + margin.right.size
}

fn push_empty_column(data: &mut Vec<Vec<String>>) {
    let empty_cell = String::from("‥");
    for row in data {
        row.push(empty_cell.clone());
    }
}

mod util {
    use crate::debug::explain::debug_string_without_formatting;
    use nu_engine::get_columns;
    use nu_protocol::Value;
    use nu_protocol::engine::EngineState;

    /// Try to build column names and a table grid.
    pub fn collect_input(
        engine_state: &EngineState,
        value: Value,
    ) -> (Vec<String>, Vec<Vec<String>>) {
        let span = value.span();
        match value {
            Value::Record { val: record, .. } => {
                let (cols, vals): (Vec<_>, Vec<_>) = record.into_owned().into_iter().unzip();
                (
                    match cols.is_empty() {
                        true => vec![String::from("")],
                        false => cols,
                    },
                    match vals
                        .into_iter()
                        .map(|s| debug_string_without_formatting(engine_state, &s))
                        .collect::<Vec<String>>()
                    {
                        vals if vals.is_empty() => vec![],
                        vals => vec![vals],
                    },
                )
            }
            Value::List { vals, .. } => {
                let mut columns = get_columns(&vals);
                let data = convert_records_to_dataset(engine_state, &columns, vals);

                if columns.is_empty() {
                    columns = vec![String::from("")];
                }

                (columns, data)
            }
            Value::String { val, .. } => {
                let lines = val
                    .lines()
                    .map(|line| Value::string(line.to_string(), span))
                    .map(|val| vec![debug_string_without_formatting(engine_state, &val)])
                    .collect();

                (vec![String::from("")], lines)
            }
            Value::Nothing { .. } => (vec![], vec![]),
            value => (
                vec![String::from("")],
                vec![vec![debug_string_without_formatting(engine_state, &value)]],
            ),
        }
    }

    fn convert_records_to_dataset(
        engine_state: &EngineState,
        cols: &[String],
        records: Vec<Value>,
    ) -> Vec<Vec<String>> {
        if !cols.is_empty() {
            create_table_for_record(engine_state, cols, &records)
        } else if cols.is_empty() && records.is_empty() {
            vec![]
        } else if cols.len() == records.len() {
            vec![
                records
                    .into_iter()
                    .map(|s| debug_string_without_formatting(engine_state, &s))
                    .collect(),
            ]
        } else {
            records
                .into_iter()
                .map(|record| vec![debug_string_without_formatting(engine_state, &record)])
                .collect()
        }
    }

    fn create_table_for_record(
        engine_state: &EngineState,
        headers: &[String],
        items: &[Value],
    ) -> Vec<Vec<String>> {
        let mut data = vec![Vec::new(); items.len()];

        for (i, item) in items.iter().enumerate() {
            let row = record_create_row(engine_state, headers, item);
            data[i] = row;
        }

        data
    }

    fn record_create_row(
        engine_state: &EngineState,
        headers: &[String],
        item: &Value,
    ) -> Vec<String> {
        if let Value::Record { val, .. } = item {
            headers
                .iter()
                .map(|col| {
                    val.get(col)
                        .map(|v| debug_string_without_formatting(engine_state, v))
                        .unwrap_or_else(String::new)
                })
                .collect()
        } else {
            // should never reach here due to `get_columns` above which will return
            // empty columns if any value in the list is not a record
            vec![String::new(); headers.len()]
        }
    }
}

mod global_horizontal_char {
    use nu_table::NuRecords;
    use tabled::{
        grid::{
            config::{ColoredConfig, Offset, Position},
            dimension::{CompleteDimension, Dimension},
            records::{ExactRecords, Records},
        },
        settings::TableOption,
    };

    pub struct SetHorizontalChar {
        intersection: char,
        split: char,
        index: usize,
    }

    impl SetHorizontalChar {
        pub fn new(intersection: char, split: char, index: usize) -> Self {
            Self {
                intersection,
                split,
                index,
            }
        }
    }

    impl TableOption<NuRecords, ColoredConfig, CompleteDimension> for SetHorizontalChar {
        fn change(
            self,
            records: &mut NuRecords,
            cfg: &mut ColoredConfig,
            dimension: &mut CompleteDimension,
        ) {
            let count_columns = records.count_columns();
            let count_rows = records.count_rows();

            if count_columns == 0 || count_rows == 0 {
                return;
            }

            let widths = get_widths(dimension, records.count_columns());

            let has_vertical = cfg.has_vertical(0, count_columns);
            if has_vertical && self.index == 0 {
                let mut border = cfg.get_border(Position::new(0, 0), (count_rows, count_columns));
                border.left_top_corner = Some(self.intersection);
                cfg.set_border(Position::new(0, 0), border);
                return;
            }

            let mut i = 1;
            for (col, width) in widths.into_iter().enumerate() {
                if self.index < i + width {
                    let o = self.index - i;
                    cfg.set_horizontal_char(Position::new(0, col), Offset::Start(o), self.split);
                    return;
                }

                i += width;

                let has_vertical = cfg.has_vertical(col, count_columns);
                if has_vertical {
                    if self.index == i {
                        let mut border =
                            cfg.get_border(Position::new(0, col), (count_rows, count_columns));
                        border.right_top_corner = Some(self.intersection);
                        cfg.set_border(Position::new(0, col), border);
                        return;
                    }

                    i += 1;
                }
            }
        }
    }

    fn get_widths(dims: &CompleteDimension, count_columns: usize) -> Vec<usize> {
        let mut widths = vec![0; count_columns];
        for (col, width) in widths.iter_mut().enumerate() {
            *width = dims.get_width(col);
        }

        widths
    }
}
