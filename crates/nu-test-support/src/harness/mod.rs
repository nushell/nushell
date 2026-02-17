use std::{
    collections::{BTreeMap, HashMap},
    env,
    fmt::{Debug, Display},
    hash::{DefaultHasher, Hash, Hasher},
    num::NonZeroUsize,
    ops::{ControlFlow, Deref},
    process::Termination,
    sync::LazyLock,
};

use crate::{self as nu_test_support};

use itertools::Itertools;
use kitest::{
    formatter::pretty::PrettyFormatter,
    group::{
        SimpleGroupRunner, TestGroupBTreeMap, TestGroupOutcomes, TestGroupRunner, TestGrouper,
    },
    runner::DefaultRunner,
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
pub static TESTS: [kitest::test::Test<TestMetaExtra>];

pub struct TestMetaExtra {
    pub experimental_options: &'static [(&'static ExperimentalOption, bool)],
    pub environment_variables: &'static [(&'static str, &'static str)],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct GroupKey(u64);

impl From<&TestMetaExtra> for GroupKey {
    fn from(extra: &TestMetaExtra) -> Self {
        if extra.experimental_options.is_empty() && extra.environment_variables.is_empty() {
            return Self(0);
        }

        let mut hasher = DefaultHasher::new();
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
    pub experimental_options: BTreeMap<&'static ExperimentalOption, bool>,
    pub environment_variables: BTreeMap<&'static str, &'static str>,
}

impl Display for GroupCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut experimental_options = self.experimental_options.iter();
        if let Some(first) = experimental_options.next() {
            write!(f, "exp[")?;
            write!(f, "{}={}", first.0.identifier(), first.1)?;
            for item in experimental_options {
                write!(f, ", {}={}", item.0.identifier(), item.1)?;
            }
            write!(f, "]")?;
        }

        let mut environment_variables = self.environment_variables.iter();
        if let Some(first) = environment_variables.next() {
            if self.experimental_options.is_empty() {
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

impl TestGrouper<TestMetaExtra, GroupKey, GroupCtx> for Grouper {
    fn group(&mut self, meta: &TestMeta<TestMetaExtra>) -> GroupKey {
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
                },
            );
        }
        key
    }

    fn group_ctx(&self, key: &GroupKey) -> Option<&GroupCtx> {
        self.0.get(key)
    }
}

#[derive(Default)]
struct GroupRunner(SimpleGroupRunner);

impl<'t> TestGroupRunner<'t, TestMetaExtra, GroupKey, GroupCtx> for GroupRunner {
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

        let outcomes = <SimpleGroupRunner as TestGroupRunner<
            't,
            TestMetaExtra,
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

pub fn main() -> impl Termination {
    kitest::harness(TESTS.deref())
        .with_grouper(Grouper::default())
        .with_formatter(PrettyFormatter::default().with_group_label_from_ctx())
        .with_group_runner(GroupRunner::default())
        .with_groups(TestGroupBTreeMap::default())
        .with_runner(DefaultRunner::default().with_thread_count(*DEFAULT_THREAD_COUNT))
        .run()
}
