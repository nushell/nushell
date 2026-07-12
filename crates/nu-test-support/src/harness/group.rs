use core::fmt;
use itertools::Itertools;
use kitest::{
    group::{SimpleGroupRunner, TestGroupOutcomes, TestGroupRunner, TestGrouper},
    test::TestMeta,
};
use nu_experimental::ExperimentalOption;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    env,
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
    ops::ControlFlow,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

use crate::{
    harness::{
        deps::{Dependency, PreloadedPlugin},
        test::Extra,
    },
    tester::{GLOBAL_PATH_ENV_AUTO_LOAD, GLOBAL_PLUGIN_AUTO_LOAD, PluginAutoLoader},
};

pub static RUN_TEST_GROUP_IN_SERIAL: AtomicBool = AtomicBool::new(false);
static CURRENT_GROUP_KEY: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct GroupKey(u64);

impl GroupKey {
    /// Load the current group key.
    pub fn current() -> GroupKey {
        let current_value = CURRENT_GROUP_KEY.load(Ordering::Relaxed);
        GroupKey(current_value)
    }
}

impl From<&Extra> for GroupKey {
    fn from(extra: &Extra) -> Self {
        if extra.experimental_options.is_empty()
            && extra.environment_variables.is_empty()
            && !extra.run_in_serial
            && extra.dependencies.is_empty()
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
        extra
            .dependencies
            .iter()
            .map(|dep| dep.bin_name.as_ref())
            .sorted()
            .for_each(|item| item.hash(&mut hasher));
        GroupKey(hasher.finish())
    }
}

#[derive(Debug)]
pub struct GroupCtx {
    pub run_in_serial: bool,
    pub experimental_options: BTreeMap<&'static ExperimentalOption, bool>,
    pub environment_variables: BTreeMap<&'static str, &'static str>,
    pub dependencies: BTreeSet<&'static Dependency<'static>>,
}

impl Display for GroupCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let run_in_serial = self
            .run_in_serial
            .then(|| fmt::from_fn(|f| write!(f, "serial")));

        let experimental_options = self.experimental_options.iter().next().map(|(key, val)| {
            fmt::from_fn(move |f| {
                write!(f, "exp[")?;
                write!(f, "{}={val}", key.identifier())?;
                for (key, val) in self.experimental_options.iter().skip(1) {
                    write!(f, ", {}={val}", key.identifier())?;
                }
                write!(f, "]")
            })
        });

        let environment_variables = self.environment_variables.iter().next().map(|(key, val)| {
            fmt::from_fn(move |f| {
                write!(f, "env[")?;
                write!(f, "{key}={val}")?;
                for (key, val) in self.environment_variables.iter().skip(1) {
                    write!(f, ", {key}={val}")?;
                }
                write!(f, "]")
            })
        });

        let dependencies = self.dependencies.iter().next().map(|first| {
            fmt::from_fn(move |f| {
                write!(f, "deps[")?;
                write!(f, "{}", first.bin_name)?;
                for item in self.dependencies.iter().skip(1) {
                    write!(f, ", {}", item.bin_name)?;
                }
                write!(f, "]")
            })
        });

        fn make_dyn(item: &Option<impl Display>) -> Option<&dyn Display> {
            item.as_ref().map(|item| item as &dyn Display)
        }

        let () = itertools::intersperse(
            [
                make_dyn(&run_in_serial),
                make_dyn(&experimental_options),
                make_dyn(&environment_variables),
                make_dyn(&dependencies),
            ]
            .into_iter()
            .flat_map(|item| item),
            &fmt::from_fn(|f| write!(f, ", ")) as &dyn Display,
        )
        .try_for_each(|item| write!(f, "{item}"))?;

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct Grouper(HashMap<GroupKey, GroupCtx>);

impl TestGrouper<Extra, GroupKey, GroupCtx> for Grouper {
    fn group(&mut self, meta: &TestMeta<Extra>) -> GroupKey {
        let key = GroupKey::from(&meta.extra);
        self.0.entry(key).or_insert_with(|| GroupCtx {
            run_in_serial: meta.extra.run_in_serial,
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
            dependencies: meta.extra.dependencies.iter().copied().collect(),
        });
        key
    }

    fn group_ctx(&mut self, key: &GroupKey) -> Option<GroupCtx> {
        self.0.remove(key)
    }
}

#[derive(Debug, Default)]
pub struct GroupRunner {
    runner: SimpleGroupRunner,

    #[cfg(feature = "plugin")]
    preloaded_plugins: HashMap<&'static Dependency<'static>, PreloadedPlugin>,
}

impl GroupRunner {
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
        CURRENT_GROUP_KEY.store(key.0, Ordering::Relaxed);

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

        {
            // Load this inside a block to ensure the guard is dropped
            let mut paths = GLOBAL_PATH_ENV_AUTO_LOAD.write();
            paths.clear();
            if let Some(ctx) = ctx {
                paths.extend(ctx.dependencies.iter().map(|dep| {
                    dep.path()
                        .parent()
                        .expect("bin lives in target dir")
                        .to_path_buf()
                }));
            }
        }

        #[cfg(feature = "plugin")]
        {
            // Load this inside a block to ensure the guard is dropped
            let mut auto_loaders = GLOBAL_PLUGIN_AUTO_LOAD.write();
            auto_loaders.clear();
            if let Some(ctx) = ctx {
                auto_loaders.extend(
                    ctx.dependencies
                        .iter()
                        .filter(|dep| dep.is_plugin)
                        .flat_map(|dep| self.preloaded_plugins.get(dep))
                        .map(|plugin| PluginAutoLoader {
                            identity: plugin.identity.clone(),
                            plugin: Some(plugin.plugin.clone()),
                            signatures: Some(plugin.signatures.clone()),
                        }),
                );
            }
        }

        let run_test_group_in_serial = ctx.map(|ctx| ctx.run_in_serial).unwrap_or(false);
        RUN_TEST_GROUP_IN_SERIAL.store(run_test_group_in_serial, Ordering::Relaxed);

        let outcomes =
            <SimpleGroupRunner as TestGroupRunner<'t, Extra, GroupKey, GroupCtx>>::run_group::<F>(
                &self.runner,
                f,
                key,
                ctx,
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
