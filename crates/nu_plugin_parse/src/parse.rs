use nu_source::Tag;
use regex::Regex;

pub struct Parse {
    pub regex: Regex,
    pub name: Tag,
    pub column_names: Vec<String>,
}

impl Parse {
    #[allow(clippy::trivial_regex)]
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Parse {
            regex: Regex::new("")?,
            name: Tag::unknown(),
            column_names: vec![],
        })
    }
}
