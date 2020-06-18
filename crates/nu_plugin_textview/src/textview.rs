use nu_protocol::{Primitive, UntaggedValue, Value};
use nu_source::{AnchorLocation, Tag};
use std::path::Path;
// use nu_cli::utils::test_bins as binaries;
// use nu_cli::{create_default_context, EnvironmentSyncer};
//use nu_cli::{self, data, config::config};
use nu_cli::data::config as nuconfig;

#[derive(Default)]
pub struct TextView;

impl TextView {
    pub fn new() -> TextView {
        TextView
    }
}

pub fn view_text_value(value: &Value) {
    //let cfg = nu_cli::data::config::config(Tag::unknown());
    // let cfg = nuconfig::config(Tag::unknown());
    // before we start up, let's run our startup commands
    if let Ok(config) = nuconfig::config(Tag::unknown()) {
        if let Some(commands) = config.get("[bat]") {
            match commands {
                Value {
                    value: UntaggedValue::Table(pipelines),
                    ..
                } => {
                    for pipeline in pipelines {
                        if let Ok(pipeline_string) = pipeline.as_string() {
                            // let _ = run_pipeline_standalone(
                            //     pipeline_string,
                            //     false,
                            //     &mut context,
                            //     false,
                            // )
                            // .await;
                        }
                    }
                }
                _ => {
                    println!("warning: expected a table of pipeline strings as startup commands");
                }
            }
        }
    }




    let value_anchor = value.anchor();
    if let UntaggedValue::Primitive(Primitive::String(ref s)) = &value.value {
        if let Some(source) = value_anchor {
            let file_path: Option<String> = match source {
                AnchorLocation::File(file) => {
                    let path = Path::new(&file);
                    Some(path.to_string_lossy().to_string())
                }
                AnchorLocation::Url(url) => {
                    let url = url::Url::parse(&url);
                    if let Ok(url) = url {
                        if let Some(mut segments) = url.path_segments() {
                            if let Some(file) = segments.next_back() {
                                let path = Path::new(file);
                                Some(path.to_string_lossy().to_string())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                //FIXME: this probably isn't correct
                AnchorLocation::Source(_source) => None,
            };

            match file_path {
                Some(file_path) => {
                    // Let bat do it's thing
                    bat::PrettyPrinter::new()
                        .input_from_bytes_with_name(s.as_bytes(), file_path)
                        .term_width(textwrap::termwidth())
                        .tab_width(Some(4))
                        .colored_output(true)
                        .true_color(true)
                        .header(true)
                        .line_numbers(true)
                        .grid(true)
                        .vcs_modification_markers(true)
                        .snip(true)
                        .wrapping_mode(bat::WrappingMode::NoWrapping)
                        .use_italics(true)
                        .paging_mode(bat::PagingMode::QuitIfOneScreen)
                        .pager("less")
                        .line_ranges(bat::line_range::LineRanges::all())
                        .highlight_range(0, 0)
                        .theme("OneHalfDark")
                        .print()
                        .expect("Error with bat PrettyPrint");
                }
                _ => {
                    bat::PrettyPrinter::new()
                        .input_from_bytes(s.as_bytes())
                        .term_width(textwrap::termwidth())
                        .tab_width(Some(4))
                        .colored_output(true)
                        .true_color(true)
                        .header(true)
                        .line_numbers(true)
                        .grid(true)
                        .vcs_modification_markers(true)
                        .snip(true)
                        .wrapping_mode(bat::WrappingMode::NoWrapping)
                        .use_italics(true)
                        .paging_mode(bat::PagingMode::QuitIfOneScreen)
                        .pager("less")
                        .line_ranges(bat::line_range::LineRanges::all())
                        .highlight_range(0, 0)
                        .theme("OneHalfDark")
                        .print()
                        .expect("Error with bat PrettyPrint");
                }
            }
        } else {
            bat::PrettyPrinter::new()
                .input_from_bytes(s.as_bytes())
                .term_width(textwrap::termwidth())
                .tab_width(Some(4))
                .colored_output(true)
                .true_color(true)
                .header(true)
                .line_numbers(true)
                .grid(true)
                .vcs_modification_markers(true)
                .snip(true)
                .wrapping_mode(bat::WrappingMode::NoWrapping)
                .use_italics(true)
                .paging_mode(bat::PagingMode::QuitIfOneScreen)
                .pager("less")
                .line_ranges(bat::line_range::LineRanges::all())
                .highlight_range(0, 0)
                .theme("OneHalfDark")
                .print()
                .expect("Error with bat PrettyPrint");
        }
    }
}
