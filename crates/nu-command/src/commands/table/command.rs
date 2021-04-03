use crate::commands::table::options::{ConfigExtensions, NuConfig as TableConfiguration};
use crate::prelude::*;
use crate::primitive::get_color_config;
use nu_data::value::{format_leaf, style_leaf};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, SyntaxShape, UntaggedValue, Value};
use nu_table::{draw_table, Alignment, StyledString, TextStyle};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::Instant;

#[cfg(feature = "table-pager")]
use {
    futures::future::join,
    minus::{ExitStrategy, Pager},
    std::fmt::Write,
};

const STREAM_PAGE_SIZE: usize = 1000;
const STREAM_TIMEOUT_CHECK_INTERVAL: usize = 100;

pub struct Command;

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "table"
    }

    fn signature(&self) -> Signature {
        Signature::build("table").named(
            "start_number",
            SyntaxShape::Number,
            "row number to start viewing from",
            Some('n'),
        )
    }

    fn usage(&self) -> &str {
        "View the contents of the pipeline as a table."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        table(TableConfiguration::new(), args).await
    }
}

pub fn from_list(
    values: &[Value],
    configuration: &TableConfiguration,
    starting_idx: usize,
    color_hm: &HashMap<String, nu_ansi_term::Style>,
) -> nu_table::Table {
    let header_style = configuration.header_style();
    let mut headers: Vec<StyledString> = nu_protocol::merge_descriptors(values)
        .into_iter()
        .map(|x| StyledString::new(x, header_style))
        .collect();
    let entries = values_to_entries(values, &mut headers, configuration, starting_idx, &color_hm);
    nu_table::Table {
        headers,
        data: entries,
        theme: configuration.table_mode(),
    }
}

fn values_to_entries(
    values: &[Value],
    headers: &mut Vec<StyledString>,
    configuration: &TableConfiguration,
    starting_idx: usize,
    color_hm: &HashMap<String, nu_ansi_term::Style>,
) -> Vec<Vec<StyledString>> {
    let disable_indexes = configuration.disabled_indexes();
    let mut entries = vec![];

    if headers.is_empty() {
        headers.push(StyledString::new("".to_string(), TextStyle::basic_left()));
    }

    for (idx, value) in values.iter().enumerate() {
        let mut row: Vec<StyledString> = headers
            .iter()
            .map(|d: &StyledString| {
                if d.contents.is_empty() {
                    match value {
                        Value {
                            value: UntaggedValue::Row(..),
                            ..
                        } => StyledString::new(
                            format_leaf(&UntaggedValue::nothing()).plain_string(100_000),
                            style_leaf(&UntaggedValue::nothing(), &color_hm),
                        ),
                        _ => StyledString::new(
                            format_leaf(value).plain_string(100_000),
                            style_leaf(value, &color_hm),
                        ),
                    }
                } else {
                    match value {
                        Value {
                            value: UntaggedValue::Row(..),
                            ..
                        } => {
                            let data = value.get_data(&d.contents);

                            StyledString::new(
                                format_leaf(data.borrow()).plain_string(100_000),
                                style_leaf(data.borrow(), &color_hm),
                            )
                        }
                        _ => StyledString::new(
                            format_leaf(&UntaggedValue::nothing()).plain_string(100_000),
                            style_leaf(&UntaggedValue::nothing(), &color_hm),
                        ),
                    }
                }
            })
            .collect();

        // Indices are green, bold, right-aligned:
        // unless we change them :)
        if !disable_indexes {
            row.insert(
                0,
                StyledString::new(
                    (starting_idx + idx).to_string(),
                    TextStyle::new().alignment(Alignment::Right).style(
                        color_hm
                            .get("index_color")
                            .unwrap_or(
                                &nu_ansi_term::Style::default()
                                    .bold()
                                    .fg(nu_ansi_term::Color::Green),
                            )
                            .to_owned(),
                    ),
                ),
            );
        }

        entries.push(row);
    }

    if !disable_indexes {
        headers.insert(
            0,
            StyledString::new(
                "#".to_owned(),
                TextStyle::new()
                    .alignment(Alignment::Center)
                    .fg(nu_ansi_term::Color::Green)
                    .bold(Some(true)),
            ),
        );
    }

    entries
}

async fn table(
    configuration: TableConfiguration,
    args: CommandArgs,
) -> Result<OutputStream, ShellError> {
    let mut args = args.evaluate_once().await?;

    // Ideally, get_color_config would get all the colors configured in the config.toml
    // and create a style based on those settings. However, there are few places where
    // this just won't work right now, like header styling, because a style needs to know
    // more than just color, it needs fg & bg color, bold, dimmed, italic, underline,
    // blink, reverse, hidden, strikethrough and most of those aren't available in the
    // config.toml.... yet.
    let color_hm = get_color_config();

    let mut start_number = match args.get("start_number") {
        Some(Value {
            value: UntaggedValue::Primitive(Primitive::Int(i)),
            ..
        }) => {
            if let Some(num) = i.to_usize() {
                num
            } else {
                return Err(ShellError::labeled_error(
                    "Expected a row number",
                    "expected a row number",
                    &args.args.call_info.name_tag,
                ));
            }
        }
        _ => 0,
    };

    let mut delay_slot = None;

    let term_width = args.host.lock().width();

    #[cfg(feature = "table-pager")]
    let pager = Pager::new()
        .set_exit_strategy(ExitStrategy::PagerQuit)
        .set_searchable(true)
        .set_page_if_havent_overflowed(false)
        .set_input_handler(Box::new(input_handling::MinusInputHandler {}))
        .finish();

    let stream_data = async {
        let finished = Arc::new(AtomicBool::new(false));
        // we are required to clone finished, for use within the callback, otherwise we get borrow errors
        #[cfg(feature = "table-pager")]
        let finished_within_callback = finished.clone();
        #[cfg(feature = "table-pager")]
        {
            // This is called when the pager finishes, to indicate to the
            // while loop below to finish, in case of long running InputStream consumer
            // that doesn't finish by the time the user quits out of the pager
            pager.lock().await.add_exit_callback(move || {
                finished_within_callback.store(true, Ordering::Relaxed);
            });
        }
        while !finished.clone().load(Ordering::Relaxed) {
            let mut new_input: VecDeque<Value> = VecDeque::new();

            let start_time = Instant::now();
            for idx in 0..STREAM_PAGE_SIZE {
                if let Some(val) = delay_slot {
                    new_input.push_back(val);
                    delay_slot = None;
                } else {
                    match args.input.next().await {
                        Some(a) => {
                            if !new_input.is_empty() {
                                if let Some(descs) = new_input.get(0) {
                                    let descs = descs.data_descriptors();
                                    let compare = a.data_descriptors();
                                    if descs != compare {
                                        delay_slot = Some(a);
                                        break;
                                    } else {
                                        new_input.push_back(a);
                                    }
                                } else {
                                    new_input.push_back(a);
                                }
                            } else {
                                new_input.push_back(a);
                            }
                        }
                        _ => {
                            finished.store(true, Ordering::Relaxed);
                            break;
                        }
                    }

                    // Check if we've gone over our buffering threshold
                    if (idx + 1) % STREAM_TIMEOUT_CHECK_INTERVAL == 0 {
                        let end_time = Instant::now();

                        // If we've been buffering over a second, go ahead and send out what we have so far
                        if (end_time - start_time).as_secs() >= 1 {
                            break;
                        }

                        if finished.load(Ordering::Relaxed) {
                            break;
                        }
                    }
                }
            }

            let input: Vec<Value> = new_input.into();

            if !input.is_empty() {
                let t = from_list(&input, &configuration, start_number, &color_hm);
                let output = draw_table(&t, term_width, &color_hm);
                #[cfg(feature = "table-pager")]
                {
                    let mut pager = pager.lock().await;
                    writeln!(pager.lines, "{}", output).map_err(|_| {
                        ShellError::untagged_runtime_error("Error writing to pager")
                    })?;
                }

                #[cfg(not(feature = "table-pager"))]
                println!("{}", output);
            }

            start_number += input.len();
        }

        #[cfg(feature = "table-pager")]
        {
            let mut pager_lock = pager.lock().await;
            pager_lock.data_finished();
        }

        Result::<_, ShellError>::Ok(())
    };

    #[cfg(feature = "table-pager")]
    {
        let (minus_result, streaming_result) =
            join(minus::async_std_updating(pager.clone()), stream_data).await;
        minus_result.map_err(|_| ShellError::untagged_runtime_error("Error paging data"))?;
        streaming_result?;
    }

    #[cfg(not(feature = "table-pager"))]
    stream_data
        .await
        .map_err(|_| ShellError::untagged_runtime_error("Error streaming data"))?;

    Ok(OutputStream::empty())
}

#[cfg(feature = "table-pager")]
mod input_handling {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
    use minus::{InputEvent, InputHandler, LineNumbers, SearchMode};
    pub struct MinusInputHandler;

    impl InputHandler for MinusInputHandler {
        fn handle_input(
            &self,
            ev: Event,
            upper_mark: usize,
            search_mode: SearchMode,
            ln: LineNumbers,
            rows: usize,
        ) -> Option<InputEvent> {
            match ev {
                // Scroll up by one.
                Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    modifiers: KeyModifiers::NONE,
                }) => Some(InputEvent::UpdateUpperMark(upper_mark.saturating_sub(1))),

                // Scroll down by one.
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    modifiers: KeyModifiers::NONE,
                }) => Some(InputEvent::UpdateUpperMark(upper_mark.saturating_add(1))),

                // Mouse scroll up/down
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::ScrollUp,
                    ..
                }) => Some(InputEvent::UpdateUpperMark(upper_mark.saturating_sub(5))),
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::ScrollDown,
                    ..
                }) => Some(InputEvent::UpdateUpperMark(upper_mark.saturating_add(5))),
                // Go to top.
                Event::Key(KeyEvent {
                    code: KeyCode::Home,
                    modifiers: KeyModifiers::NONE,
                }) => Some(InputEvent::UpdateUpperMark(0)),
                // Go to bottom.
                Event::Key(KeyEvent {
                    code: KeyCode::End,
                    modifiers: KeyModifiers::NONE,
                }) => Some(InputEvent::UpdateUpperMark(usize::MAX)),

                // Page Up/Down
                Event::Key(KeyEvent {
                    code: KeyCode::PageUp,
                    modifiers: KeyModifiers::NONE,
                }) => Some(InputEvent::UpdateUpperMark(
                    upper_mark.saturating_sub(rows - 1),
                )),
                Event::Key(KeyEvent {
                    code: KeyCode::PageDown,
                    modifiers: KeyModifiers::NONE,
                }) => Some(InputEvent::UpdateUpperMark(
                    upper_mark.saturating_add(rows - 1),
                )),

                // Resize event from the terminal.
                Event::Resize(_, height) => Some(InputEvent::UpdateRows(height as usize)),
                // Switch line number display.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('l'),
                    modifiers: KeyModifiers::CONTROL,
                }) => Some(InputEvent::UpdateLineNumber(!ln)),
                // Quit.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    modifiers: KeyModifiers::NONE,
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Char('Q'),
                    modifiers: KeyModifiers::SHIFT,
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Esc,
                    modifiers: KeyModifiers::NONE,
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                }) => Some(InputEvent::Exit),
                Event::Key(KeyEvent {
                    code: KeyCode::Char('/'),
                    modifiers: KeyModifiers::NONE,
                }) => Some(InputEvent::Search(SearchMode::Unknown)),
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    modifiers: KeyModifiers::CONTROL,
                }) => {
                    if search_mode == SearchMode::Unknown {
                        Some(InputEvent::NextMatch)
                    } else {
                        None
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    modifiers: KeyModifiers::CONTROL,
                }) => {
                    if search_mode == SearchMode::Unknown {
                        Some(InputEvent::PrevMatch)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Command;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Command {})
    }
}
