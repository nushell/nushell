use std::borrow::Cow;

// use super::icons::{icon_for_file, iconify_style_ansi_to_nu};
use super::{icons::icon_for_file, table_tui::UITable};
use lscolors::Style;
use nu_engine::{get_columns, CallExt};
use nu_protocol::{
    ast::{Call, PathMember, Pipeline},
    engine::{Command, EngineState, Stack},
    Category, Config, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Value,
};
use nu_term_grid::grid::{Alignment, Cell, Direction, Filling, Grid, GridOptions};
use nu_utils::get_ls_colors;
use terminal_size::{Height, Width};
#[derive(Clone)]
pub struct TableLess;

impl Command for TableLess {
    fn name(&self) -> &str {
        "tabless"
    }

    fn usage(&self) -> &str {
        "11231321123"
    }

    fn signature(&self) -> nu_protocol::Signature {
        // todo: Fix error message when it's empty
        // if we set h i short flags it panics????

        Signature::build("tabless")
            .named("head", SyntaxShape::Boolean, "xxxx", None)
            .named("index", SyntaxShape::Boolean, "asdsad", None)
            .category(Category::Viewers)
    }

    fn extra_usage(&self) -> &str {
        "11231321123"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let show_head: bool = call.get_flag(engine_state, stack, "head")?.unwrap_or(true);
        let show_index: bool = call
            .get_flag(engine_state, stack, "index")?
            .unwrap_or(false);

        let ctrlc = engine_state.ctrlc.clone();
        let config = engine_state.get_config();

        match input {
            PipelineData::Value(value, ..) => {
                let mut tmp_list = Vec::new();
                #[allow(unused_assignments)]
                let mut tmp_list2 = Vec::new();

                let (cols, vals): (&[String], &[Value]) = match &value {
                    Value::Record { cols, vals, .. } => (cols, vals),
                    Value::List { vals, .. } => {
                        tmp_list = get_columns(vals);
                        (&tmp_list, vals)
                    }
                    value => {
                        tmp_list2 = vec![value.clone()];
                        (&tmp_list, &tmp_list2)
                    }
                };

                let table = UITable::new(cols, vals, config, ctrlc, show_index, show_head);
                table.handle().unwrap();
            }
            PipelineData::ListStream(mut stream, ..) => {
                let mut data = vec![];
                for item in stream.by_ref() {
                    data.push(item);
                }

                let cols = get_columns(&data);

                let table = UITable::new(&cols, &data, config, ctrlc, show_index, show_head);
                table.handle().unwrap();
            }
            input => todo!("{:?}", input),
        }

        Ok(PipelineData::Value(Value::default(), None))
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}

/// Removes ANSI escape codes and some ASCII control characters
///
/// Keeps `\n` removes `\r`, `\t` etc.
///
/// If parsing fails silently returns the input string
fn strip_ansi(string: &str) -> Cow<str> {
    // Check if any ascii control character except LF(0x0A = 10) is present,
    // which will be stripped. Includes the primary start of ANSI sequences ESC
    // (0x1B = decimal 27)
    if string.bytes().any(|x| matches!(x, 0..=9 | 11..=31)) {
        if let Ok(stripped) = strip_ansi_escapes::strip(string) {
            if let Ok(new_string) = String::from_utf8(stripped) {
                return Cow::Owned(new_string);
            }
        }
    }
    // Else case includes failures to parse!
    Cow::Borrowed(string)
}

fn create_grid_output(
    items: Vec<(usize, String, String)>,
    call: &Call,
    width_param: Option<i64>,
    color_param: bool,
    separator_param: Option<String>,
    env_str: Option<String>,
    use_grid_icons: bool,
) -> Result<PipelineData, ShellError> {
    let ls_colors = get_ls_colors(env_str);

    let cols = if let Some(col) = width_param {
        col as u16
    } else if let Some((Width(w), Height(_h))) = terminal_size::terminal_size() {
        w
    } else {
        80u16
    };
    let sep = if let Some(separator) = separator_param {
        separator
    } else {
        " │ ".to_string()
    };

    let mut grid = Grid::new(GridOptions {
        direction: Direction::TopToBottom,
        filling: Filling::Text(sep),
    });

    for (_row_index, header, value) in items {
        // only output value if the header name is 'name'
        if header == "name" {
            if color_param {
                if use_grid_icons {
                    let no_ansi = strip_ansi(&value);
                    let path = std::path::Path::new(no_ansi.as_ref());
                    let icon = icon_for_file(path, call.head)?;
                    let ls_colors_style = ls_colors.style_for_path(path);

                    let icon_style = match ls_colors_style {
                        Some(c) => c.to_crossterm_style(),
                        None => crossterm::style::ContentStyle::default(),
                    };

                    let ansi_style = ls_colors_style
                        .map(Style::to_crossterm_style)
                        .unwrap_or_default();

                    let item = format!("{} {}", icon_style.apply(icon), ansi_style.apply(value));

                    let mut cell = Cell::from(item);
                    cell.alignment = Alignment::Left;
                    grid.add(cell);
                } else {
                    let style = ls_colors.style_for_path(value.clone());
                    let ansi_style = style.map(Style::to_crossterm_style).unwrap_or_default();
                    let mut cell = Cell::from(ansi_style.apply(value).to_string());
                    cell.alignment = Alignment::Left;
                    grid.add(cell);
                }
            } else {
                let mut cell = Cell::from(value);
                cell.alignment = Alignment::Left;
                grid.add(cell);
            }
        }
    }

    Ok(
        if let Some(grid_display) = grid.fit_into_width(cols as usize) {
            Value::String {
                val: grid_display.to_string(),
                span: call.head,
            }
        } else {
            Value::String {
                val: format!("Couldn't fit grid into {} columns!", cols),
                span: call.head,
            }
        }
        .into_pipeline_data(),
    )
}

fn convert_to_list(
    iter: impl IntoIterator<Item = Value>,
    config: &Config,
    head: Span,
) -> Option<Vec<(usize, String, String)>> {
    let mut iter = iter.into_iter().peekable();

    if let Some(first) = iter.peek() {
        let mut headers = first.columns();

        if !headers.is_empty() {
            headers.insert(0, "#".into());
        }

        let mut data = vec![];

        for (row_num, item) in iter.enumerate() {
            let mut row = vec![row_num.to_string()];

            if headers.is_empty() {
                row.push(item.into_string(", ", config))
            } else {
                for header in headers.iter().skip(1) {
                    let result = match item {
                        Value::Record { .. } => item.clone().follow_cell_path(
                            &[PathMember::String {
                                val: header.into(),
                                span: head,
                            }],
                            false,
                        ),
                        _ => Ok(item.clone()),
                    };

                    match result {
                        Ok(value) => row.push(value.into_string(", ", config)),
                        Err(_) => row.push(String::new()),
                    }
                }
            }

            data.push(row);
        }

        let mut h: Vec<String> = headers.into_iter().collect();

        // This is just a list
        if h.is_empty() {
            // let's fake the header
            h.push("#".to_string());
            h.push("name".to_string());
        }

        // this tuple is (row_index, header_name, value)
        let mut interleaved = vec![];
        for (i, v) in data.into_iter().enumerate() {
            for (n, s) in v.into_iter().enumerate() {
                if h.len() == 1 {
                    // always get the 1th element since this is a simple list
                    // and we hacked the header above because it was empty
                    // 0th element is an index, 1th element is the value
                    interleaved.push((i, h[1].clone(), s))
                } else {
                    interleaved.push((i, h[n].clone(), s))
                }
            }
        }

        Some(interleaved)
    } else {
        None
    }
}
