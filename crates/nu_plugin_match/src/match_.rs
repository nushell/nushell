use regex::Regex;

pub struct Match {
    pub column: String,
    pub regex: Regex,
}

impl Match {
    #[allow(clippy::trivial_regex)]
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Match {
            column: String::new(),
            regex: Regex::new("")?,
        })
    }
}
