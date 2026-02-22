use itertools::Itertools;
use kitest::{
    group::{SimpleGroupRunner, TestGroupOutcomes, TestGroupRunner, TestGrouper},
    test::TestMeta,
};
use nu_experimental::ExperimentalOption;
use std::{
    collections::{BTreeMap, HashMap},
    env,
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
    ops::ControlFlow,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::harness::test::Extra;

pub static RUN_TEST_GROUP_IN_SERIAL: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupKey(u64);

impl From<&Extra> for GroupKey {
    fn from(extra: &Extra) -> Self {
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

pub struct GroupCtx {
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

#[derive(Default)]
pub struct Grouper(HashMap<GroupKey, GroupCtx>);

impl TestGrouper<Extra, GroupKey, GroupCtx> for Grouper {
    fn group(&mut self, meta: &TestMeta<Extra>) -> GroupKey {
        let key = GroupKey::from(&meta.extra);
        self.0.entry(key).or_insert_with(|| GroupCtx {
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
        });
        key
    }

    fn group_ctx(&mut self, key: &GroupKey) -> Option<GroupCtx> {
        self.0.remove(key)
    }
}

#[derive(Default)]
pub struct GroupRunner(SimpleGroupRunner);

impl<'t> TestGroupRunner<'t, Extra, GroupKey, GroupCtx> for GroupRunner {
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
            .flat_map(|ctx| ctx.environment_variables.keys())
            .map(|key| (key, env::var_os(key)))
            .collect();
        ctx.iter()
            .flat_map(|ctx| ctx.environment_variables.iter())
            .for_each(|(key, value)| unsafe { env::set_var(key, value) });

        let run_test_group_in_serial = ctx.map(|ctx| ctx.run_in_serial).unwrap_or(false);
        RUN_TEST_GROUP_IN_SERIAL.store(run_test_group_in_serial, Ordering::Relaxed);

        let outcomes =
            <SimpleGroupRunner as TestGroupRunner<'t, Extra, GroupKey, GroupCtx>>::run_group::<F>(
                &self.0, f, key, ctx,
            );

        old_envs.into_iter().for_each(|(key, value)| unsafe {
            match value {
                Some(value) => env::set_var(key, value),
                None => env::remove_var(key),
            }
        });

        outcomes
    }
}
