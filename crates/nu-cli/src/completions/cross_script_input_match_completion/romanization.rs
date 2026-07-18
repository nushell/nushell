//! Universal romanization search-key generation.

use std::sync::OnceLock;

use uroman::{Uroman, rom_format};

use super::{SearchKeyKind, SearchKeyProvider, SearchKeys};

pub(super) struct RomanizationProvider {
    language_hints: Vec<String>,
}

static UROMAN: OnceLock<Uroman> = OnceLock::new();

fn uroman() -> &'static Uroman {
    UROMAN.get_or_init(Uroman::new)
}

impl RomanizationProvider {
    pub(super) fn new(language_hints: &[String]) -> Self {
        let mut hints = Vec::with_capacity(language_hints.len());
        for hint in language_hints {
            if !hint.is_empty() && !hints.contains(hint) {
                hints.push(hint.clone());
            }
        }

        Self {
            language_hints: hints,
        }
    }

    fn add_key(&self, candidate: &str, hint: Option<&str>, output: &mut SearchKeys) {
        let key = uroman()
            .romanize_string::<rom_format::Str>(candidate, hint)
            .to_string();
        if key != candidate {
            output.push(SearchKeyKind::Romanization, key);
        }
    }
}

impl SearchKeyProvider for RomanizationProvider {
    fn search_keys(&self, candidate: &str, output: &mut SearchKeys) {
        self.add_key(candidate, None, output);
        for hint in &self.language_hints {
            self.add_key(candidate, Some(hint), output);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn romanizations(candidate: &str, hints: &[&str]) -> Vec<String> {
        let hints = hints
            .iter()
            .map(|hint| (*hint).to_owned())
            .collect::<Vec<_>>();
        let provider = RomanizationProvider::new(&hints);
        let mut keys = SearchKeys::default();
        provider.search_keys(candidate, &mut keys);
        keys.0.into_iter().map(|key| key.text).collect()
    }

    #[test]
    fn provides_documented_multiscript_keys() {
        let cases = [
            ("ひらがな.txt", "hiragana.txt"),
            ("ドキュメント", "dokyumento"),
            ("москва.txt", "moskva.txt"),
            ("αθήνα.md", "athena.md"),
            ("한글-파일", "hangeul-pail"),
            ("مرحبا.txt", "mrhba.txt"),
            ("नमस्ते-दुनिया", "namaste-duniyaa"),
            ("notes_ドキュメント-v2.txt", "notes_dokyumento-v2.txt"),
        ];

        for (candidate, expected) in cases {
            assert_eq!(vec![expected], romanizations(candidate, &[]));
        }
    }

    #[test]
    fn language_hint_adds_a_distinct_key() {
        assert_eq!(
            vec!["elena.txt", "yelena.txt"],
            romanizations("елена.txt", &["rus", "rus", ""])
        );
    }

    #[test]
    fn han_uses_mandarin_readings_not_japanese_kanji_readings() {
        assert_eq!(vec!["ribenyu"], romanizations("日本語", &["jpn"]));
    }
}
