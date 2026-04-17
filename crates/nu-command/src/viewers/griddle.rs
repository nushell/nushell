use devicons::icon_for_file;
use lscolors::Style;
use nu_color_config::lookup_ansi_color_style;
use nu_engine::{command_prelude::*, env_to_string};
use nu_protocol::Config;
use nu_protocol::shell_error::generic::GenericError;
use nu_term_grid::grid::{Alignment, Cell, Direction, Filling, Grid, GridOptions};
use nu_utils::{get_ls_colors, terminal_size};
use std::path::Path;

const NAME_COLUMN: &str = "name";

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
                "Number of terminal columns wide (not output columns).",
                Some('w'),
            )
            .switch("color", "Draw output with color.", Some('c'))
            .switch(
                "icons",
                "Draw output with icons (assumes nerd font is used).",
                Some('i'),
            )
            .named(
                "separator",
                SyntaxShape::String,
                "Character to separate grid with.",
                Some('s'),
            )
            .category(Category::Viewers)
    }

    fn extra_description(&self) -> &str {
        "The `grid` command was built to give a concise gridded layout for ls. It
prints every item of the list in a grid layout. However, for table
or record, it determines what to put in the grid by looking for a
column named 'name'. This is subject to change in the future."
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
                let items = val
                    .get(NAME_COLUMN)
                    .map(|v| v.to_expanded_string(", ", config))
                    .into_iter()
                    .collect();

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
    items: Vec<String>,
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

    for value in items {
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

    if let Some(grid_display) = grid.fit_into_width(cols as usize) {
        Ok(Value::string(grid_display.to_string(), call.head).into_pipeline_data())
    } else {
        Err(ShellError::Generic(
            GenericError::new(
                format!("Couldn't fit grid into {cols} columns"),
                "too few columns to fit the grid into",
                call.head,
            )
            .with_help("try rerunning with a different --width"),
        ))
    }
}

#[allow(clippy::type_complexity)]
fn convert_to_list(
    iter: impl IntoIterator<Item = Value>,
    config: &Config,
) -> Result<Option<Vec<String>>, ShellError> {
    let mut iter = iter.into_iter().peekable();

    let Some(first) = iter.peek() else {
        return Ok(None);
    };

    let headers = first.columns().collect::<Vec<_>>();
    let has_name_header = headers.iter().any(|&str| str == NAME_COLUMN);

    if !headers.is_empty() && !has_name_header {
        return Ok(Some(vec![]));
    }

    iter.map(|item| {
        if let Value::Error { error, .. } = item {
            return Err(*error);
        }

        let string = if !has_name_header {
            item.to_expanded_string(", ", config)
        } else {
            let result = match &item {
                Value::Record { val, .. } => val.get(NAME_COLUMN),
                item => Some(item),
            };

            match result {
                Some(value) => {
                    if let Value::Error { error, .. } = item {
                        return Err(*error);
                    }
                    value.to_expanded_string(", ", config)
                }
                None => String::new(),
            }
        };

        Ok(Some(string))
    })
    .collect()
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() -> nu_test_support::Result {
        use super::Griddle;
        nu_test_support::test().examples(Griddle)
    }
}
