//! Pinyin search-key generation.

use pinyin::ToPinyin;

use super::{SearchKeyKind, SearchKeyProvider, SearchKeys};

pub(super) struct PinyinProvider;

impl SearchKeyProvider for PinyinProvider {
    fn search_keys(&self, candidate: &str, output: &mut SearchKeys) {
        let mut full = String::with_capacity(candidate.len());
        let mut initials = String::with_capacity(candidate.len());
        let mut has_pinyin = false;

        for ch in candidate.chars() {
            if let Some(pinyin) = ch.to_pinyin() {
                let plain = pinyin.plain();
                full.push_str(plain);
                if let Some(initial) = plain.chars().next() {
                    initials.push(initial);
                }
                has_pinyin = true;
            } else if ch.is_ascii_alphanumeric() {
                full.push(ch.to_ascii_lowercase());
                initials.push(ch.to_ascii_lowercase());
            }
        }

        if !has_pinyin {
            return;
        }

        output.push(SearchKeyKind::Pinyin, full);
        output.push(SearchKeyKind::PinyinInitials, initials);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provides_full_pinyin_and_initials() {
        let mut keys = SearchKeys::default();
        PinyinProvider.search_keys("项目资料", &mut keys);

        assert_eq!("xiangmuziliao", keys.0[0].text);
        assert_eq!("xmzl", keys.0[1].text);
    }

    #[test]
    fn retains_ascii_fragments() {
        let mut keys = SearchKeys::default();
        PinyinProvider.search_keys("Rust学习资料", &mut keys);

        assert_eq!("rustxuexiziliao", keys.0[0].text);
        assert_eq!("rustxxzl", keys.0[1].text);
    }
}
