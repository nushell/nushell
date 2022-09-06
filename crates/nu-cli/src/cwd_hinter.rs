use nu_ansi_term::{Color, Style};
use nu_protocol::engine::EngineState;
use reedline::{CommandLineSearch, Hinter, SearchDirection, SearchFilter, SearchQuery};

struct CwdHinter {
    style: Style,
    current_hint: String,
    min_chars: usize,
    engine_state: Arc<EngineState>,
}

impl Hinter for CwdHinter {
    fn handle(
        &mut self,
        line: &str,
        pos: usize,
        history: &dyn reedline::History,
        use_ansi_coloring: bool,
    ) -> String {
        self.current_hint = if line.chars().count() >= self.min_chars {
            let cwd = if let Some(d) = self.engine_state.get_env_var("PWD") {
                match d.as_string() {
                    Ok(s) => s,
                    Err(_) => "".to_string(),
                }
            } else {
                "".to_string()
            };
            
            let mut search_filter = SearchFilter::anything();
            search_filter.command_line = Some(CommandLineSearch::Prefix(line.to_string()));
            search_filter.cwd_exact = Some(cwd);

            history
                .search(SearchQuery {
                    direction: SearchDirection::Backward,
                    start_time: None,
                    end_time: None,
                    start_id: None,
                    end_id: None,
                    limit: Some(1),
                    filter: search_filter,
                })
                .expect("todo: error handling")
                .get(0)
                .map_or_else(String::new, |entry| {
                    entry
                        .command_line
                        .get(line.len()..)
                        .unwrap_or_default()
                        .to_string()
                })
        } else {
            String::new()
        };

        if use_ansi_coloring && !self.current_hint.is_empty() {
            self.style.paint(&self.current_hint).to_string()
        } else {
            self.current_hint.clone()
        }
    }

    fn complete_hint(&self) -> String {
        todo!()
    }

    fn next_hint_token(&self) -> String {
        todo!()
    }
}
