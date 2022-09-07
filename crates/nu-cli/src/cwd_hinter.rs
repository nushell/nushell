use nu_ansi_term::{Color, Style};
use reedline::{CommandLineSearch, Hinter, SearchDirection, SearchFilter, SearchQuery};

pub(crate) struct CwdHinter {
    style: Style,
    current_hint: String,
    min_chars: usize,
    cwd: Option<String>,
}

impl Hinter for CwdHinter {
    fn handle(
        &mut self,
        line: &str,
        #[allow(unused_variables)] pos: usize,
        history: &dyn reedline::History,
        use_ansi_coloring: bool,
    ) -> String {
        self.current_hint = if line.chars().count() >= self.min_chars {
            let mut search_filter = SearchFilter::anything();
            search_filter.command_line = Some(CommandLineSearch::Prefix(line.to_string()));
            search_filter.cwd_exact = self.cwd.clone();

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
        self.current_hint.clone()
    }

    fn next_hint_token(&self) -> String {
        let mut reached_content = false;
        let result: String = self
            .current_hint
            .chars()
            .take_while(|c| match (c.is_whitespace(), reached_content) {
                (true, true) => false,
                (true, false) => true,
                (false, true) => true,
                (false, false) => {
                    reached_content = true;
                    true
                }
            })
            .collect();
        result
    }
}

impl Default for CwdHinter {
    fn default() -> Self {
        CwdHinter {
            style: Style::new().fg(Color::LightGray),
            current_hint: String::new(),
            min_chars: 1,
            cwd: None,
        }
    }
}

impl CwdHinter {
    /// A builder that sets the style applied to the hint as part of the buffer
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// A builder that sets the current working directory to filter history completions by.
    pub fn with_cwd(mut self, cwd: Option<String>) -> Self {
        self.cwd = cwd;
        self
    }
}
