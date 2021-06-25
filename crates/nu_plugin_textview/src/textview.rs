use crate::config::Config;
use nu_protocol::{Primitive, UntaggedValue, Value};
use nu_source::{AnchorLocation, Tag};
use std::path::Path;

#[derive(Default)]
pub struct TextView;

impl TextView {
    pub fn new() -> TextView {
        TextView
    }
}

fn get_file_path(source: AnchorLocation) -> Option<String> {
    match source {
        AnchorLocation::File(file) => {
            let path = Path::new(&file);

            Some(path.to_string_lossy().to_string())
        }
        AnchorLocation::Url(url) => url::Url::parse(&url).ok().and_then(|url| {
            url.path_segments().and_then(|mut segments| {
                segments
                    .next_back()
                    .map(|segment| Path::new(segment).to_string_lossy().to_string())
            })
        }),
        //FIXME: this probably isn't correct
        AnchorLocation::Source(_source) => None,
    }
}

#[allow(clippy::cognitive_complexity)]
pub fn view_text_value(value: &Value) {
    let config = nu_data::config::config(Tag::unknown())
        .ok()
        .and_then(|config| config.get("textview").map(Config::from))
        .unwrap_or_else(Config::default);

    if let UntaggedValue::Primitive(Primitive::String(ref s)) = &value.value {
        let mut printer = bat::PrettyPrinter::new();

        printer
            .term_width(config.term_width)
            .tab_width(Some(config.tab_width as usize))
            .colored_output(config.colored_output)
            .true_color(config.true_color)
            .header(config.header)
            .line_numbers(config.line_numbers)
            .grid(config.grid)
            .vcs_modification_markers(config.vcs_modification_markers)
            .snip(config.snip)
            .wrapping_mode(config.wrapping_mode)
            .use_italics(config.use_italics)
            .paging_mode(config.paging_mode)
            .pager(&config.pager)
            .line_ranges(config.line_ranges)
            .highlight_range(
                config.highlight_range_from as usize,
                config.highlight_range_to as usize,
            )
            .theme(&config.theme);

        match value.anchor().and_then(get_file_path) {
            Some(file_path) => printer.input(bat::Input::from_bytes(s.as_bytes()).name(file_path)),
            None => printer.input_from_bytes(s.as_bytes()),
        };

        printer.print().expect("Error with bat PrettyPrint");
    }
}
