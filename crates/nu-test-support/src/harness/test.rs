use std::{
    any::Any,
    collections::HashSet,
    fmt::Debug,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
    thread::Scope,
};

use kitest::{
    Whatever,
    capture::DefaultPanicHookProvider,
    outcome::TestOutcome,
    runner::{DefaultRunner, SimpleRunner},
    test::{TestMeta, TestResult},
};
use nu_experimental::ExperimentalOption;
use nu_utils::downcast;

use crate::{
    harness::{deps::*, group::RUN_TEST_GROUP_IN_SERIAL},
    tester::*,
};

#[cfg(feature = "plugin")]
use std::collections::HashMap;

#[derive(Debug)]
pub struct Extra {
    pub run_in_serial: bool,
    pub experimental_options: &'static [(&'static ExperimentalOption, bool)],
    pub environment_variables: &'static [(&'static str, &'static str)],
    pub dependencies: &'static [&'static Dependency<'static>],
}

#[derive(Debug)]
pub struct TestRunner {
    parallel: DefaultRunner<DefaultPanicHookProvider, TestScopeFactory>,
    serial: SimpleRunner<DefaultPanicHookProvider, TestScopeFactory>,
    exact: bool,
}

impl Default for TestRunner {
    fn default() -> Self {
        Self {
            parallel: DefaultRunner::default().with_test_scope_factory(TestScopeFactory::default()),
            serial: SimpleRunner::default().with_test_scope_factory(TestScopeFactory::default()),
            exact: false,
        }
    }
}

impl TestRunner {
    pub fn with_test_scope_factory(self, test_scope_factory: TestScopeFactory) -> Self {
        Self {
            parallel: self
                .parallel
                .with_test_scope_factory(test_scope_factory.clone()),
            serial: self.serial.with_test_scope_factory(test_scope_factory),
            ..self
        }
    }

    pub fn with_thread_count(self, thread_count: NonZeroUsize) -> Self {
        Self {
            parallel: self.parallel.with_thread_count(thread_count),
            ..self
        }
    }

    pub fn with_exact(self, exact: bool) -> Self {
        Self { exact, ..self }
    }
}

enum NuTestRunnerIterator<IP, IS> {
    Parallel(IP),
    Serial(IS),
}

impl<'t, IP, IS> Iterator for NuTestRunnerIterator<IP, IS>
where
    IP: Iterator<Item = (&'t TestMeta<Extra>, TestOutcome)>,
    IS: Iterator<Item = (&'t TestMeta<Extra>, TestOutcome)>,
    Extra: 't,
{
    type Item = (&'t TestMeta<Extra>, TestOutcome);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Parallel(iter) => iter.next(),
            Self::Serial(iter) => iter.next(),
        }
    }
}

impl<'t> kitest::runner::TestRunner<'t, Extra> for TestRunner {
    fn run<'s, I, F>(
        &self,
        tests: I,
        scope: &'s Scope<'s, 't>,
    ) -> impl Iterator<Item = (&'t TestMeta<Extra>, kitest::outcome::TestOutcome)>
    where
        I: ExactSizeIterator<Item = (F, &'t TestMeta<Extra>)>,
        F: (Fn() -> kitest::outcome::TestStatus) + Send + 's,
        Extra: 't,
    {
        match self.exact || RUN_TEST_GROUP_IN_SERIAL.load(Ordering::Relaxed) {
            false => NuTestRunnerIterator::Parallel(
                <DefaultRunner<_, _> as kitest::runner::TestRunner<Extra>>::run(
                    &self.parallel,
                    tests,
                    scope,
                ),
            ),
            true => {
                NuTestRunnerIterator::Serial(<SimpleRunner<_, _> as kitest::runner::TestRunner<
                    Extra,
                >>::run(&self.serial, tests, scope))
            }
        }
    }

    fn worker_count(&self, tests_count: usize) -> NonZeroUsize {
        match RUN_TEST_GROUP_IN_SERIAL.load(Ordering::Relaxed) {
            true => const { NonZeroUsize::new(1).unwrap() },
            false => <DefaultRunner<_, _> as kitest::runner::TestRunner<Extra>>::worker_count(
                &self.parallel,
                tests_count,
            ),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct TestScopeFactory {
    target_dir: Option<PathBuf>,

    #[cfg(feature = "plugin")]
    preloaded_plugins: HashMap<&'static Dependency<'static>, PreloadedPlugin>,
}

impl TestScopeFactory {
    pub fn with_target_dir(self, target_dir: impl Into<Option<PathBuf>>) -> Self {
        Self {
            target_dir: target_dir.into(),
            ..self
        }
    }

    #[cfg(feature = "plugin")]
    pub fn with_preloaded_plugins(
        self,
        preloaded_plugins: HashMap<&'static Dependency<'static>, PreloadedPlugin>,
    ) -> Self {
        Self {
            preloaded_plugins,
            ..self
        }
    }
}

impl<'t> kitest::runner::scope::TestScopeFactory<'t, Extra> for TestScopeFactory {
    type Scope<'f>
        = TestScope<'f>
    where
        't: 'f,
        Self: 'f;

    fn make_scope<'f>(&'f self) -> Self::Scope<'f>
    where
        't: 'f,
    {
        TestScope {
            target_dir: self.target_dir.as_deref(),

            #[cfg(feature = "plugin")]
            preloaded_plugins: &self.preloaded_plugins,
        }
    }
}

#[derive(Debug)]
pub struct TestScope<'f> {
    target_dir: Option<&'f Path>,

    #[cfg(feature = "plugin")]
    preloaded_plugins: &'f HashMap<&'f Dependency<'f>, PreloadedPlugin>,
}

impl<'f, 't> kitest::runner::scope::TestScope<'t, Extra> for TestScope<'f> {
    fn before_test(&mut self, meta: &'t TestMeta<Extra>) {
        // TODO: load preloaded plugins somehow

        PATH_ENV_AUTO_LOAD.with_borrow_mut(|paths| {
            paths.clear();

            let Some(target_dir) = self.target_dir else {
                return;
            };

            let dependency_paths: HashSet<_> = meta
                .extra
                .dependencies
                .iter()
                .map(|dep| {
                    dep.path(target_dir)
                        .parent()
                        .expect("bin lives in target dir")
                        .to_path_buf()
                })
                .collect();

            paths.extend(dependency_paths);
        });

        #[cfg(feature = "plugin")]
        PLUGIN_AUTO_LOAD.with_borrow_mut(|auto_loaders| {
            auto_loaders.clear();
            auto_loaders.extend(
                meta.extra
                    .dependencies
                    .iter()
                    .filter(|dep| dep.is_plugin)
                    .flat_map(|dep| self.preloaded_plugins.get(dep))
                    .map(|plugin| PluginAutoLoader {
                        identity: plugin.identity.clone(),
                        plugin: Some(plugin.plugin.clone()),
                        signatures: Some(plugin.signatures.clone()),
                    }),
            );
        });
    }
}

pub trait IntoTestResult {
    fn into_test_result(self) -> TestResult;
}

impl IntoTestResult for () {
    fn into_test_result(self) -> TestResult {
        self.into()
    }
}

impl<E: Debug + Any> IntoTestResult for Result<(), E> {
    fn into_test_result(self) -> TestResult {
        let Err(err) = self else {
            return TestResult(Ok(None));
        };

        match downcast::<E, TestError>(err) {
            Ok(test_error) => TestResult(Err(Whatever::from(test_error))),
            Err(err) => Err(err).into(),
        }
    }
}
