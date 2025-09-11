use devicons::icon_for_file;
use lscolors::Style;
use nu_color_config::lookup_ansi_color_style;
use nu_engine::{command_prelude::*, env_to_string};
use nu_protocol::Config;
use nu_term_grid::grid::{Alignment, Cell, Direction, Filling, Grid, GridOptions};
use nu_utils::{get_ls_colors, terminal_size};
use std::path::Path;

#[derive(Clone)]
pub struct Griddle;

impl Command for Griddle {
    fn name(&self) -> &str {
        "grid"
    }

    fn description(&self) -> &str {
        "Renders the output to a textual terminal grid."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("grid")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Any)), Type::String),
                (Type::record(), Type::String),
            ])
            .named(
                "width",
                SyntaxShape::Int,
                "number of terminal columns wide (not output columns)",
                Some('w'),
            )
            .switch("color", "draw output with color", Some('c'))
            .switch(
                "icons",
                "draw output with icons (assumes nerd font is used)",
                Some('i'),
            )
            .named(
                "separator",
                SyntaxShape::String,
                "character to separate grid with",
                Some('s'),
            )
            .category(Category::Viewers)
    }

    fn extra_description(&self) -> &str {
        r#"grid was built to give a concise gridded layout for ls. however,
it determines what to put in the grid by looking for a column named
'name'. this works great for tables and records but for lists we
need to do something different. such as with '[one two three] | grid'
it creates a fake column called 'name' for these values so that it
prints out the list properly."#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let width_param: Option<i64> = call.get_flag(engine_state, stack, "width")?;
        let color_param: bool = call.has_flag(engine_state, stack, "color")?;
        let separator_param: Option<String> = call.get_flag(engine_state, stack, "separator")?;
        let icons_param: bool = call.has_flag(engine_state, stack, "icons")?;
        let config = &stack.get_config(engine_state);
        let env_str = match stack.get_env_var(engine_state, "LS_COLORS") {
            Some(v) => Some(env_to_string("LS_COLORS", v, engine_state, stack)?),
            None => None,
        };

        let use_color: bool = color_param && config.use_ansi_coloring.get(engine_state);
        let cwd = engine_state.cwd(Some(stack))?;

        match input {
            PipelineData::Value(Value::List { vals, .. }, ..) => {
                // dbg!("value::list");
                let data = convert_to_list(vals, config)?;
                if let Some(items) = data {
                    Ok(create_grid_output(
                        items,
                        call,
                        width_param,
                        use_color,
                        separator_param,
                        env_str,
                        icons_param,
                        cwd.as_ref(),
                    )?)
                } else {
                    Ok(PipelineData::empty())
                }
            }
            PipelineData::ListStream(stream, ..) => {
                // dbg!("value::stream");
                let data = convert_to_list(stream, config)?;
                if let Some(items) = data {
                    Ok(create_grid_output(
                        items,
                        call,
                        width_param,
                        use_color,
                        separator_param,
                        env_str,
                        icons_param,
                        cwd.as_ref(),
                    )?)
                } else {
                    // dbg!(data);
                    Ok(PipelineData::empty())
                }
            }
            PipelineData::Value(Value::Record { val, .. }, ..) => {
                // dbg!("value::record");
                let mut items = vec![];

                for (i, (c, v)) in val.into_owned().into_iter().enumerate() {
                    items.push((i, c, v.to_expanded_string(", ", config)))
                }

                Ok(create_grid_output(
                    items,
                    call,
                    width_param,
                    use_color,
                    separator_param,
                    env_str,
                    icons_param,
                    cwd.as_ref(),
                )?)
            }
            x => {
                // dbg!("other value");
                // dbg!(x.get_type());
                Ok(x)
            }
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Render a simple list to a grid",
                example: "[1 2 3 a b c] | grid",
                result: Some(Value::test_string("1 │ 2 │ 3 │ a │ b │ c\n")),
            },
            Example {
                description: "The above example is the same as:",
                example: "[1 2 3 a b c] | wrap name | grid",
                result: Some(Value::test_string("1 │ 2 │ 3 │ a │ b │ c\n")),
            },
            Example {
                description: "Render a record to a grid",
                example: "{name: 'foo', b: 1, c: 2} | grid",
                result: Some(Value::test_string("foo\n")),
            },
            Example {
                description: "Render a list of records to a grid",
                example: "[{name: 'A', v: 1} {name: 'B', v: 2} {name: 'C', v: 3}] | grid",
                result: Some(Value::test_string("A │ B │ C\n")),
            },
            Example {
                description: "Render a table with 'name' column in it to a grid",
                example: "[[name patch]; [0.1.0 false] [0.1.1 true] [0.2.0 false]] | grid",
                result: Some(Value::test_string("0.1.0 │ 0.1.1 │ 0.2.0\n")),
            },
            Example {
                description: "Render a table with 'name' column in it to a grid with icons and colors",
                example: "[[name patch]; [Cargo.toml false] [README.md true] [SECURITY.md false]] | grid --icons --color",
                result: None,
            },
        ]
    }
}

#[allow(clippy::too_many_arguments)]
fn create_grid_output(
    items: Vec<(usize, String, String)>,
    call: &Call,
    width_param: Option<i64>,
    use_color: bool,
    separator_param: Option<String>,
    env_str: Option<String>,
    icons_param: bool,
    cwd: &Path,
) -> Result<PipelineData, ShellError> {
    let ls_colors = get_ls_colors(env_str);

    let cols = if let Some(col) = width_param {
        col as u16
    } else if let Ok((w, _h)) = terminal_size() {
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
            if use_color {
                if icons_param {
                    let no_ansi = nu_utils::strip_ansi_unlikely(&value);
                    let path = cwd.join(no_ansi.as_ref());
                    let file_icon = icon_for_file(&path, &None);
                    let ls_colors_style = ls_colors.style_for_path(path);
                    let icon_style = lookup_ansi_color_style(file_icon.color);

                    let ansi_style = ls_colors_style
                        .map(Style::to_nu_ansi_term_style)
                        .unwrap_or_default();

                    let item = format!(
                        "{} {}",
                        icon_style.paint(String::from(file_icon.icon)),
                        ansi_style.paint(value)
                    );

                    let mut cell = Cell::from(item);
                    cell.alignment = Alignment::Left;
                    grid.add(cell);
                } else {
                    let no_ansi = nu_utils::strip_ansi_unlikely(&value);
                    let path = cwd.join(no_ansi.as_ref());
                    let style = ls_colors.style_for_path(path.clone());
                    let ansi_style = style.map(Style::to_nu_ansi_term_style).unwrap_or_default();
                    let mut cell = Cell::from(ansi_style.paint(value).to_string());
                    cell.alignment = Alignment::Left;
                    grid.add(cell);
                }
            } else if icons_param {
                let no_ansi = nu_utils::strip_ansi_unlikely(&value);
                let path = cwd.join(no_ansi.as_ref());
                let file_icon = icon_for_file(&path, &None);
                let item = format!("{} {}", String::from(file_icon.icon), value);
                let mut cell = Cell::from(item);
                cell.alignment = Alignment::Left;
                grid.add(cell);
            } else {
                let mut cell = Cell::from(value);
                cell.alignment = Alignment::Left;
                grid.add(cell);
            }
        }
    }

    if let Some(grid_display) = grid.fit_into_width(cols as usize) {
        Ok(Value::string(grid_display.to_string(), call.head).into_pipeline_data())
    } else {
        Err(ShellError::GenericError {
            error: format!("Couldn't fit grid into {cols} columns"),
            msg: "too few columns to fit the grid into".into(),
            span: Some(call.head),
            help: Some("try rerunning with a different --width".into()),
            inner: Vec::new(),
        })
    }
}

#[allow(clippy::type_complexity)]
fn convert_to_list(
    iter: impl IntoIterator<Item = Value>,
    config: &Config,
) -> Result<Option<Vec<(usize, String, String)>>, ShellError> {
    let mut iter = iter.into_iter().peekable();

    if let Some(first) = iter.peek() {
        let mut headers: Vec<String> = first.columns().cloned().collect();

        if !headers.is_empty() {
            headers.insert(0, "#".into());
        }

        let mut data = vec![];

        for (row_num, item) in iter.enumerate() {
            if let Value::Error { error, .. } = item {
                return Err(*error);
            }

            let mut row = vec![row_num.to_string()];

            if headers.is_empty() {
                row.push(item.to_expanded_string(", ", config))
            } else {
                for header in headers.iter().skip(1) {
                    let result = match &item {
                        Value::Record { val, .. } => val.get(header),
                        item => Some(item),
                    };

                    match result {
                        Some(value) => {
                            if let Value::Error { error, .. } = item {
                                return Err(*error);
                            }
                            row.push(value.to_expanded_string(", ", config));
                        }
                        None => row.push(String::new()),
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

        Ok(Some(interleaved))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::Griddle;
        use crate::test_examples;
        test_examples(Griddle {})
    }
}
