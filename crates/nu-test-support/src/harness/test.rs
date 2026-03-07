use std::{num::NonZeroUsize, sync::atomic::Ordering, thread::Scope};

use kitest::{
    capture::DefaultPanicHookProvider,
    outcome::TestOutcome,
    runner::{DefaultRunner, SimpleRunner, scope::NoScopeFactory},
    test::TestMeta,
};
use nu_experimental::ExperimentalOption;

use crate::harness::group::RUN_TEST_GROUP_IN_SERIAL;

pub struct Extra {
    pub run_in_serial: bool,
    pub experimental_options: &'static [(&'static ExperimentalOption, bool)],
    pub environment_variables: &'static [(&'static str, &'static str)],
}

#[derive(Debug, Default)]
pub struct TestRunner {
    parallel: DefaultRunner<DefaultPanicHookProvider, NoScopeFactory>,
    serial: SimpleRunner<DefaultPanicHookProvider, NoScopeFactory>,
    exact: bool,
}

impl TestRunner {
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
