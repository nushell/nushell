use nu_protocol::Value;
use tabled::{
    builder::Builder,
    peaker::PriorityMax,
    width::{MinWidth, Wrap},
    Style,
};

use self::{
    global_horizontal_char::SetHorizontalChar, peak2::Peak2, table_column_width::GetColumnWidths,
    truncate_table::TruncateTable, width_increase::IncWidth,
};

pub fn build_table(value: Value, description: String, termsize: usize) -> String {
    let (head, mut data) = util::collect_input(value);
    data.insert(0, head);

    let mut val_table = Builder::from(data).build();
    let val_table_width = val_table.total_width();

    let desc = vec![vec![String::from("description"), description]];

    let mut desc_table = Builder::from(desc).build();
    let desc_table_width = desc_table.total_width();

    let width = val_table_width.clamp(desc_table_width, termsize);

    desc_table
        .with(Style::rounded().off_bottom())
        .with(Wrap::new(width).priority::<PriorityMax>())
        .with(MinWidth::new(width).priority::<Peak2>());

    val_table
        .with(Style::rounded().top_left_corner('├').top_right_corner('┤'))
        .with(TruncateTable(width))
        .with(Wrap::new(width).priority::<PriorityMax>())
        .with(IncWidth(width));

    let mut desc_widths = GetColumnWidths(Vec::new());
    desc_table.with(&mut desc_widths);

    val_table.with(SetHorizontalChar::new('┼', '┴', 0, desc_widths.0[0]));

    format!("{desc_table}\n{val_table}")
}

mod truncate_table {
    use tabled::{
        papergrid::{
            records::{Records, RecordsMut, Resizable},
            width::{CfgWidthFunction, WidthEstimator},
            Estimate,
        },
        TableOption,
    };

    pub struct TruncateTable(pub usize);

    impl<R> TableOption<R> for TruncateTable
    where
        R: Records + RecordsMut<String> + Resizable,
    {
        fn change(&mut self, table: &mut tabled::Table<R>) {
            let width = table.total_width();
            if width <= self.0 {
                return;
            }

            let count_columns = table.get_records().count_columns();
            if count_columns < 1 {
                return;
            }

            let mut evaluator = WidthEstimator::default();
            evaluator.estimate(table.get_records(), table.get_config());
            let columns_width: Vec<_> = evaluator.into();

            const SPLIT_LINE_WIDTH: usize = 1;
            let mut width = 0;
            let mut i = 0;
            for w in columns_width {
                width += w + SPLIT_LINE_WIDTH;

                if width >= self.0 {
                    break;
                }

                i += 1;
            }

            if i == 0 && count_columns > 0 {
                i = 1;
            } else if i + 1 == count_columns {
                // we want to left at least 1 column
                i -= 1;
            }

            let count_columns = table.get_records().count_columns();
            let y = count_columns - i;

            let mut column = count_columns;
            for _ in 0..y {
                column -= 1;
                table.get_records_mut().remove_column(column);
            }

            table.get_records_mut().push_column();

            let width_ctrl = CfgWidthFunction::from_cfg(table.get_config());
            let last_column = table.get_records().count_columns() - 1;
            for row in 0..table.get_records().count_rows() {
                table
                    .get_records_mut()
                    .set((row, last_column), String::from("‥"), &width_ctrl)
            }
        }
    }
}

mod util {
    use crate::debug::explain::debug_string_without_formatting;
    use nu_engine::get_columns;
    use nu_protocol::{ast::PathMember, Span, Value};

    /// Try to build column names and a table grid.
    pub fn collect_input(value: Value) -> (Vec<String>, Vec<Vec<String>>) {
        match value {
            Value::Record { cols, vals, .. } => (
                cols,
                vec![vals
                    .into_iter()
                    .map(|s| debug_string_without_formatting(&s))
                    .collect()],
            ),
            Value::List { vals, .. } => {
                let mut columns = get_columns(&vals);
                let data = convert_records_to_dataset(&columns, vals);

                if columns.is_empty() && !data.is_empty() {
                    columns = vec![String::from("")];
                }

                (columns, data)
            }
            Value::String { val, span } => {
                let lines = val
                    .lines()
                    .map(|line| Value::String {
                        val: line.to_string(),
                        span,
                    })
                    .map(|val| vec![debug_string_without_formatting(&val)])
                    .collect();

                (vec![String::from("")], lines)
            }
            Value::Nothing { .. } => (vec![], vec![]),
            value => (
                vec![String::from("")],
                vec![vec![debug_string_without_formatting(&value)]],
            ),
        }
    }

    fn convert_records_to_dataset(cols: &Vec<String>, records: Vec<Value>) -> Vec<Vec<String>> {
        if !cols.is_empty() {
            create_table_for_record(cols, &records)
        } else if cols.is_empty() && records.is_empty() {
            vec![]
        } else if cols.len() == records.len() {
            vec![records
                .into_iter()
                .map(|s| debug_string_without_formatting(&s))
                .collect()]
        } else {
            records
                .into_iter()
                .map(|record| vec![debug_string_without_formatting(&record)])
                .collect()
        }
    }

    fn create_table_for_record(headers: &[String], items: &[Value]) -> Vec<Vec<String>> {
        let mut data = vec![Vec::new(); items.len()];

        for (i, item) in items.iter().enumerate() {
            let row = record_create_row(headers, item);
            data[i] = row;
        }

        data
    }

    fn record_create_row(headers: &[String], item: &Value) -> Vec<String> {
        let mut rows = vec![String::default(); headers.len()];

        for (i, header) in headers.iter().enumerate() {
            let value = record_lookup_value(item, header);
            rows[i] = debug_string_without_formatting(&value);
        }

        rows
    }

    fn record_lookup_value(item: &Value, header: &str) -> Value {
        match item {
            Value::Record { .. } => {
                let path = PathMember::String {
                    val: header.to_owned(),
                    span: Span::unknown(),
                    optional: false,
                };

                item.clone()
                    .follow_cell_path(&[path], false, false)
                    .unwrap_or_else(|_| item.clone())
            }
            item => item.clone(),
        }
    }
}

mod style_no_left_right_1st {
    use tabled::{papergrid::records::Records, Table, TableOption};

    struct StyleOffLeftRightFirstLine;

    impl<R> TableOption<R> for StyleOffLeftRightFirstLine
    where
        R: Records,
    {
        fn change(&mut self, table: &mut Table<R>) {
            let shape = table.shape();
            let cfg = table.get_config_mut();

            let mut b = cfg.get_border((0, 0), shape);
            b.left = Some(' ');
            cfg.set_border((0, 0), b);

            let mut b = cfg.get_border((0, shape.1 - 1), shape);
            b.right = Some(' ');
            cfg.set_border((0, 0), b);
        }
    }
}

mod peak2 {
    use tabled::peaker::Peaker;

    pub struct Peak2;

    impl Peaker for Peak2 {
        fn create() -> Self {
            Self
        }

        fn peak(&mut self, _: &[usize], _: &[usize]) -> Option<usize> {
            Some(1)
        }
    }
}

mod table_column_width {
    use tabled::papergrid::{records::Records, Estimate};

    pub struct GetColumnWidths(pub Vec<usize>);

    impl<R> tabled::TableOption<R> for GetColumnWidths
    where
        R: Records,
    {
        fn change(&mut self, table: &mut tabled::Table<R>) {
            let mut evaluator = tabled::papergrid::width::WidthEstimator::default();
            evaluator.estimate(table.get_records(), table.get_config());
            self.0 = evaluator.into();
        }
    }
}

mod global_horizontal_char {
    use tabled::{
        papergrid::{records::Records, width::WidthEstimator, Estimate, Offset::Begin},
        Table, TableOption,
    };

    pub struct SetHorizontalChar {
        c1: char,
        c2: char,
        line: usize,
        position: usize,
    }

    impl SetHorizontalChar {
        pub fn new(c1: char, c2: char, line: usize, position: usize) -> Self {
            Self {
                c1,
                c2,
                line,
                position,
            }
        }
    }

    impl<R> TableOption<R> for SetHorizontalChar
    where
        R: Records,
    {
        fn change(&mut self, table: &mut Table<R>) {
            let shape = table.shape();

            let is_last_line = self.line == (shape.0 * 2);
            let mut row = self.line;
            if is_last_line {
                row = self.line - 1;
            }

            let mut evaluator = WidthEstimator::default();
            evaluator.estimate(table.get_records(), table.get_config());
            let widths: Vec<_> = evaluator.into();

            let mut i = 0;
            #[allow(clippy::needless_range_loop)]
            for column in 0..shape.1 {
                let has_vertical = table.get_config().has_vertical(column, shape.1);

                if has_vertical {
                    if self.position == i {
                        let mut border = table.get_config().get_border((row, column), shape);
                        if is_last_line {
                            border.left_bottom_corner = Some(self.c1);
                        } else {
                            border.left_top_corner = Some(self.c1);
                        }

                        table.get_config_mut().set_border((row, column), border);

                        return;
                    }

                    i += 1;
                }

                let width = widths[column];

                if self.position < i + width {
                    let offset = self.position + 1 - i;
                    // let offset = width - offset;

                    table.get_config_mut().override_horizontal_border(
                        (self.line, column),
                        self.c2,
                        Begin(offset),
                    );

                    return;
                }

                i += width;
            }

            let has_vertical = table.get_config().has_vertical(shape.1, shape.1);
            if self.position == i && has_vertical {
                let mut border = table.get_config().get_border((row, shape.1), shape);
                if is_last_line {
                    border.left_bottom_corner = Some(self.c1);
                } else {
                    border.left_top_corner = Some(self.c1);
                }

                table.get_config_mut().set_border((row, shape.1), border);
            }
        }
    }
}

mod width_increase {
    use tabled::{
        object::Cell,
        papergrid::{
            records::{Records, RecordsMut},
            width::WidthEstimator,
            Entity, Estimate, GridConfig,
        },
        peaker::PriorityNone,
        Modify, Width,
    };

    use tabled::{peaker::Peaker, Table, TableOption};

    #[derive(Debug)]
    pub struct IncWidth(pub usize);

    impl<R> TableOption<R> for IncWidth
    where
        R: Records + RecordsMut<String>,
    {
        fn change(&mut self, table: &mut Table<R>) {
            if table.is_empty() {
                return;
            }

            let (widths, total_width) =
                get_table_widths_with_total(table.get_records(), table.get_config());
            if total_width >= self.0 {
                return;
            }

            let increase_list =
                get_increase_list(widths, self.0, total_width, PriorityNone::default());

            for (col, width) in increase_list.into_iter().enumerate() {
                for row in 0..table.get_records().count_rows() {
                    let pad = table.get_config().get_padding(Entity::Cell(row, col));
                    let width = width - pad.left.size - pad.right.size;

                    table.with(Modify::new(Cell(row, col)).with(Width::increase(width)));
                }
            }
        }
    }

    fn get_increase_list<F>(
        mut widths: Vec<usize>,
        total_width: usize,
        mut width: usize,
        mut peaker: F,
    ) -> Vec<usize>
    where
        F: Peaker,
    {
        while width != total_width {
            let col = match peaker.peak(&[], &widths) {
                Some(col) => col,
                None => break,
            };

            widths[col] += 1;
            width += 1;
        }

        widths
    }

    fn get_table_widths_with_total<R>(records: R, cfg: &GridConfig) -> (Vec<usize>, usize)
    where
        R: Records,
    {
        let mut evaluator = WidthEstimator::default();
        evaluator.estimate(&records, cfg);
        let total_width = get_table_total_width(&records, cfg, &evaluator);
        let widths = evaluator.into();

        (widths, total_width)
    }

    pub(crate) fn get_table_total_width<W, R>(records: R, cfg: &GridConfig, ctrl: &W) -> usize
    where
        W: Estimate<R>,
        R: Records,
    {
        ctrl.total()
            + cfg.count_vertical(records.count_columns())
            + cfg.get_margin().left.size
            + cfg.get_margin().right.size
    }
}
