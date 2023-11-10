use charming::{
    component::Legend,
    element::ItemStyle,
    series::{Pie, PieRoseType},
    Chart, HtmlRenderer,
};
use std::iter::zip;

pub fn create_plot(labels: Vec<String>, values: Vec<i32>) {
    let data = zip(values, labels).collect();
    let chart = Chart::new().legend(Legend::new().top("bottom")).series(
        Pie::new()
            .name("Nightingale Chart")
            .rose_type(PieRoseType::Radius)
            .radius(vec!["50", "250"])
            .center(vec!["50%", "50%"])
            .item_style(ItemStyle::new().border_radius(8))
            .data(data),
    );

    let renderer = HtmlRenderer::new("chart", 1000, 800);
    let _html_str = renderer.render(&chart).unwrap();
}
