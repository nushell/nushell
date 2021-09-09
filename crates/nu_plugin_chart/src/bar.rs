use nu_data::utils::Model;
use nu_errors::ShellError;

use tui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::BarChart,
};

const DEFAULT_COLOR: Color = Color::Green;

pub struct Bar<'a> {
    pub title: &'a str,
    pub data: Vec<(&'a str, u64)>,
    pub enhanced_graphics: bool,
}

impl<'a> Bar<'a> {
    pub fn from_model(model: &'a Model) -> Result<Bar<'a>, ShellError> {
        let mut data = Vec::new();
        let mut data_points = Vec::new();

        for percentages in model
            .percentages
            .table_entries()
            .cloned()
            .collect::<Vec<_>>()
        {
            let mut percentages_collected = vec![];

            for percentage in percentages.table_entries().cloned().collect::<Vec<_>>() {
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

        Ok(Bar {
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

            let barchart = BarChart::default()
                .data(&self.data)
                .bar_width(9)
                .bar_style(Style::default().fg(DEFAULT_COLOR))
                .value_style(
                    Style::default()
                        .bg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                );

            f.render_widget(barchart, chunks[0]);
        })?;
        Ok(())
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
