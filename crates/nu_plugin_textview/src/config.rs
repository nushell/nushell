use nu_protocol::Value;

pub struct Config {
    pub term_width: usize,
    pub tab_width: u64,
    pub colored_output: bool,
    pub true_color: bool,
    pub header: bool,
    pub line_numbers: bool,
    pub grid: bool,
    pub vcs_modification_markers: bool,
    pub snip: bool,
    pub wrapping_mode: bat::WrappingMode,
    pub use_italics: bool,
    pub paging_mode: bat::PagingMode,
    pub pager: String,
    pub line_ranges: bat::line_range::LineRanges,
    // TODO: Not configurable
    #[allow(unused)]
    highlight_range: String,
    // TODO: Not configurable
    pub highlight_range_from: u64,
    // TODO: Not configurable
    pub highlight_range_to: u64,
    pub theme: String,
}

impl From<&Value> for Config {
    fn from(value: &Value) -> Self {
        let mut config = Config::default();

        for (idx, entry) in value.row_entries() {
            match idx.as_ref() {
                "term_width" => {
                    config.term_width = entry.as_u64().unwrap_or(config.term_width as u64) as usize;
                }
                "tab_width" => {
                    config.tab_width = entry.as_u64().unwrap_or(4_u64);
                }
                "colored_output" => config.colored_output = entry.as_bool().unwrap_or(true),
                "true_color" => config.true_color = entry.as_bool().unwrap_or(true),
                "header" => config.header = entry.as_bool().unwrap_or(true),
                "line_numbers" => config.line_numbers = entry.as_bool().unwrap_or(true),
                "grid" => config.grid = entry.as_bool().unwrap_or(true),
                "vcs_modification_markers" => {
                    config.vcs_modification_markers = entry.as_bool().unwrap_or(true)
                }
                "snip" => config.snip = entry.as_bool().unwrap_or(true),
                "wrapping_mode" => {
                    config.wrapping_mode = match entry.as_string() {
                        Ok(s) if s.to_lowercase() == "nowrapping" => {
                            bat::WrappingMode::NoWrapping(true)
                        }
                        Ok(s) if s.to_lowercase() == "character" => bat::WrappingMode::Character,
                        _ => bat::WrappingMode::NoWrapping(true),
                    }
                }
                "use_italics" => config.use_italics = entry.as_bool().unwrap_or(true),
                "paging_mode" => {
                    config.paging_mode = match entry.as_string() {
                        Ok(s) if s.to_lowercase() == "always" => bat::PagingMode::Always,
                        Ok(s) if s.to_lowercase() == "never" => bat::PagingMode::Never,
                        Ok(s) if s.to_lowercase() == "quitifonescreen" => {
                            bat::PagingMode::QuitIfOneScreen
                        }
                        _ => bat::PagingMode::QuitIfOneScreen,
                    }
                }
                "pager" => config.pager = entry.as_string().unwrap_or_else(|_| "less".to_string()),
                // TODO: not real sure what to do with this
                "line_ranges" => config.line_ranges = bat::line_range::LineRanges::all(),
                "highlight_range" => config.highlight_range = "0,0".into(),
                "theme" => {
                    config.theme = value
                        .as_string()
                        .unwrap_or_else(|_| "OneDarkHalf".to_string())
                }
                _ => (),
            }
        }

        config
    }
}

impl Default for Config {
    fn default() -> Self {
        let (term_width, _) = term_size::dimensions().unwrap_or((80, 20));

        Self {
            term_width,
            tab_width: 4,
            colored_output: true,
            true_color: true,
            header: true,
            line_numbers: true,
            grid: true,
            vcs_modification_markers: true,
            snip: true,
            wrapping_mode: bat::WrappingMode::NoWrapping(true),
            use_italics: true,
            paging_mode: bat::PagingMode::QuitIfOneScreen,
            pager: "less".into(),
            line_ranges: bat::line_range::LineRanges::all(),
            highlight_range: "0.0".into(),
            highlight_range_from: 0,
            highlight_range_to: 0,
            theme: "OneHalfDark".into(),
        }
    }
}
