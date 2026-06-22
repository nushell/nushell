use devicons::icon_for_file;
use lscolors::Style;
use nu_color_config::lookup_ansi_color_style;
use nu_engine::{command_prelude::*, env_to_string};
use nu_protocol::Config;
use nu_protocol::shell_error::generic::GenericError;
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
            .input_output_type(Type::List(Box::new(Type::Any)), Type::String)
            .optional(
                "column",
                SyntaxShape::CellPath,
                "Format this column in a grid.",
            )
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
        "The `grid` command creates a concise gridded layout for the input. It
prints every item of the list in a grid layout. For tables or list
containing records, it will look for a 'name' column by default; if
the 'name' column is missing, the entire record is rendered instead."
    }
    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_path: Option<CellPath> = call.opt(engine_state, stack, 0)?;
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
                let items = convert_to_list(vals, cell_path, config)?;
                create_grid_output(
                    items,
                    call,
                    width_param,
                    use_color,
                    separator_param,
                    env_str,
                    icons_param,
                    cwd.as_ref(),
                )
            }
            PipelineData::ListStream(stream, ..) => {
                // dbg!("value::stream");
                let items = convert_to_list(stream, cell_path, config)?;
                create_grid_output(
                    items,
                    call,
                    width_param,
                    use_color,
                    separator_param,
                    env_str,
                    icons_param,
                    cwd.as_ref(),
                )
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
                example: "[1 2 3 a b c] | wrap name | grid name",
                result: Some(Value::test_string("1 │ 2 │ 3 │ a │ b │ c\n")),
            },
            Example {
                description: "Render a list of records to a grid",
                example: "[{name: 'A', v: 1} {name: 'B', v: 2} {name: 'C', v: 3}] | grid name",
                result: Some(Value::test_string("A │ B │ C\n")),
            },
            Example {
                description: "Render a table with 'name' column in it to a grid",
                example: "[[name patch]; [0.1.0 false] [0.1.1 true] [0.2.0 false]] | grid name",
                result: Some(Value::test_string("0.1.0 │ 0.1.1 │ 0.2.0\n")),
            },
            Example {
                description: "Render a table with 'name' column in it to a grid with icons and colors",
                example: "ls | grid --icons --color name",
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

/// Converts an iterator of values into a list of expanded strings, suitable for grid layouts.
///
/// This function supports two evaluation paths depending on the presence of a cell path:
///
/// - **Explicit Path:** If a `cell_path` is specified (e.g., `ls | grid name`), it extracts the
///   value at that inner path for every item.
/// - **Implicit Fallback:** If no path is provided (e.g., `ls | grid`), it checks if the item is
///   a `Value::Record`. If the record contains a `"name"` column, it extracts that value;
///   otherwise, it falls back to processing the item as-is.
///
/// # Errors
///
/// Returns a `ShellError` if any item evaluates to a `Value::Error`, or if a provided
/// `cell_path` fails to resolve against the data structure.
fn convert_to_list(
    iter: impl IntoIterator<Item = Value>,
    cell_path: Option<CellPath>,
    config: &Config,
) -> Result<Vec<String>, ShellError> {
    let iter = iter.into_iter();

    if let Some(cell_path) = cell_path {
        // Path A: Explicit cell path provided (e.g., `ls | grid name`)
        iter.map(|item| {
            if let Value::Error { error, .. } = item {
                return Err(*error);
            }

            let string = item
                .follow_cell_path(&cell_path.members)?
                .to_expanded_string(", ", config);

            Ok(string)
        })
        .collect()
    } else {
        // Path B: Implicit fallback (e.g., `ls | grid`). Matches the "name" column if present.

        iter.map(|item| {
            let target_value = match &item {
                Value::Record { val, .. } => val.get("name").unwrap_or(&item),
                item => item,
            };

            match target_value {
                Value::Error { error, .. } => Err(*error.clone()),
                val => Ok(val.to_expanded_string(", ", config)),
            }
        })
        .collect()
    }
}
#[cfg(test)]
mod test {
    #[test]
    fn test_examples() -> nu_test_support::Result {
        use super::Griddle;
        nu_test_support::test().examples(Griddle)
    }
}
