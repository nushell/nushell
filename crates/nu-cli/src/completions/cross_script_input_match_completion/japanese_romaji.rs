//! Japanese dictionary-reading search-key generation.

use std::sync::OnceLock;

use lindera::{
    dictionary::load_dictionary, mode::Mode, segmenter::Segmenter, tokenizer::Tokenizer,
};
use wana_kana::ConvertJapanese;

use super::{SearchKeyKind, SearchKeyProvider, SearchKeys};

pub(super) struct JapaneseReadingProvider;

static TOKENIZER: OnceLock<Result<Tokenizer, String>> = OnceLock::new();

fn tokenizer() -> Option<&'static Tokenizer> {
    TOKENIZER
        .get_or_init(|| {
            let dictionary = load_dictionary("embedded://ipadic").map_err(|err| err.to_string())?;
            let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
            Ok(Tokenizer::new(segmenter))
        })
        .as_ref()
        .ok()
}

impl JapaneseReadingProvider {
    pub(super) fn new() -> Option<Self> {
        tokenizer().map(|_| Self)
    }
}

impl SearchKeyProvider for JapaneseReadingProvider {
    fn search_keys(&self, candidate: &str, output: &mut SearchKeys) {
        let Some(tokenizer) = tokenizer() else {
            return;
        };
        let Ok(mut tokens) = tokenizer.tokenize(candidate) else {
            return;
        };

        let mut kana_reading = String::with_capacity(candidate.len());
        let mut has_dictionary_reading = false;
        for token in &mut tokens {
            let surface = token.surface.clone();
            let reading = token.get("reading").map(str::to_owned);
            let pronunciation = token.get("pronunciation").map(str::to_owned);
            match reading.as_deref() {
                Some(reading) if !reading.is_empty() && reading != "*" => {
                    // IPADIC spells the greeting's final particle as ハ in its reading,
                    // while its pronunciation correctly records ワ. Keep orthographic
                    // readings elsewhere so long vowels remain `ou`, as in `toukyou`.
                    let reading = match pronunciation.as_deref() {
                        Some(pronunciation)
                            if surface.ends_with('は')
                                && reading.ends_with('ハ')
                                && pronunciation.ends_with('ワ') =>
                        {
                            pronunciation
                        }
                        _ => reading,
                    };
                    kana_reading.push_str(reading);
                    has_dictionary_reading = true;
                }
                _ => kana_reading.push_str(surface.as_ref()),
            }
        }

        if !has_dictionary_reading && !candidate.chars().any(is_kana) {
            return;
        }

        // `wana_kana` treats small `ァ` as a separate vowel after `フ`.
        // Protect this common loanword digraph so ファイル becomes `fairu`.
        let kana_reading = kana_reading.replace("ファ", "fa").replace("ふぁ", "fa");
        let key = kana_reading.as_str().to_romaji();
        if key != candidate {
            output.push(SearchKeyKind::JapaneseReading, key);
        }
    }
}

fn is_kana(ch: char) -> bool {
    matches!(
        ch,
        '\u{3040}'..='\u{30ff}' | '\u{31f0}'..='\u{31ff}' | '\u{ff66}'..='\u{ff9d}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn readings(candidate: &str) -> Vec<String> {
        let provider = JapaneseReadingProvider::new().expect("embedded IPADIC should load");
        let mut keys = SearchKeys::default();
        provider.search_keys(candidate, &mut keys);
        keys.0.into_iter().map(|key| key.text).collect()
    }

    #[test]
    fn provides_ipadic_readings_and_preserves_mixed_fragments() {
        let cases = [
            ("日本語", "nihongo"),
            ("東京", "toukyou"),
            ("こんにちは", "konnichiwa"),
            ("ドキュメント", "dokyumento"),
            ("日本語ファイル", "nihongofairu"),
            ("関西空港", "kansaikuukou"),
            ("東京-file_2026!", "toukyou-file_2026!"),
        ];

        for (candidate, expected) in cases {
            assert_eq!(
                vec![expected],
                readings(candidate),
                "candidate: {candidate}"
            );
        }
    }

    #[test]
    fn unknown_tokens_are_preserved_in_mixed_candidates() {
        let keys = readings("東京OpenAI新語2026.txt");
        assert_eq!(1, keys.len());
        assert!(keys[0].contains("OpenAI"));
        assert!(keys[0].contains("2026.txt"));
    }

    #[test]
    fn reports_ipadic_analysis_for_ambiguous_han() {
        assert_eq!(1, readings("下载").len());
    }
}
