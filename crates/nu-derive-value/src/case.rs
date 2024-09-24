use heck::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Case {
    // directly supported by heck
    Pascal,
    Camel,
    Snake,
    Kebab,
    ScreamingSnake,
    Title,
    Cobol,
    Train,

    // custom variants
    Upper,
    Lower,
    Flat,
    ScreamingFlat,
}

impl Case {
    pub fn from_str(s: impl AsRef<str>) -> Option<Self> {
        match s.as_ref() {
            // The matched case are all useful variants from `convert_case` with aliases
            // that `serde` uses.
            "PascalCase" | "UpperCamelCase" => Case::Pascal,
            "camelCase" | "lowerCamelCase" => Case::Camel,
            "snake_case" => Case::Snake,
            "kebab-case" => Case::Kebab,
            "SCREAMING_SNAKE_CASE" | "UPPER_SNAKE_CASE" | "SHOUTY_SNAKE_CASE" => {
                Case::ScreamingSnake
            }
            "Title Case" => Case::Title,
            "COBOL-CASE" | "SCREAMING-KEBAB-CASE" | "UPPER-KEBAB-CASE" => Case::Cobol,
            "Train-Case" => Case::Train,

            "UPPER CASE" | "UPPER WITH SPACES CASE" => Case::Upper,
            "lower case" | "lower with spaces case" => Case::Lower,
            "flatcase" | "lowercase" => Case::Flat,
            "SCREAMINGFLATCASE" | "UPPERFLATCASE" | "UPPERCASE" => Case::ScreamingFlat,

            _ => return None,
        }
        .into()
    }
}

pub trait Casing {
    fn to_case(&self, case: impl Into<Option<Case>>) -> String;
}

impl<T: ToString> Casing for T {
    fn to_case(&self, case: impl Into<Option<Case>>) -> String {
        let s = self.to_string();
        let Some(case) = case.into() else {
            return s.to_string();
        };

        match case {
            Case::Pascal => s.to_upper_camel_case(),
            Case::Camel => s.to_lower_camel_case(),
            Case::Snake => s.to_snake_case(),
            Case::Kebab => s.to_kebab_case(),
            Case::ScreamingSnake => s.to_shouty_snake_case(),
            Case::Title => s.to_title_case(),
            Case::Cobol => s.to_shouty_kebab_case(),
            Case::Train => s.to_train_case(),

            Case::Upper => s.to_shouty_snake_case().replace('_', " "),
            Case::Lower => s.to_snake_case().replace('_', " "),
            Case::Flat => s.to_snake_case().replace('_', ""),
            Case::ScreamingFlat => s.to_shouty_snake_case().replace('_', ""),
        }
    }
}
