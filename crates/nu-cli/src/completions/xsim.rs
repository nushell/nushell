#[cfg(feature = "xsim-pinyin")]
mod pinyin;
#[cfg(feature = "xsim-romanization")]
mod romanization;

use nu_protocol::XsimConfig;

use super::CompletionOptions;
#[cfg(any(feature = "xsim-pinyin", feature = "xsim-romanization"))]
use super::completion_options::NuMatcher;

#[cfg(feature = "xsim-pinyin")]
use self::pinyin::PinyinProvider;
#[cfg(feature = "xsim-romanization")]
use self::romanization::RomanizationProvider;

#[cfg(any(feature = "xsim-pinyin", feature = "xsim-romanization"))]
const QUOTES: [char; 3] = ['"', '\'', '`'];

#[cfg(any(feature = "xsim-pinyin", feature = "xsim-romanization"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SearchKeyKind {
    #[cfg(feature = "xsim-pinyin")]
    Pinyin,
    #[cfg(feature = "xsim-pinyin")]
    PinyinInitials,
    #[cfg(feature = "xsim-romanization")]
    Romanization,
}

#[cfg(any(feature = "xsim-pinyin", feature = "xsim-romanization"))]
#[derive(Debug, PartialEq, Eq)]
struct SearchKey {
    kind: SearchKeyKind,
    text: String,
}

#[cfg(any(feature = "xsim-pinyin", feature = "xsim-romanization"))]
#[derive(Default)]
struct SearchKeys(Vec<SearchKey>);

#[cfg(any(feature = "xsim-pinyin", feature = "xsim-romanization"))]
impl SearchKeys {
    fn push(&mut self, kind: SearchKeyKind, text: String) {
        if text.is_empty() || self.0.iter().any(|key| key.text == text) {
            return;
        }
        self.0.push(SearchKey { kind, text });
    }
}

#[cfg(any(feature = "xsim-pinyin", feature = "xsim-romanization"))]
trait SearchKeyProvider {
    fn search_keys(&self, candidate: &str, output: &mut SearchKeys);
}

/// The statically dispatched providers enabled for one completion request.
pub(crate) struct ProviderRegistry {
    #[cfg(feature = "xsim-pinyin")]
    pinyin: Option<PinyinProvider>,
    #[cfg(feature = "xsim-romanization")]
    romanization: Option<RomanizationProvider>,
}

impl ProviderRegistry {
    pub(crate) fn for_paths(config: &XsimConfig) -> Option<Self> {
        if config.targets.paths {
            Self::new(config)
        } else {
            None
        }
    }

    pub(crate) fn for_commands(config: &XsimConfig) -> Option<Self> {
        if config.targets.commands {
            Self::new(config)
        } else {
            None
        }
    }

    fn new(config: &XsimConfig) -> Option<Self> {
        if !config.enabled || !has_enabled_provider(config) {
            return None;
        }

        Some(Self {
            #[cfg(feature = "xsim-pinyin")]
            pinyin: config.pinyin.enabled.then_some(PinyinProvider),
            #[cfg(feature = "xsim-romanization")]
            romanization: config
                .romanization
                .enabled
                .then(|| RomanizationProvider::new(&config.romanization.language_hints)),
        })
    }

    #[cfg(any(feature = "xsim-pinyin", feature = "xsim-romanization"))]
    fn search_keys(&self, candidate: &str) -> SearchKeys {
        let mut output = SearchKeys::default();

        #[cfg(feature = "xsim-pinyin")]
        if let Some(provider) = &self.pinyin {
            provider.search_keys(candidate, &mut output);
        }

        #[cfg(feature = "xsim-romanization")]
        if let Some(provider) = &self.romanization {
            provider.search_keys(candidate, &mut output);
        }

        output
    }
}

#[cfg(all(feature = "xsim-pinyin", feature = "xsim-romanization"))]
fn has_enabled_provider(config: &XsimConfig) -> bool {
    config.pinyin.enabled || config.romanization.enabled
}

#[cfg(all(feature = "xsim-pinyin", not(feature = "xsim-romanization")))]
fn has_enabled_provider(config: &XsimConfig) -> bool {
    config.pinyin.enabled
}

#[cfg(all(not(feature = "xsim-pinyin"), feature = "xsim-romanization"))]
fn has_enabled_provider(config: &XsimConfig) -> bool {
    config.romanization.enabled
}

#[cfg(not(any(feature = "xsim-pinyin", feature = "xsim-romanization")))]
fn has_enabled_provider(_config: &XsimConfig) -> bool {
    false
}

/// Matches generated keys with Nushell's native algorithms while retaining real candidates.
#[cfg(any(feature = "xsim-pinyin", feature = "xsim-romanization"))]
pub(crate) struct XsimMatcher<'options, 'providers, T> {
    providers: &'providers ProviderRegistry,
    hidden: bool,
    candidates: Vec<Option<T>>,
    #[cfg(feature = "xsim-pinyin")]
    pinyin: Option<NuMatcher<'options, usize>>,
    #[cfg(feature = "xsim-pinyin")]
    pinyin_initials: Option<NuMatcher<'options, usize>>,
    #[cfg(feature = "xsim-romanization")]
    romanization: Option<NuMatcher<'options, usize>>,
}

#[cfg(any(feature = "xsim-pinyin", feature = "xsim-romanization"))]
impl<'options, 'providers, T> XsimMatcher<'options, 'providers, T> {
    pub(crate) fn new(
        input: &str,
        options: &'options CompletionOptions,
        providers: &'providers ProviderRegistry,
    ) -> Option<Self> {
        let input = input.trim_matches(QUOTES);
        let (input, hidden) = match input.strip_prefix('.') {
            Some(input) => (input, true),
            None => (input, false),
        };

        if input.is_empty() {
            return None;
        }

        Some(Self {
            providers,
            hidden,
            candidates: Vec::new(),
            #[cfg(feature = "xsim-pinyin")]
            pinyin: providers
                .pinyin
                .as_ref()
                .map(|_| NuMatcher::new(input, options, true)),
            #[cfg(feature = "xsim-pinyin")]
            pinyin_initials: providers
                .pinyin
                .as_ref()
                .map(|_| NuMatcher::new(input, options, true)),
            #[cfg(feature = "xsim-romanization")]
            romanization: providers
                .romanization
                .as_ref()
                .map(|_| NuMatcher::new(input, options, true)),
        })
    }

    /// Adds a real candidate only when at least one generated search key matches.
    pub(crate) fn add(&mut self, candidate: &str, item: T) -> bool {
        let candidate = match (self.hidden, candidate.strip_prefix('.')) {
            (true, Some(candidate)) => candidate,
            (true, None) | (false, Some(_)) => return false,
            (false, None) => candidate,
        };

        if candidate.is_ascii() {
            return false;
        }

        let candidate_id = self.candidates.len();
        let mut matched = false;
        for key in self.providers.search_keys(candidate).0 {
            let added = match key.kind {
                #[cfg(feature = "xsim-pinyin")]
                SearchKeyKind::Pinyin => self
                    .pinyin
                    .as_mut()
                    .is_some_and(|matcher| matcher.add(key.text, candidate_id)),
                #[cfg(feature = "xsim-pinyin")]
                SearchKeyKind::PinyinInitials => self
                    .pinyin_initials
                    .as_mut()
                    .is_some_and(|matcher| matcher.add(key.text, candidate_id)),
                #[cfg(feature = "xsim-romanization")]
                SearchKeyKind::Romanization => self
                    .romanization
                    .as_mut()
                    .is_some_and(|matcher| matcher.add(key.text, candidate_id)),
            };
            matched |= added;
        }

        if matched {
            self.candidates.push(Some(item));
        }
        matched
    }

    /// Returns candidates in provider priority order and discards generated-key indices.
    pub(crate) fn results(self) -> Vec<T> {
        let mut candidates = self.candidates;
        let mut output = Vec::with_capacity(candidates.len());

        #[cfg(feature = "xsim-pinyin")]
        append_results(self.pinyin, &mut candidates, &mut output);
        #[cfg(feature = "xsim-pinyin")]
        append_results(self.pinyin_initials, &mut candidates, &mut output);
        #[cfg(feature = "xsim-romanization")]
        append_results(self.romanization, &mut candidates, &mut output);

        output
    }
}

#[cfg(any(feature = "xsim-pinyin", feature = "xsim-romanization"))]
fn append_results<T>(
    matcher: Option<NuMatcher<'_, usize>>,
    candidates: &mut [Option<T>],
    output: &mut Vec<T>,
) {
    let Some(matcher) = matcher else {
        return;
    };

    for (candidate_id, _) in matcher.results() {
        if let Some(candidate) = candidates.get_mut(candidate_id).and_then(Option::take) {
            output.push(candidate);
        }
    }
}

#[cfg(not(any(feature = "xsim-pinyin", feature = "xsim-romanization")))]
pub(crate) struct XsimMatcher<T>(std::marker::PhantomData<T>);

#[cfg(not(any(feature = "xsim-pinyin", feature = "xsim-romanization")))]
impl<T> XsimMatcher<T> {
    pub(crate) fn new(
        _input: &str,
        _options: &CompletionOptions,
        _providers: &ProviderRegistry,
    ) -> Option<Self> {
        None
    }

    pub(crate) fn add(&mut self, _candidate: &str, _item: T) -> bool {
        false
    }

    pub(crate) fn results(self) -> Vec<T> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use nu_protocol::XsimConfig;

    use super::ProviderRegistry;

    #[test]
    fn disabled_registry_is_unavailable() {
        assert!(ProviderRegistry::for_paths(&XsimConfig::default()).is_none());
        assert!(ProviderRegistry::for_commands(&XsimConfig::default()).is_none());
    }

    #[cfg(not(any(feature = "xsim-pinyin", feature = "xsim-romanization")))]
    #[test]
    fn registry_is_unavailable_without_compiled_providers() {
        let mut config = XsimConfig {
            enabled: true,
            ..XsimConfig::default()
        };
        config.pinyin.enabled = true;

        assert!(ProviderRegistry::for_paths(&config).is_none());
    }

    #[cfg(all(feature = "xsim-pinyin", feature = "xsim-romanization"))]
    #[test]
    fn duplicate_keys_keep_the_higher_priority_provider() {
        use super::SearchKeyKind;

        let mut config = XsimConfig {
            enabled: true,
            ..XsimConfig::default()
        };
        config.pinyin.enabled = true;
        let Some(providers) = ProviderRegistry::for_paths(&config) else {
            panic!("both providers should be compiled and enabled");
        };

        let keys = providers.search_keys("下载").0;
        assert_eq!(2, keys.len());
        assert_eq!(SearchKeyKind::Pinyin, keys[0].kind);
        assert_eq!("xiazai", keys[0].text);
        assert_eq!(SearchKeyKind::PinyinInitials, keys[1].kind);
        assert_eq!("xz", keys[1].text);
    }
}
