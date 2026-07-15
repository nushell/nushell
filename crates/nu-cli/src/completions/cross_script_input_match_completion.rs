//! Cross-script input match completion (XSIMC) for native completion candidates.

#[cfg(feature = "xsim-japanese-romaji")]
mod japanese_romaji;
#[cfg(feature = "xsimc-pinyin")]
mod pinyin;
#[cfg(feature = "xsimc-romanization")]
mod romanization;

use nu_protocol::CrossScriptInputMatchCompletionConfig;

use super::CompletionOptions;
#[cfg(any(
    feature = "xsim-japanese-romaji",
    feature = "xsimc-pinyin",
    feature = "xsimc-romanization"
))]
use super::completion_options::NuMatcher;

#[cfg(feature = "xsim-japanese-romaji")]
use self::japanese_romaji::JapaneseReadingProvider;
#[cfg(feature = "xsimc-pinyin")]
use self::pinyin::PinyinProvider;
#[cfg(feature = "xsimc-romanization")]
use self::romanization::RomanizationProvider;

#[cfg(any(
    feature = "xsim-japanese-romaji",
    feature = "xsimc-pinyin",
    feature = "xsimc-romanization"
))]
const QUOTES: [char; 3] = ['"', '\'', '`'];

#[cfg(any(
    feature = "xsim-japanese-romaji",
    feature = "xsimc-pinyin",
    feature = "xsimc-romanization"
))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SearchKeyKind {
    #[cfg(feature = "xsimc-pinyin")]
    Pinyin,
    #[cfg(feature = "xsimc-pinyin")]
    PinyinInitials,
    #[cfg(feature = "xsim-japanese-romaji")]
    JapaneseReading,
    #[cfg(feature = "xsimc-romanization")]
    Romanization,
}

#[cfg(any(
    feature = "xsim-japanese-romaji",
    feature = "xsimc-pinyin",
    feature = "xsimc-romanization"
))]
#[derive(Debug, PartialEq, Eq)]
struct SearchKey {
    kind: SearchKeyKind,
    text: String,
}

#[cfg(any(
    feature = "xsim-japanese-romaji",
    feature = "xsimc-pinyin",
    feature = "xsimc-romanization"
))]
#[derive(Default)]
struct SearchKeys(Vec<SearchKey>);

#[cfg(any(
    feature = "xsim-japanese-romaji",
    feature = "xsimc-pinyin",
    feature = "xsimc-romanization"
))]
impl SearchKeys {
    fn push(&mut self, kind: SearchKeyKind, text: String) {
        if text.is_empty() || self.0.iter().any(|key| key.text == text) {
            return;
        }
        self.0.push(SearchKey { kind, text });
    }
}

#[cfg(any(
    feature = "xsim-japanese-romaji",
    feature = "xsimc-pinyin",
    feature = "xsimc-romanization"
))]
trait SearchKeyProvider {
    fn search_keys(&self, candidate: &str, output: &mut SearchKeys);
}

/// The statically dispatched providers enabled for one completion request.
pub(crate) struct ProviderRegistry {
    #[cfg(feature = "xsimc-pinyin")]
    pinyin: Option<PinyinProvider>,
    #[cfg(feature = "xsim-japanese-romaji")]
    japanese_romaji: Option<JapaneseReadingProvider>,
    #[cfg(feature = "xsimc-romanization")]
    romanization: Option<RomanizationProvider>,
}

impl ProviderRegistry {
    pub(crate) fn for_paths(config: &CrossScriptInputMatchCompletionConfig) -> Option<Self> {
        if config.targets.paths {
            Self::new(config)
        } else {
            None
        }
    }

    pub(crate) fn for_commands(config: &CrossScriptInputMatchCompletionConfig) -> Option<Self> {
        if config.targets.commands {
            Self::new(config)
        } else {
            None
        }
    }

    fn new(config: &CrossScriptInputMatchCompletionConfig) -> Option<Self> {
        if !config.enabled || !has_enabled_provider(config) {
            return None;
        }

        Some(Self {
            #[cfg(feature = "xsimc-pinyin")]
            pinyin: config.pinyin.enabled.then_some(PinyinProvider),
            #[cfg(feature = "xsim-japanese-romaji")]
            japanese_romaji: config
                .japanese_romaji
                .enabled
                .then(JapaneseReadingProvider::new)
                .flatten(),
            #[cfg(feature = "xsimc-romanization")]
            romanization: config
                .romanization
                .enabled
                .then(|| RomanizationProvider::new(&config.romanization.language_hints)),
        })
    }

    #[cfg(any(
        feature = "xsim-japanese-romaji",
        feature = "xsimc-pinyin",
        feature = "xsimc-romanization"
    ))]
    fn search_keys(&self, candidate: &str) -> SearchKeys {
        let mut output = SearchKeys::default();

        #[cfg(feature = "xsimc-pinyin")]
        if let Some(provider) = &self.pinyin {
            provider.search_keys(candidate, &mut output);
        }

        #[cfg(feature = "xsim-japanese-romaji")]
        if let Some(provider) = &self.japanese_romaji {
            provider.search_keys(candidate, &mut output);
        }

        #[cfg(feature = "xsimc-romanization")]
        if let Some(provider) = &self.romanization {
            provider.search_keys(candidate, &mut output);
        }

        output
    }
}

#[allow(unused_mut)]
#[cfg_attr(
    not(any(
        feature = "xsim-japanese-romaji",
        feature = "xsimc-pinyin",
        feature = "xsimc-romanization"
    )),
    allow(unused_variables)
)]
fn has_enabled_provider(config: &CrossScriptInputMatchCompletionConfig) -> bool {
    let mut enabled = false;
    #[cfg(feature = "xsimc-pinyin")]
    {
        enabled |= config.pinyin.enabled;
    }
    #[cfg(feature = "xsim-japanese-romaji")]
    {
        enabled |= config.japanese_romaji.enabled;
    }
    #[cfg(feature = "xsimc-romanization")]
    {
        enabled |= config.romanization.enabled;
    }
    enabled
}

/// Matches generated keys with Nushell's native algorithms while retaining real candidates.
#[cfg(any(
    feature = "xsim-japanese-romaji",
    feature = "xsimc-pinyin",
    feature = "xsimc-romanization"
))]
pub(crate) struct CrossScriptInputMatcher<'options, 'providers, T> {
    providers: &'providers ProviderRegistry,
    hidden: bool,
    candidates: Vec<Option<T>>,
    #[cfg(feature = "xsimc-pinyin")]
    pinyin: Option<NuMatcher<'options, usize>>,
    #[cfg(feature = "xsimc-pinyin")]
    pinyin_initials: Option<NuMatcher<'options, usize>>,
    #[cfg(feature = "xsim-japanese-romaji")]
    japanese_romaji: Option<NuMatcher<'options, usize>>,
    #[cfg(feature = "xsimc-romanization")]
    romanization: Option<NuMatcher<'options, usize>>,
}

#[cfg(any(
    feature = "xsim-japanese-romaji",
    feature = "xsimc-pinyin",
    feature = "xsimc-romanization"
))]
impl<'options, 'providers, T> CrossScriptInputMatcher<'options, 'providers, T> {
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
            #[cfg(feature = "xsimc-pinyin")]
            pinyin: providers
                .pinyin
                .as_ref()
                .map(|_| NuMatcher::new(input, options, true)),
            #[cfg(feature = "xsimc-pinyin")]
            pinyin_initials: providers
                .pinyin
                .as_ref()
                .map(|_| NuMatcher::new(input, options, true)),
            #[cfg(feature = "xsim-japanese-romaji")]
            japanese_romaji: providers
                .japanese_romaji
                .as_ref()
                .map(|_| NuMatcher::new(input, options, true)),
            #[cfg(feature = "xsimc-romanization")]
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
                #[cfg(feature = "xsimc-pinyin")]
                SearchKeyKind::Pinyin => self
                    .pinyin
                    .as_mut()
                    .is_some_and(|matcher| matcher.add(key.text, candidate_id)),
                #[cfg(feature = "xsimc-pinyin")]
                SearchKeyKind::PinyinInitials => self
                    .pinyin_initials
                    .as_mut()
                    .is_some_and(|matcher| matcher.add(key.text, candidate_id)),
                #[cfg(feature = "xsim-japanese-romaji")]
                SearchKeyKind::JapaneseReading => self
                    .japanese_romaji
                    .as_mut()
                    .is_some_and(|matcher| matcher.add(key.text, candidate_id)),
                #[cfg(feature = "xsimc-romanization")]
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

        #[cfg(feature = "xsimc-pinyin")]
        append_results(self.pinyin, &mut candidates, &mut output);
        #[cfg(feature = "xsimc-pinyin")]
        append_results(self.pinyin_initials, &mut candidates, &mut output);
        #[cfg(feature = "xsim-japanese-romaji")]
        append_results(self.japanese_romaji, &mut candidates, &mut output);
        #[cfg(feature = "xsimc-romanization")]
        append_results(self.romanization, &mut candidates, &mut output);

        output
    }
}

#[cfg(any(
    feature = "xsim-japanese-romaji",
    feature = "xsimc-pinyin",
    feature = "xsimc-romanization"
))]
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

#[cfg(not(any(
    feature = "xsim-japanese-romaji",
    feature = "xsimc-pinyin",
    feature = "xsimc-romanization"
)))]
pub(crate) struct CrossScriptInputMatcher<T>(std::marker::PhantomData<T>);

#[cfg(not(any(
    feature = "xsim-japanese-romaji",
    feature = "xsimc-pinyin",
    feature = "xsimc-romanization"
)))]
impl<T> CrossScriptInputMatcher<T> {
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
    #[cfg(feature = "xsim-japanese-romaji")]
    use nu_utils::time::Instant;

    use nu_protocol::CrossScriptInputMatchCompletionConfig;

    use super::ProviderRegistry;
    #[cfg(feature = "xsim-japanese-romaji")]
    use super::{super::completion_options::CompletionOptions, CrossScriptInputMatcher};

    #[test]
    fn registry_is_unavailable_without_enabled_providers() {
        assert!(
            ProviderRegistry::for_paths(&CrossScriptInputMatchCompletionConfig::default())
                .is_none()
        );
        assert!(
            ProviderRegistry::for_commands(&CrossScriptInputMatchCompletionConfig::default())
                .is_none()
        );
    }

    #[cfg(not(any(
        feature = "xsim-japanese-romaji",
        feature = "xsimc-pinyin",
        feature = "xsimc-romanization"
    )))]
    #[test]
    fn registry_is_unavailable_without_compiled_providers() {
        let mut config = CrossScriptInputMatchCompletionConfig {
            enabled: true,
            ..CrossScriptInputMatchCompletionConfig::default()
        };
        config.pinyin.enabled = true;

        assert!(ProviderRegistry::for_paths(&config).is_none());
    }

    #[cfg(all(
        feature = "xsim-japanese-romaji",
        feature = "xsimc-pinyin",
        feature = "xsimc-romanization"
    ))]
    #[test]
    fn duplicate_keys_keep_the_higher_priority_provider() {
        use super::SearchKeyKind;

        let mut config = CrossScriptInputMatchCompletionConfig {
            enabled: true,
            ..CrossScriptInputMatchCompletionConfig::default()
        };
        config.pinyin.enabled = true;
        let providers = ProviderRegistry::for_paths(&config)
            .expect("both provider features are enabled for this test");

        let keys = providers.search_keys("下载").0;
        assert_eq!(2, keys.len());
        assert_eq!(SearchKeyKind::Pinyin, keys[0].kind);
        assert_eq!("xiazai", keys[0].text);
        assert_eq!(SearchKeyKind::PinyinInitials, keys[1].kind);
        assert_eq!("xz", keys[1].text);
    }

    #[cfg(feature = "xsim-japanese-romaji")]
    #[test]
    #[ignore = "manual XSIM Japanese reading performance measurement"]
    fn japanese_romaji_performance_probe() {
        let memory = || {
            let system = sysinfo::System::new_all();
            system
                .process(sysinfo::Pid::from(std::process::id() as usize))
                .map(|process| process.memory())
                .unwrap_or_default()
        };
        let mut config = CrossScriptInputMatchCompletionConfig::default();
        config.japanese_romaji.enabled = true;

        let memory_before = memory();
        let started = Instant::now();
        let providers = ProviderRegistry::for_paths(&config)
            .expect("Japanese reading provider should initialize");
        println!("initialization_us={}", started.elapsed().as_micros());
        let memory_after = memory();
        println!(
            "rss_before_bytes={memory_before},rss_after_bytes={memory_after},rss_growth_bytes={}",
            memory_after.saturating_sub(memory_before)
        );

        let options = CompletionOptions::default();
        for count in [100, 1_000, 10_000] {
            for (label, query, japanese) in [
                ("ascii_miss", "zzzz", false),
                ("japanese_hit", "nihongo", true),
                ("japanese_miss", "zzzz", true),
            ] {
                let started = Instant::now();
                let mut matcher = CrossScriptInputMatcher::new(query, &options, &providers)
                    .expect("non-empty query should make a matcher");
                for index in 0..count {
                    let candidate = if japanese {
                        format!("日本語{index}.txt")
                    } else {
                        format!("ascii-{index}.txt")
                    };
                    matcher.add(&candidate, index);
                }
                let matches = matcher.results().len();
                println!(
                    "{label},candidates={count},matches={matches},elapsed_us={}",
                    started.elapsed().as_micros()
                );
            }
        }
    }
}
