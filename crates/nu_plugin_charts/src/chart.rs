use nu_errors::ShellError;
use nu_protocol::Value;
use nu_source::Tagged;

use tui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{
        BarChart as TuiBarChart, Block, Borders,
    },
};

pub enum Reduction {
    Accumulate,
    Count,
}

pub enum Columns {
    One(Tagged<String>),
    Two(Tagged<String>, Tagged<String>),
    None,
}

pub struct Chart {
    pub reduction: Reduction,
    pub columns: Columns,
    pub eval: Option<Box<dyn Fn(usize, &Value) -> Result<Value, ShellError> + Send>>,
    pub format: Option<String>,
}

impl Chart {
    pub fn new() -> Chart {
        Chart {
            reduction: Reduction::Count,
            columns: Columns::None,
            eval: None,
            format: None,
        }
    }
}

pub struct BarChart<'a> {
    pub title: &'a str,
    pub should_quit: bool,
    pub show_chart: bool,
    pub progress: f64,
    pub data: Vec<(&'a str, u64)>,
    pub enhanced_graphics: bool,
}

impl<'a> BarChart<'a> {
    pub fn from_model(model: &'a nu_data::utils::Model) -> Result<BarChart<'a>, ShellError> {
        let mut data = Vec::new();

        for (_idx, split) in model.percentages.table_entries().enumerate() {
            for (idxx, group) in split.table_entries().enumerate() {
                data.push((
                    model
                        .labels
                        .at(idxx as usize)
                        .ok_or_else(|| ShellError::untagged_runtime_error("Could not load data"))?,
                    group.as_u64()?,
                ));
            }
        }

        println!("{:#?}", data.to_vec());

        Ok(BarChart {
            title: "chart",
            should_quit: false,
            show_chart: true,
            progress: 0.0,
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
                .margin(2)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(f.size());

            let barchart = TuiBarChart::default()
                .block(Block::default().title("Data1").borders(Borders::ALL))
                .data(&self.data)
                .bar_width(9)
                .bar_style(Style::default().fg(Color::Green))
                .value_style(
                    Style::default()
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                );

            f.render_widget(barchart, chunks[0]);
        })
    }

    #[allow(unused)]
    pub fn on_key(&mut self, c: char) {
        match c {
            'q' => {
                self.should_quit = true;
            }
            't' => {
                self.show_chart = !self.show_chart;
            }
            _ => {}
        }
    }

    pub fn on_right(&mut self) {
        let event = self.data.pop().unwrap();
        self.data.insert(0, event);
    }

    pub fn on_left(&mut self) {
        let event = self.data.remove(0);
        self.data.push(event);
    }

    pub fn on_tick(&mut self) {
        // Update progress
        self.progress += 0.001;
        if self.progress > 1.0 {
            self.progress = 0.0;
        }
        /*
        let event = self.barchart.pop().unwrap();
        self.barchart.insert(0, event);
        */
    }
}
