use nu_errors::ShellError;
use nu_protocol::Value;
use nu_source::Tagged;

use tui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{BarChart as TuiBarChart, Block, Borders},
};

pub enum Columns {
    One(Tagged<String>),
    Two(Tagged<String>, Tagged<String>),
    None,
}

#[allow(clippy::type_complexity)]
pub struct Chart {
    pub reduction: nu_data::utils::Reduction,
    pub columns: Columns,
    pub eval: Option<Box<dyn Fn(usize, &Value) -> Result<Value, ShellError> + Send>>,
    pub format: Option<String>,
}

impl Default for Chart {
    fn default() -> Self {
        Self::new()
    }
}

impl Chart {
    pub fn new() -> Chart {
        Chart {
            reduction: nu_data::utils::Reduction::Count,
            columns: Columns::None,
            eval: None,
            format: None,
        }
    }
}

pub struct BarChart<'a> {
    pub title: &'a str,
    pub data: Vec<(&'a str, u64)>,
    pub enhanced_graphics: bool,
}

impl<'a> BarChart<'a> {
    pub fn from_model(model: &'a nu_data::utils::Model) -> Result<BarChart<'a>, ShellError> {
        let mut data = Vec::new();
        let mut data_points = Vec::new();

        for percentages in model
            .percentages
            .table_entries()
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
        {
            let mut percentages_collected = vec![];

            for percentage in percentages
                .table_entries()
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
            {
                percentages_collected.push(percentage.as_u64()?);
            }

            data_points.push(percentages_collected);
        }

        let mark_in = if model.labels.y.len() <= 1 {
            0
        } else {
            (model.labels.y.len() as f64 / 2.0).floor() as usize
        };

        for idx in 0..model.labels.x.len() {
            let mut current = 0;

            loop {
                let label = if current == mark_in {
                    model
                        .labels
                        .at(idx)
                        .ok_or_else(|| ShellError::untagged_runtime_error("Could not load data"))?
                } else {
                    ""
                };

                let percentages_collected = data_points
                    .get(current)
                    .ok_or_else(|| ShellError::untagged_runtime_error("Could not load data"))?;

                data.push((
                    label,
                    *percentages_collected
                        .get(idx)
                        .ok_or_else(|| ShellError::untagged_runtime_error("Could not load data"))?,
                ));

                current += 1;

                if current == model.labels.y.len() {
                    break;
                }
            }
        }

        Ok(BarChart {
            title: "Bar Chart",
            data: (&data[..]).to_vec(),
            enhanced_graphics: true,
        })
    }

    pub fn draw<T>(&mut self, ui: &mut tui::Terminal<T>) -> std::io::Result<()>
    where
        T: tui::backend::Backend,
    {
        ui.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(0)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(f.size());

            let barchart = TuiBarChart::default()
                .block(Block::default().title("Chart").borders(Borders::ALL))
                .data(&self.data)
                .bar_width(9)
                .bar_style(Style::default().fg(Color::Green))
                .value_style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                );

            f.render_widget(barchart, chunks[0]);
        })
    }

    pub fn on_right(&mut self) {
        let one_bar = self.data.remove(0);
        self.data.push(one_bar);
    }

    pub fn on_left(&mut self) {
        if let Some(one_bar) = self.data.pop() {
            self.data.insert(0, one_bar);
        }
    }
}
