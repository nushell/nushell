// use nu_protocol::ast::{Call, PathMember};
// use nu_protocol::engine::{Command, EvaluationContext};
// use nu_protocol::{Signature, Span, Spanned, SyntaxShape, Value};
// use nu_table::StyledString;
// use std::collections::HashMap;
use super::grid::{Alignment, Cell, Direction, Filling, Grid, GridOptions};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, PathMember},
    engine::{Command, EvaluationContext},
    Signature, Span, SyntaxShape, Value,
};
use terminal_size::{Height, Width};

pub struct Griddle;

//NOTE: this is not a real implementation :D. It's just a simple one to test with until we port the real one.
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
        let columns: Option<String> = call.get_flag(context, "columns")?;

        match input {
            Value::List { vals, .. } => {
                // let table = convert_to_table(vals);

                // if let Some(table) = table {
                //     let result = nu_table::draw_table(&table, 80, &HashMap::new());

                //     Ok(Value::String {
                //         val: result,
                //         span: call.head,
                //     })
                // } else {
                //     Ok(Value::Nothing { span: call.head })
                // }
                dbg!("value::list");
                dbg!("{:#?}", vals);
                Ok(Value::Nothing { span: call.head })
            }
            Value::Stream { stream, .. } => {
                // dbg!("value::stream");
                // let table = convert_to_table(stream);

                // if let Some(table) = table {
                //     let result = nu_table::draw_table(&table, 80, &HashMap::new());

                //     Ok(Value::String {
                //         val: result,
                //         span: call.head,
                //     })
                // } else {
                //     Ok(Value::Nothing { span: call.head })
                // }
                let data = convert_to_list(stream);
                if let Some(data) = data {
                    let mut grid = Grid::new(GridOptions {
                        direction: Direction::TopToBottom,
                        filling: Filling::Text(" | ".into()),
                    });

                    for h in data {
                        let a_string = (&h[1]).to_string(); // bytes ->, &h[3]);
                        let mut cell = Cell::from(a_string);
                        cell.alignment = Alignment::Right;
                        grid.add(cell);
                    }

                    let cols = if let Some(col) = columns {
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
                            val: "Couldn't fit grid into 80 columns!".to_string(),
                            span: call.head,
                        })
                    }

                    // Ok(Value::String {
                    //     val: "".to_string(),
                    //     span: call.head,
                    // })
                } else {
                    // dbg!(data);
                    Ok(Value::Nothing { span: call.head })
                }
            }
            Value::Record {
                cols: _, vals: _, ..
            } => {
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
                dbg!("value::record");
                Ok(Value::Nothing { span: call.head })
            }
            x => Ok(x),
        }
    }
}

fn convert_to_list(iter: impl IntoIterator<Item = Value>) -> Option<Vec<Vec<String>>> {
    let mut iter = iter.into_iter().peekable();
    let mut data = vec![];

    if let Some(first) = iter.peek() {
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
