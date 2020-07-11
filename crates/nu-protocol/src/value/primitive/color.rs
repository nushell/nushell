use std::fmt;
use serde::{Deserialize, Serialize};
use std::hash::{Hash};
use std::str::FromStr;
use bigdecimal::BigDecimal;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum ColorData {
    Rgba(Rgb, u8),
    Hsla(Hsl, u8),
    Hsva(Hsv, u8)
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Hsl {
    pub hue: BigDecimal,
    pub saturation: BigDecimal,
    pub lightness: BigDecimal,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Hsv {
    pub hue: BigDecimal,
    pub saturation: BigDecimal,
    pub value: BigDecimal,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Color {
    color: ColorData
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.color {
            ColorData::Rgba(color, alpha) => if *alpha == u8::MAX {
                write!(f, "rgb({}, {}, {})", color.red, color.green, color.blue)
            } else {
                write!(f, "rgba({}, {}, {}, {})", color.red, color.green, color.blue, *alpha as f64 / 255.0)
            },
            ColorData::Hsla(color, alpha) => if *alpha == u8::MAX {
                write!(f, "hsl({}%, {}%, {}%)", color.hue, color.saturation, color.lightness)
            } else {
                write!(f, "hsl({}%, {}%, {}%, {})", color.hue, color.saturation, color.lightness, *alpha as f64 / 255.0)
            },
            ColorData::Hsva(color, alpha) => if *alpha == u8::MAX {
                write!(f, "hsv({}%, {}%, {}%)", color.hue, color.saturation, color.value)
            } else {
                write!(f, "hsva({}%, {}%, {}%, {})", color.hue, color.saturation, color.value, *alpha as f64 / 255.0)
            }
        }
    }
}

pub enum ColorParseError {
    UnknownNamespace,
    ArityMismatch,
    BadFormat
}

pub enum ParseError<T> {
    Color(ColorParseError),
    Number(T),
    Alpha(std::num::ParseIntError)
}

pub enum FromStrError {
    Color(ColorParseError),
    Int(std::num::ParseIntError),
    Decimal(bigdecimal::ParseBigDecimalError)
}

fn parse_color<T: std::str::FromStr>(s: &str, namespace: &str, with_alpha: bool, with_percent: bool) -> Result<(T,T,T,u8),ParseError<<T as std::str::FromStr>::Err>> {
    let prefix = match with_alpha {
        false => namespace.to_string(),
        true => namespace.to_owned()+"a"
    };
    let expected_len = match with_alpha {
        false => 3,
        true => 4
    };
    let trimmed = s.trim_start_matches(&prefix).trim();
    if !s.starts_with("(") || !s.starts_with(")") {
        return Err(ParseError::Color(ColorParseError::BadFormat));
    }
    let mut split: Vec<&str> = trimmed.trim_start_matches("(").trim_end_matches(")").split(",").map(|s| s.trim()).collect();
    if split.len() != expected_len {
        return Err(ParseError::Color(ColorParseError::ArityMismatch));
    }
    if with_percent {
        split[0] = split[0].trim_end_matches("%");
        split[1] = split[1].trim_end_matches("%");
        split[2] = split[2].trim_end_matches("%");
    }
    let one = match split[0].parse::<T>() {
        Ok(val) => val,
        Err(err) => return Err(ParseError::Number(err))
    };
    let two = match split[1].parse::<T>() {
        Ok(val) => val,
        Err(err) => return Err(ParseError::Number(err))
    };
    let three = match split[2].parse::<T>() {
        Ok(val) => val,
        Err(err) => return Err(ParseError::Number(err))
    };
    let alpha = match with_alpha {
        false => 255,
        true => match split[3].parse::<u8>() {
            Ok(val) => val,
            Err(err) => return Err(ParseError::Alpha(err))
        }
    };
    Ok((one, two, three, alpha))
}

impl FromStr for Color {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self,Self::Err> {
        if s.starts_with("rgba") {
            match parse_color::<u8>(s, "rgb", true, false) {
                Ok(val) => Ok(Color{color: ColorData::Rgba(Rgb{red: val.0, green: val.1, blue: val.2}, val.3)}),
                Err(err) => match err {
                    ParseError::Color(val) => Err(FromStrError::Color(val)),
                    ParseError::Number(val) | ParseError::Alpha(val) => Err(FromStrError::Int(val))
                },
            }
        } else if s.starts_with("rgb") {
            match parse_color::<u8>(s, "rgb", false, false) {
                Ok(val) => Ok(Color{color: ColorData::Rgba(Rgb{red: val.0, green: val.1, blue: val.2}, val.3)}),
                Err(err) => match err {
                    ParseError::Color(val) => Err(FromStrError::Color(val)),
                    ParseError::Number(val) | ParseError::Alpha(val) => Err(FromStrError::Int(val))
                },
            }
        } else if s.starts_with("hsla") {
            match parse_color::<BigDecimal>(s, "hsl", true, true) {
                Ok(val) => Ok(Color{color: ColorData::Hsla(Hsl{hue: val.0, saturation: val.1, lightness: val.2}, val.3)}),
                Err(err) => match err {
                    ParseError::Color(val) => Err(FromStrError::Color(val)),
                    ParseError::Number(val) => Err(FromStrError::Decimal(val)),
                    ParseError::Alpha(val) => Err(FromStrError::Int(val))
                },
            }
        } else if s.starts_with("hsl") {
            match parse_color::<BigDecimal>(s, "hsl", false, true) {
                Ok(val) => Ok(Color{color: ColorData::Hsla(Hsl{hue: val.0, saturation: val.1, lightness: val.2}, val.3)}),
                Err(err) => match err {
                    ParseError::Color(val) => Err(FromStrError::Color(val)),
                    ParseError::Number(val) => Err(FromStrError::Decimal(val)),
                    ParseError::Alpha(val) => Err(FromStrError::Int(val))
                },
            }
        } else if s.starts_with("hsva") {
            match parse_color::<BigDecimal>(s, "hsv", false, true) {
                Ok(val) => Ok(Color{color: ColorData::Hsva(Hsv{hue: val.0, saturation: val.1, value: val.2}, val.3)}),
                Err(err) => match err {
                    ParseError::Color(val) => Err(FromStrError::Color(val)),
                    ParseError::Number(val) => Err(FromStrError::Decimal(val)),
                    ParseError::Alpha(val) => Err(FromStrError::Int(val))
                },
            }
        } else if s.starts_with("hsv") {
            match parse_color::<BigDecimal>(s, "hsv", true, true) {
                Ok(val) => Ok(Color{color: ColorData::Hsva(Hsv{hue: val.0, saturation: val.1, value: val.2}, val.3)}),
                Err(err) => match err {
                    ParseError::Color(val) => Err(FromStrError::Color(val)),
                    ParseError::Number(val) => Err(FromStrError::Decimal(val)),
                    ParseError::Alpha(val) => Err(FromStrError::Int(val))
                },
            }
        } else {
            Err(FromStrError::Color(ColorParseError::UnknownNamespace))
        }
    }
}

impl Color {
    pub fn is_valid(&self) -> bool {
        match &self.color {
            // u8 and u64 can't hold invalid values
            ColorData::Rgba(_,_) => true,
            ColorData::Hsla(_, _) => true,
            ColorData::Hsva(_, _) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn color_parsing() {
        assert_eq!(super::Color::from_str("rgb(255, 255, 255)").unwrap(), "rgb(255, 255, 255)");
    }
}