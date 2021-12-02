use nu_data::utils::Model;
use nu_errors::ShellError;

use tui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    symbols,
    text::Span,
    widgets::{Axis, Chart, Dataset, GraphType},
};

const DEFAULT_COLOR: Color = Color::Green;

const DEFAULT_LINE_COLORS: [Color; 5] = [
    Color::Green,
    Color::Cyan,
    Color::Magenta,
    Color::Yellow,
    Color::Red,
];

#[derive(Debug)]
pub struct Line {
    x_labels: Vec<String>,
    x_range: [f64; 2],
    y_range: [f64; 2],
    datasets_names: Vec<String>,
    data: Vec<Vec<(f64, f64)>>,
}

impl<'a> Line {
    pub fn from_model(model: &'a Model) -> Result<Line, ShellError> {
        Ok(Line {
            x_labels: model.labels.x.to_vec(),
            x_range: [
                model.ranges.0.start.as_u64()? as f64,
                model.labels.x.len() as f64,
            ],
            y_range: [
                model.ranges.1.start.as_u64()? as f64,
                model.ranges.1.end.as_u64()? as f64,
            ],
            datasets_names: if model.labels.y.len() == 1 {
                vec!["".to_string()]
            } else {
                model.labels.y.to_vec()
            },
            data: model
                .data
                .table_entries()
                .collect::<Vec<_>>()
                .iter()
                .map(|subset| {
                    subset
                        .table_entries()
                        .enumerate()
                        .map(|(idx, data_point)| {
                            (
                                idx as f64,
                                if let Ok(point) = data_point.as_u64() {
                                    point as f64
                                } else {
                                    0.0
                                },
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>(),
        })
    }

    pub fn draw<T>(&mut self, ui: &mut tui::Terminal<T>) -> std::io::Result<()>
    where
        T: tui::backend::Backend,
    {
        ui.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(f.size());

            let x_labels = self
                .x_labels
                .iter()
                .map(move |label| {
                    Span::styled(label, Style::default().add_modifier(Modifier::BOLD))
                })
                .collect::<Vec<_>>();

            let y_labels = vec![
                Span::styled(
                    self.y_range[0].to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(((self.y_range[0] + self.y_range[1]) / 2.0).to_string()),
                Span::styled(
                    self.y_range[1].to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ];

            let marker = if x_labels.len() > 60 {
                symbols::Marker::Braille
            } else {
                symbols::Marker::Dot
            };

            let datasets = self
                .data
                .iter()
                .enumerate()
                .map(|(idx, data_series)| {
                    Dataset::default()
                        .name(&self.datasets_names[idx])
                        .marker(marker)
                        .graph_type(GraphType::Line)
                        .style(
                            Style::default()
                                .fg(*DEFAULT_LINE_COLORS.get(idx).unwrap_or(&DEFAULT_COLOR)),
                        )
                        .data(data_series)
                })
                .collect();

            let chart = Chart::new(datasets)
                .x_axis(
                    Axis::default()
                        .style(Style::default().fg(Color::Gray))
                        .labels(x_labels)
                        .bounds(self.x_range),
                )
                .y_axis(
                    Axis::default()
                        .style(Style::default().fg(Color::Gray))
                        .labels(y_labels)
                        .bounds(self.y_range),
                );
            f.render_widget(chart, chunks[0]);
        })?;
        Ok(())
    }
}
