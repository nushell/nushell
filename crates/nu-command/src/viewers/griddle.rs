use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, PathMember},
    engine::{Command, EvaluationContext},
    Signature, Span, SyntaxShape, Value,
};
use nu_term_grid::grid::{Alignment, Cell, Direction, Filling, Grid, GridOptions};
use terminal_size::{Height, Width};

pub struct Griddle;

impl Command for Griddle {
    fn name(&self) -> &str {
        "grid"
    }

    fn usage(&self) -> &str {
        "Render the grid."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("grid").named(
            "columns",
            SyntaxShape::Int,
            "number of columns wide",
            Some('c'),
        )
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        let columns_param: Option<String> = call.get_flag(context, "columns")?;

        match input {
            Value::List { vals, .. } => {
                // dbg!("value::list");
                let data = convert_to_list(vals);
                if let Some(items) = data {
                    let mut grid = Grid::new(GridOptions {
                        direction: Direction::TopToBottom,
                        filling: Filling::Text(" | ".into()),
                    });
                    for list in items {
                        // looks like '&list = [ "0", "one",]'
                        let a_string = (&list[1]).to_string();
                        let mut cell = Cell::from(a_string);
                        cell.alignment = Alignment::Right;
                        grid.add(cell);
                    }

                    let cols = if let Some(col) = columns_param {
                        col.parse::<u16>().unwrap_or(80)
                    } else {
                        // 80usize
                        if let Some((Width(w), Height(_h))) = terminal_size::terminal_size() {
                            w
                        } else {
                            80u16
                        }
                    };

                    // eprintln!("columns size = {}", cols);
                    if let Some(grid_display) = grid.fit_into_width(cols as usize) {
                        // println!("{}", grid_display);
                        Ok(Value::String {
                            val: grid_display.to_string(),
                            span: call.head,
                        })
                    } else {
                        // println!("Couldn't fit grid into 80 columns!");
                        Ok(Value::String {
                            val: format!("Couldn't fit grid into {} columns!", cols),
                            span: call.head,
                        })
                    }
                } else {
                    Ok(Value::Nothing { span: call.head })
                }
            }
            Value::Stream { stream, .. } => {
                // dbg!("value::stream");
                let data = convert_to_list(stream);
                if let Some(items) = data {
                    let mut grid = Grid::new(GridOptions {
                        direction: Direction::TopToBottom,
                        filling: Filling::Text(" | ".into()),
                    });

                    for list in items {
                        // dbg!(&list);
                        // from the output of ls, it looks like
                        // '&list = [ "0", ".git", "Dir", "4.1 KB", "23 minutes ago",]'
                        // so we take the 1th index for the file name
                        // but this [[col1 col2]; [one two] [three four]] | grid
                        // prints one | three
                        // TODO: what should we do about tables in the grid? should we
                        // allow one to specify a column or perhaps all columns?
                        let a_string = (&list[1]).to_string(); // bytes ->, &h[3]);
                        let mut cell = Cell::from(a_string);
                        cell.alignment = Alignment::Right;
                        grid.add(cell);
                    }

                    let cols = if let Some(col) = columns_param {
                        col.parse::<u16>().unwrap_or(80)
                    } else {
                        // 80usize
                        if let Some((Width(w), Height(_h))) = terminal_size::terminal_size() {
                            w
                        } else {
                            80u16
                        }
                    };

                    // eprintln!("columns size = {}", cols);
                    if let Some(grid_display) = grid.fit_into_width(cols as usize) {
                        // println!("{}", grid_display);
                        Ok(Value::String {
                            val: grid_display.to_string(),
                            span: call.head,
                        })
                    } else {
                        // println!("Couldn't fit grid into 80 columns!");
                        Ok(Value::String {
                            val: format!("Couldn't fit grid into {} columns!", cols),
                            span: call.head,
                        })
                    }
                } else {
                    // dbg!(data);
                    Ok(Value::Nothing { span: call.head })
                }
            }
            Value::Record {
                cols: _, vals: _, ..
            } => {
                dbg!("value::record");

                // let mut output = vec![];

                // for (c, v) in cols.into_iter().zip(vals.into_iter()) {
                //     output.push(vec![
                //         StyledString {
                //             contents: c,
                //             style: nu_table::TextStyle::default_field(),
                //         },
                //         StyledString {
                //             contents: v.into_string(),
                //             style: nu_table::TextStyle::default(),
                //         },
                //     ])
                // }

                // let table = nu_table::Table {
                //     headers: vec![],
                //     data: output,
                //     theme: nu_table::Theme::rounded(),
                // };

                // let result = nu_table::draw_table(&table, 80, &HashMap::new());

                // Ok(Value::String {
                //     val: result,
                //     span: call.head,
                // })
                Ok(Value::Nothing { span: call.head })
            }
            x => {
                // dbg!("other value");
                // dbg!(x.get_type());
                Ok(x)
            }
        }
    }
}

fn convert_to_list(iter: impl IntoIterator<Item = Value>) -> Option<Vec<Vec<String>>> {
    let mut iter = iter.into_iter().peekable();
    let mut data = vec![];

    if let Some(first) = iter.peek() {
        // dbg!(&first);
        let mut headers = first.columns();

        if !headers.is_empty() {
            headers.insert(0, "#".into());
        }

        for (row_num, item) in iter.enumerate() {
            let mut row = vec![row_num.to_string()];

            if headers.is_empty() {
                row.push(item.into_string())
            } else {
                for header in headers.iter().skip(1) {
                    let result = match item {
                        Value::Record { .. } => {
                            item.clone().follow_cell_path(&[PathMember::String {
                                val: header.into(),
                                span: Span::unknown(),
                            }])
                        }
                        _ => Ok(item.clone()),
                    };

                    match result {
                        Ok(value) => row.push(value.into_string()),
                        Err(_) => row.push(String::new()),
                    }
                }
            }

            data.push(row);
        }

        Some(data)
    } else {
        None
    }
    //     Some(nu_table::Table {
    //         headers: headers
    //             .into_iter()
    //             .map(|x| StyledString {
    //                 contents: x,
    //                 style: nu_table::TextStyle::default_header(),
    //             })
    //             .collect(),
    //         data: data
    //             .into_iter()
    //             .map(|x| {
    //                 x.into_iter()
    //                     .enumerate()
    //                     .map(|(col, y)| {
    //                         if col == 0 {
    //                             StyledString {
    //                                 contents: y,
    //                                 style: nu_table::TextStyle::default_header(),
    //                             }
    //                         } else {
    //                             StyledString {
    //                                 contents: y,
    //                                 style: nu_table::TextStyle::basic_left(),
    //                             }
    //                         }
    //                     })
    //                     .collect::<Vec<StyledString>>()
    //             })
    //             .collect(),
    //         theme: nu_table::Theme::rounded(),
    //     })
    // } else {
    //     None
    // }
}
// fn convert_to_table(iter: impl IntoIterator<Item = Value>) -> Option<nu_table::Table> {
//     let mut iter = iter.into_iter().peekable();

//     if let Some(first) = iter.peek() {
//         let mut headers = first.columns();

//         if !headers.is_empty() {
//             headers.insert(0, "#".into());
//         }

//         let mut data = vec![];

//         for (row_num, item) in iter.enumerate() {
//             let mut row = vec![row_num.to_string()];

//             if headers.is_empty() {
//                 row.push(item.into_string())
//             } else {
//                 for header in headers.iter().skip(1) {
//                     let result = match item {
//                         Value::Record { .. } => {
//                             item.clone().follow_cell_path(&[PathMember::String {
//                                 val: header.into(),
//                                 span: Span::unknown(),
//                             }])
//                         }
//                         _ => Ok(item.clone()),
//                     };

//                     match result {
//                         Ok(value) => row.push(value.into_string()),
//                         Err(_) => row.push(String::new()),
//                     }
//                 }
//             }

//             data.push(row);
//         }

//         Some(nu_table::Table {
//             headers: headers
//                 .into_iter()
//                 .map(|x| StyledString {
//                     contents: x,
//                     style: nu_table::TextStyle::default_header(),
//                 })
//                 .collect(),
//             data: data
//                 .into_iter()
//                 .map(|x| {
//                     x.into_iter()
//                         .enumerate()
//                         .map(|(col, y)| {
//                             if col == 0 {
//                                 StyledString {
//                                     contents: y,
//                                     style: nu_table::TextStyle::default_header(),
//                                 }
//                             } else {
//                                 StyledString {
//                                     contents: y,
//                                     style: nu_table::TextStyle::basic_left(),
//                                 }
//                             }
//                         })
//                         .collect::<Vec<StyledString>>()
//                 })
//                 .collect(),
//             theme: nu_table::Theme::rounded(),
//         })
//     } else {
//         None
//     }
// }
