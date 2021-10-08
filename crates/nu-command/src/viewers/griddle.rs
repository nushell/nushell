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
        "Renders the output to a textual terminal grid."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("grid").named(
            "columns",
            SyntaxShape::Int,
            "number of columns wide",
            Some('c'),
        )
    }

    fn extra_usage(&self) -> &str {
        r#"grid was built to give a concise gridded layout for ls. however,
it determines what to put in the grid by looking for a column named
'name'. this works great for tables and records but for lists we
need to do something different. such as with '[one two three] | grid'
it creates a fake column called 'name' for these values so that it
prints out the list properly."#
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
                let data = convert_to_list2(vals);
                if let Some(items) = data {
                    Ok(create_grid_output2(items, call, columns_param))
                } else {
                    Ok(Value::Nothing { span: call.head })
                }
            }
            Value::Stream { stream, .. } => {
                // dbg!("value::stream");
                let data = convert_to_list2(stream);
                if let Some(items) = data {
                    Ok(create_grid_output2(items, call, columns_param))
                } else {
                    // dbg!(data);
                    Ok(Value::Nothing { span: call.head })
                }
            }
            Value::Record { cols, vals, .. } => {
                // dbg!("value::record");
                let mut items = vec![];

                for (i, (c, v)) in cols.into_iter().zip(vals.into_iter()).enumerate() {
                    items.push((i, c, v.into_string()))
                }

                Ok(create_grid_output2(items, call, columns_param))
            }
            x => {
                // dbg!("other value");
                // dbg!(x.get_type());
                Ok(x)
            }
        }
    }
}

fn create_grid_output2(
    items: Vec<(usize, String, String)>,
    call: &Call,
    columns_param: Option<String>,
) -> Value {
    let mut grid = Grid::new(GridOptions {
        direction: Direction::TopToBottom,
        filling: Filling::Text(" | ".into()),
    });

    for (_row_index, header, value) in items {
        // only output value if the header name is 'name'
        if header == "name" {
            let mut cell = Cell::from(value);
            cell.alignment = Alignment::Right;
            grid.add(cell);
        }
    }

    let cols = if let Some(col) = columns_param {
        col.parse::<u16>().unwrap_or(80)
    } else if let Some((Width(w), Height(_h))) = terminal_size::terminal_size() {
        w
    } else {
        80u16
    };

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
}

// fn create_grid_output(
//     items: Vec<Vec<String>>,
//     call: &Call,
//     columns_param: Option<String>,
// ) -> Value {
//     let mut grid = Grid::new(GridOptions {
//         direction: Direction::TopToBottom,
//         filling: Filling::Text(" | ".into()),
//     });

//     for list in items {
//         dbg!(&list);
//         // looks like '&list = [ "0", "one",]'
//         let a_string = (&list[1]).to_string();
//         let mut cell = Cell::from(a_string);
//         cell.alignment = Alignment::Right;
//         grid.add(cell);
//     }

//     let cols = if let Some(col) = columns_param {
//         col.parse::<u16>().unwrap_or(80)
//     } else {
//         // 80usize
//         if let Some((Width(w), Height(_h))) = terminal_size::terminal_size() {
//             w
//         } else {
//             80u16
//         }
//     };

//     // eprintln!("columns size = {}", cols);
//     if let Some(grid_display) = grid.fit_into_width(cols as usize) {
//         // println!("{}", grid_display);
//         Value::String {
//             val: grid_display.to_string(),
//             span: call.head,
//         }
//     } else {
//         // println!("Couldn't fit grid into 80 columns!");
//         Value::String {
//             val: format!("Couldn't fit grid into {} columns!", cols),
//             span: call.head,
//         }
//     }
// }

fn convert_to_list2(iter: impl IntoIterator<Item = Value>) -> Option<Vec<(usize, String, String)>> {
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

        // TODO: later, let's color these string with LS_COLORS
        // let h: Vec<String> = headers.into_iter().map(|x| x.trim().to_string()).collect();
        // let d: Vec<Vec<String>> = data.into_iter().map(|x| x.into_iter().collect()).collect();

        let mut h: Vec<String> = headers.into_iter().collect();
        // let d: Vec<Vec<String>> = data.into_iter().collect();

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

// fn convert_to_list(iter: impl IntoIterator<Item = Value>) -> Option<Vec<Vec<String>>> {
//     let mut iter = iter.into_iter().peekable();
//     let mut data = vec![];

//     if let Some(first) = iter.peek() {
//         // dbg!(&first);
//         let mut headers = first.columns();

//         if !headers.is_empty() {
//             headers.insert(0, "#".into());
//         }

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

//         Some(data)
//     } else {
//         None
//     }
// }
