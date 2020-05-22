use ansi_term::Color;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::str::Bytes;

pub struct Theme {
    pub open_delimiter: Color,
    pub close_delimiter: Color,
    pub r#type: Color,
    pub identifier: Color,
    pub it_variable: Color,
    pub variable: Color,
    pub operator: Color,
    pub dot: Color,
    pub dot_dot: Color,
    pub internal_command: Color,
    pub external_command: Color,
    pub external_word: Color,
    pub bare_member: Color,
    pub string_member: Color,
    pub string: Color,
    pub path: Color,
    pub word: Color,
    pub keyword: Color,
    pub pipe: Color,
    pub glob_pattern: Color,
    pub flag: Color,
    pub shorthand_flag: Color,
    pub int: Color,
    pub decimal: Color,
    pub garbage: Color,
    pub whitespace: Color,
    pub separator: Color,
    pub comment: Color,
    pub size_number: Color,
    pub size_unit: Color,
}

impl Theme {
    pub fn load(contents: &str) -> Result<Theme, Box<dyn Error>> {
        let theme_json: ThemeJSON = serde_json::from_str(contents)?;
        json_to_theme(theme_json)
    }
}

fn json_to_theme(json: ThemeJSON) -> Result<Theme, Box<dyn Error>> {
    let theme = Theme {
        open_delimiter: string_to_color(&json.open_delimiter)?,
        close_delimiter: string_to_color(&json.close_delimiter)?,
        r#type: string_to_color(&json.r#type)?,
        identifier: string_to_color(&json.identifier)?,
        it_variable: string_to_color(&json.it_variable)?,
        variable: string_to_color(&json.variable)?,
        operator: string_to_color(&json.operator)?,
        dot: string_to_color(&json.dot)?,
        dot_dot: string_to_color(&json.dot_dot)?,
        internal_command: string_to_color(&json.internal_command)?,
        external_command: string_to_color(&json.external_command)?,
        external_word: string_to_color(&json.external_word)?,
        bare_member: string_to_color(&json.bare_member)?,
        string_member: string_to_color(&json.string_member)?,
        string: string_to_color(&json.string)?,
        path: string_to_color(&json.path)?,
        word: string_to_color(&json.word)?,
        keyword: string_to_color(&json.keyword)?,
        pipe: string_to_color(&json.pipe)?,
        glob_pattern: string_to_color(&json.glob_pattern)?,
        flag: string_to_color(&json.flag)?,
        shorthand_flag: string_to_color(&json.shorthand_flag)?,
        int: string_to_color(&json.int)?,
        decimal: string_to_color(&json.decimal)?,
        garbage: string_to_color(&json.garbage)?,
        whitespace: string_to_color(&json.whitespace)?,
        separator: string_to_color(&json.separator)?,
        comment: string_to_color(&json.comment)?,
        size_number: string_to_color(&json.size_number)?,
        size_unit: string_to_color(&json.size_unit)?,
    };
    Ok(theme)
}

fn string_to_color(s: &str) -> Result<Color, Box<dyn Error>> {
    let mut bytes = s.bytes();
    let r = xtoi(&mut bytes)?;
    let g = xtoi(&mut bytes)?;
    let b = xtoi(&mut bytes)?;
    Ok(Color::RGB(r, g, b))
}

fn xtoi(b: &mut Bytes) -> Result<u8, Box<dyn Error>> {
    let upper = b.next().ok_or(theme_error("too short"))?;
    let lower = b.next().ok_or(theme_error("too short"))?;
    let mut val = numerical_value(upper)?;
    val = (val << 4) | numerical_value(lower)?;
    Ok(val)
}

fn numerical_value(character: u8) -> Result<u8, Box<dyn Error>> {
    match character {
        b'0'..=b'9' => Ok(character - b'0'),
        b'a'..=b'z' => Ok(character - (b'a' - 10)),
        _ => return Err(theme_error("invalid character")),
    }
}

#[derive(Deserialize, Serialize, Debug)]
struct ThemeJSON {
    open_delimiter: String,
    close_delimiter: String,
    r#type: String,
    identifier: String,
    it_variable: String,
    variable: String,
    operator: String,
    dot: String,
    dot_dot: String,
    internal_command: String,
    external_command: String,
    external_word: String,
    bare_member: String,
    string_member: String,
    string: String,
    path: String,
    word: String,
    keyword: String,
    pipe: String,
    glob_pattern: String,
    flag: String,
    shorthand_flag: String,
    int: String,
    decimal: String,
    garbage: String,
    whitespace: String,
    separator: String,
    comment: String,
    size_number: String,
    size_unit: String,
}

#[derive(Debug)]
struct ThemeError {
    msg: String,
}

fn theme_error(msg: &str) -> Box<ThemeError> {
    Box::new(ThemeError {
        msg: msg.to_string(),
    })
}

impl Error for ThemeError {}

impl fmt::Display for ThemeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}
