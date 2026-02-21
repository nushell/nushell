use std::{
    collections::{BTreeMap, HashMap},
    env,
    fmt::{Debug, Display},
    hash::{DefaultHasher, Hash, Hasher},
    num::NonZeroUsize,
    ops::{ControlFlow, Deref},
    process::ExitCode,
    sync::{
        LazyLock,
        atomic::{AtomicBool, Ordering},
    },
    thread::Scope,
};

use crate::{self as nu_test_support};

use itertools::Itertools;
use kitest::{
    capture::DefaultPanicHookProvider,
    filter::DefaultFilter,
    formatter::{
        common::color::ColorSetting, pretty::PrettyFormatter, terse::TerseFormatter,
    },
    group::{
        SimpleGroupRunner, TestGroupBTreeMap, TestGroupOutcomes, TestGroupRunner, TestGrouper,
    },
    outcome::TestOutcome,
    runner::{DefaultRunner, SimpleRunner, TestRunner, scope::NoScopeFactory},
};
#[doc(hidden)]
pub use linkme;
use nu_experimental::ExperimentalOption;

#[doc(hidden)]
pub use kitest::prelude::*;

pub mod macros {
    pub use kitest::{dbg, eprint, eprintln, print, println};
    pub use linkme::distributed_slice as collect_test;
    pub use nu_test_support_macros::test;
}

pub const DEFAULT_THREAD_COUNT_MUL: NonZeroUsize = NonZeroUsize::new(4).unwrap();
pub static DEFAULT_THREAD_COUNT: LazyLock<NonZeroUsize> = LazyLock::new(|| {
    std::thread::available_parallelism()
        .map(|n| n.saturating_mul(DEFAULT_THREAD_COUNT_MUL))
        .unwrap_or(NonZeroUsize::MIN)
});

/// All collected tests.
#[linkme::distributed_slice]
#[linkme(crate = nu_test_support::harness::linkme)]
pub static TESTS: [kitest::test::Test<NuTestMetaExtra>];

pub struct NuTestMetaExtra {
    pub run_in_serial: bool,
    pub experimental_options: &'static [(&'static ExperimentalOption, bool)],
    pub environment_variables: &'static [(&'static str, &'static str)],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct GroupKey(u64);

impl From<&NuTestMetaExtra> for GroupKey {
    fn from(extra: &NuTestMetaExtra) -> Self {
        if extra.experimental_options.is_empty()
            && extra.environment_variables.is_empty()
            && !extra.run_in_serial
        {
            return Self(0);
        }

        let mut hasher = DefaultHasher::new();
        extra.run_in_serial.hash(&mut hasher);
        extra
            .experimental_options
            .iter()
            .sorted()
            .map(|(opt, val)| (opt.identifier(), val))
            .for_each(|item| item.hash(&mut hasher));
        extra
            .environment_variables
            .iter()
            .sorted()
            .for_each(|item| item.hash(&mut hasher));
        GroupKey(hasher.finish())
    }
}

#[derive(Default)]
struct Grouper(HashMap<GroupKey, GroupCtx>);

struct GroupCtx {
    pub run_in_serial: bool,
    pub experimental_options: BTreeMap<&'static ExperimentalOption, bool>,
    pub environment_variables: BTreeMap<&'static str, &'static str>,
}

impl Display for GroupCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.run_in_serial {
            write!(f, "serial")?;
        }

        let mut experimental_options = self.experimental_options.iter();
        if let Some(first) = experimental_options.next() {
            if self.run_in_serial {
                write!(f, ", ")?;
            }

            write!(f, "exp[")?;
            write!(f, "{}={}", first.0.identifier(), first.1)?;
            for item in experimental_options {
                write!(f, ", {}={}", item.0.identifier(), item.1)?;
            }
            write!(f, "]")?;
        }

        let mut environment_variables = self.environment_variables.iter();
        if let Some(first) = environment_variables.next() {
            if self.run_in_serial || !self.experimental_options.is_empty() {
                write!(f, ", ")?;
            }

            write!(f, "env[")?;
            write!(f, "{}={}", first.0, first.1)?;
            for item in environment_variables {
                write!(f, ", {}={}", item.0, item.1)?;
            }
            write!(f, "]")?;
        }

        Ok(())
    }
}

impl TestGrouper<NuTestMetaExtra, GroupKey, GroupCtx> for Grouper {
    fn group(&mut self, meta: &TestMeta<NuTestMetaExtra>) -> GroupKey {
        let key = GroupKey::from(&meta.extra);
        if !self.0.contains_key(&key) {
            self.0.insert(
                key,
                GroupCtx {
                    experimental_options: meta
                        .extra
                        .experimental_options
                        .iter()
                        .map(|(option, value)| (*option, *value))
                        .collect(),
                    environment_variables: meta
                        .extra
                        .environment_variables
                        .iter()
                        .map(|(key, val)| (*key, *val))
                        .collect(),
                    run_in_serial: meta.extra.run_in_serial,
                },
            );
        }
        key
    }

    fn group_ctx(&mut self, key: &GroupKey) -> Option<GroupCtx> {
        self.0.remove(key)
    }
}

#[derive(Default)]
struct GroupRunner(SimpleGroupRunner);

impl<'t> TestGroupRunner<'t, NuTestMetaExtra, GroupKey, GroupCtx> for GroupRunner {
    fn run_group<F>(
        &self,
        f: F,
        key: &GroupKey,
        ctx: Option<&GroupCtx>,
    ) -> ControlFlow<TestGroupOutcomes<'t>, TestGroupOutcomes<'t>>
    where
        F: FnOnce() -> TestGroupOutcomes<'t>,
    {
        nu_experimental::ALL
            .iter()
            .for_each(|exp| unsafe { exp.unset() });
        if let Some(ctx) = ctx {
            ctx.experimental_options
                .iter()
                .for_each(|(exp, value)| unsafe { exp.set(*value) });
        }

        let old_envs: Vec<_> = ctx
            .iter()
            .map(|ctx| ctx.environment_variables.iter().map(|(key, _)| key))
            .flatten()
            .map(|key| (key, env::var_os(key)))
            .collect();
        ctx.iter()
            .map(|ctx| ctx.environment_variables.iter())
            .flatten()
            .for_each(|(key, value)| unsafe { env::set_var(key, value) });

        let run_test_group_in_serial = ctx.map(|ctx| ctx.run_in_serial).unwrap_or(false);
        RUN_TEST_GROUP_IN_SERIAL.store(run_test_group_in_serial, Ordering::Relaxed);

        let outcomes = <SimpleGroupRunner as TestGroupRunner<
            't,
            NuTestMetaExtra,
            GroupKey,
            GroupCtx,
        >>::run_group::<F>(&self.0, f, key, ctx);

        old_envs.into_iter().for_each(|(key, value)| unsafe {
            match value {
                Some(value) => env::set_var(key, value),
                None => env::remove_var(key),
            }
        });

        outcomes
    }
}

static RUN_TEST_GROUP_IN_SERIAL: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Default)]
struct NuTestRunner {
    parallel: DefaultRunner<DefaultPanicHookProvider, NoScopeFactory>,
    serial: SimpleRunner<DefaultPanicHookProvider, NoScopeFactory>,
    exact: bool,
}

impl NuTestRunner {
    fn with_thread_count(self, thread_count: NonZeroUsize) -> Self {
        Self {
            parallel: self.parallel.with_thread_count(thread_count),
            ..self
        }
    }

    fn with_exact(self, exact: bool) -> Self {
        Self { exact, ..self }
    }
}

enum NuTestRunnerIterator<IP, IS> {
    Parallel(IP),
    Serial(IS),
}

impl<'t, IP, IS> Iterator for NuTestRunnerIterator<IP, IS>
where
    IP: Iterator<Item = (&'t TestMeta<NuTestMetaExtra>, TestOutcome)>,
    IS: Iterator<Item = (&'t TestMeta<NuTestMetaExtra>, TestOutcome)>,
    NuTestMetaExtra: 't,
{
    type Item = (&'t TestMeta<NuTestMetaExtra>, TestOutcome);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Parallel(iter) => iter.next(),
            Self::Serial(iter) => iter.next(),
        }
    }
}

impl<'t> TestRunner<'t, NuTestMetaExtra> for NuTestRunner {
    fn run<'s, I, F>(
        &self,
        tests: I,
        scope: &'s Scope<'s, 't>,
    ) -> impl Iterator<Item = (&'t TestMeta<NuTestMetaExtra>, kitest::outcome::TestOutcome)>
    where
        I: ExactSizeIterator<Item = (F, &'t TestMeta<NuTestMetaExtra>)>,
        F: (Fn() -> kitest::outcome::TestStatus) + Send + 's,
        NuTestMetaExtra: 't,
    {
        match self.exact || RUN_TEST_GROUP_IN_SERIAL.load(Ordering::Relaxed) {
            false => {
                NuTestRunnerIterator::Parallel(<DefaultRunner<_, _> as TestRunner<
                    NuTestMetaExtra,
                >>::run(&self.parallel, tests, scope))
            }
            true => NuTestRunnerIterator::Serial(<SimpleRunner<_, _> as TestRunner<
                NuTestMetaExtra,
            >>::run(&self.serial, tests, scope)),
        }
    }

    fn worker_count(&self, tests_count: usize) -> NonZeroUsize {
        match RUN_TEST_GROUP_IN_SERIAL.load(Ordering::Relaxed) {
            true => const { NonZeroUsize::new(1).unwrap() },
            false => <DefaultRunner<_, _> as TestRunner<NuTestMetaExtra>>::worker_count(
                &self.parallel,
                tests_count,
            ),
        }
    }
}

#[derive(Debug)]
struct Args {
    color: ColorSetting,
    exact: bool,
    filter: Vec<String>,
    format: Format,
    ignored: bool,
    list: bool,
    no_capture: bool,
    skip: Vec<String>,
    test_threads: Option<NonZeroUsize>,
}

#[derive(Debug)]
enum Format {
    Pretty,
    Terse,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            color: ColorSetting::Automatic,
            exact: false,
            filter: Vec::new(),
            format: Format::Pretty,
            ignored: false,
            list: false,
            no_capture: false,
            skip: Vec::new(),
            test_threads: None,
        }
    }
}

impl Args {
    fn parse() -> Result<Args, lexopt::Error> {
        use lexopt::prelude::*;

        let mut args = Args::default();
        let mut parser = lexopt::Parser::from_env();

        fn parse_flag(parser: &mut lexopt::Parser, flag: &mut bool) -> Result<(), lexopt::Error> {
            Ok(match parser.optional_value() {
                None => *flag = true,
                Some(value) => *flag = value.parse()?,
            })
        }

        while let Some(arg) = parser.next()? {
            match arg {
                Long("color") => {
                    let color = parser.value()?.string()?;
                    match color.as_str() {
                        "auto" | "automatic" => args.color = ColorSetting::Automatic,
                        "always" => args.color = ColorSetting::Always,
                        "never" => args.color = ColorSetting::Never,
                        _ => todo!(),
                    }
                }
                Long("exact") => parse_flag(&mut parser, &mut args.exact)?,
                Value(value) => args.filter.push(value.parse()?),
                Long("format") => {
                    let color: String = parser.value()?.parse()?;
                    match color.as_str() {
                        "pretty" => args.format = Format::Pretty,
                        "terse" => args.format = Format::Terse,
                        _ => todo!(),
                    }
                }
                Long("ignored") => parse_flag(&mut parser, &mut args.ignored)?,
                Long("list") => parse_flag(&mut parser, &mut args.list)?,
                Long("nocapture" | "no-capture") => parse_flag(&mut parser, &mut args.no_capture)?,
                Long("skip") => args.skip.push(parser.value()?.parse()?),
                Long("test-threads") => args.test_threads = Some(parser.value()?.parse()?),
                arg => {
                    dbg!(arg);
                    todo!()
                }
            }
        }

        Ok(args)
    }
}

pub fn main() -> ExitCode {
    let args = Args::parse().unwrap(); // TODO: handle this better

    if args.no_capture {
        // TODO
    }

    let runner = NuTestRunner::default()
        .with_thread_count(args.test_threads.unwrap_or(*DEFAULT_THREAD_COUNT))
        .with_exact(args.exact);

    let filter = DefaultFilter::default()
        .with_exact(args.exact)
        .with_filter(args.filter)
        .with_skip(args.skip)
        .with_only_ignored(args.ignored);

    let harness = kitest::harness(TESTS.deref())
        .with_grouper(Grouper::default())
        .with_group_runner(GroupRunner::default())
        .with_groups(TestGroupBTreeMap::default())
        .with_runner(runner)
        .with_filter(filter);

    let pretty_formatter = PrettyFormatter::default()
        .with_color_setting(args.color)
        .with_group_label_from_ctx();
    let terse_formatter = TerseFormatter::default()
        .with_color_setting(args.color)
        .with_group_label_from_ctx();

    match (args.format, args.list) {
        (Format::Pretty, true) => harness.with_formatter(pretty_formatter).list().exit_code(),
        (Format::Pretty, false) => harness.with_formatter(pretty_formatter).run().exit_code(),
        (Format::Terse, true) => harness.with_formatter(terse_formatter).list().exit_code(),
        (Format::Terse, false) => harness.with_formatter(terse_formatter).run().exit_code(),
    }
}
