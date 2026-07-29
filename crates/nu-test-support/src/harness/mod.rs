use std::{
    collections::HashSet,
    env,
    fmt::Display,
    io,
    num::NonZeroUsize,
    ops::Deref,
    path::PathBuf,
    process::{Command, ExitCode, Stdio},
    sync::{LazyLock, OnceLock, atomic::Ordering},
};

use crate::{
    self as nu_test_support,
    harness::{
        args::{Args, Format},
        deps::*,
        group::{GroupRunner, Grouper},
        test::TestRunner,
    },
};

use kitest::{
    filter::{DefaultFilter, TestFilter},
    formatter::{pretty::PrettyFormatter, terse::TerseFormatter},
    group::TestGroupBTreeMap,
    ignore::DefaultIgnore,
};
use nu_ansi_term::Color;

#[doc(hidden)]
pub use linkme;

#[doc(hidden)]
pub use kitest::prelude::*;

mod args;
pub(crate) mod group;

pub(crate) mod test;
pub use test::{Extra, IntoTestResult};

pub mod deps;

pub mod macros {
    pub use kitest::{dbg, eprint, eprintln, print, println};
    pub use linkme::distributed_slice as collect_test;
    pub use nu_test_support_macros::test;
    pub use nu_utils::module_path_without_crate;
}

/// Environment variable that skips building required test dependency binaries when set.
///
/// Use this when the required binaries have already been built or are otherwise available at their
/// expected paths.
/// Tests that need missing dependency binaries may fail after skipping this step.
pub const SKIP_DEPS_BUILD_ENV: &str = "NU_TEST_SKIP_DEPS_BUILD";

pub const BUILD_PROFILE: &str = env!("BUILD_PROFILE");
static TARGET_DIR: OnceLock<PathBuf> = OnceLock::new();

pub const DEFAULT_THREAD_COUNT_MUL: NonZeroUsize = NonZeroUsize::new(4).unwrap();
pub static DEFAULT_THREAD_COUNT: LazyLock<NonZeroUsize> = LazyLock::new(|| {
    std::thread::available_parallelism()
        .map(|n| n.saturating_mul(DEFAULT_THREAD_COUNT_MUL))
        .unwrap_or(NonZeroUsize::MIN)
});

/// All collected tests.
#[linkme::distributed_slice]
#[linkme(crate = nu_test_support::harness::linkme)]
pub static TESTS: [kitest::test::Test<Extra>];

pub fn main() -> ExitCode {
    let args = match Args::parse() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{RedError}: {err}");
            eprintln!("help: use `--help` to see valid options");
            eprintln!();
            return ExitCode::FAILURE;
        }
    };

    if args.help {
        Args::help();
        return ExitCode::SUCCESS;
    }

    #[cfg(all(feature = "rustls-tls", feature = "network"))]
    nu_command::tls::CRYPTO_PROVIDER.default();

    // suppress direct output if `--no-capture` is *not* used
    #[cfg(feature = "plugin")]
    nu_plugin_core::SUPPRESS_STDERR.store(!args.no_capture, Ordering::Relaxed);
    nu_protocol::report_error::SUPPRESS_REPORTING.store(!args.no_capture, Ordering::Relaxed);
    kitest::capture::CAPTURE_OUTPUT_MACROS.store(!args.no_capture, Ordering::Relaxed);

    // do not allow updating the cwd via `EngineState::merge_env`
    nu_protocol::engine::UPDATE_CWD.store(false, Ordering::Relaxed);

    let filter = DefaultFilter::default()
        .with_exact(args.exact)
        .with_filter(args.filter)
        .with_skip(args.skip)
        .with_only_ignored(args.ignored);

    let dependencies: HashSet<&Dependency> = filter
        .filter(TESTS.deref())
        .tests
        .flat_map(|test| test.meta.extra.dependencies)
        .copied()
        .collect();

    #[allow(unused_variables, reason = "execution is non-pure")]
    let preparations = match args.list {
        true => TestPreparations::default(),
        false => match TestPreparations::prepare(dependencies.iter().copied()) {
            Ok(preparations) => preparations,
            Err(()) => return ExitCode::FAILURE,
        },
    };

    let runner = TestRunner::default()
        .with_thread_count(args.test_threads.unwrap_or(*DEFAULT_THREAD_COUNT))
        .with_exact(args.exact);

    let ignore = match args.include_ignored {
        false => DefaultIgnore::Default,
        true => DefaultIgnore::IncludeIgnored,
    };

    let group_runner = GroupRunner::default();
    #[cfg(feature = "plugin")]
    let group_runner = group_runner.with_preloaded_plugins(preparations.preloaded_plugins);

    let harness = kitest::harness(TESTS.deref())
        .with_grouper(Grouper::default())
        .with_group_runner(group_runner)
        .with_groups(TestGroupBTreeMap::default())
        .with_runner(runner)
        .with_filter(filter)
        .with_ignore(ignore);

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

#[derive(Debug, Default)]
struct TestPreparations {
    #[cfg(feature = "plugin")]
    preloaded_plugins: std::collections::HashMap<&'static Dependency<'static>, PreloadedPlugin>,
}

impl TestPreparations {
    fn prepare(
        dependencies: impl IntoIterator<
            IntoIter = impl ExactSizeIterator<Item = &'static Dependency<'static>>,
        >,
    ) -> Result<Self, ()> {
        #[cfg_attr(not(feature = "plugin"), expect(unused_mut))]
        let mut preparations = TestPreparations::default();
        let dependencies = dependencies.into_iter();
        if dependencies.len() == 0 {
            return Ok(preparations);
        }

        println!();
        println!("required cargo binaries: checking target dir");
        let target_dir = match target_dir() {
            Ok(target_dir) => target_dir,
            Err(err) => {
                eprintln!("{RedError}: {err}");
                eprintln!();
                return Err(());
            }
        };

        println!(
            "{} target dir is `{}`",
            Color::Green.bold().paint("    Finished"),
            target_dir.display()
        );
        TARGET_DIR
            .set(target_dir)
            .expect("TARGET_DIR is unset until now");

        for dependency in dependencies {
            println!();
            println!(
                "required binary `{}`: ensuring it is built",
                dependency.bin_name
            );

            if env::var_os(SKIP_DEPS_BUILD_ENV).is_some() {
                println!(
                    "{}: found `{}` being set, skipping build",
                    Color::Yellow.bold().paint("warning"),
                    SKIP_DEPS_BUILD_ENV,
                );
            } else {
                match env::var_os(format!("CARGO_BIN_EXE_{}", dependency.bin_name)) {
                    Some(path) if path == dependency.path() => {
                        println!(
                            "{} by cargo already",
                            Color::Green.bold().paint("    Prebuilt"),
                        );
                    }
                    Some(path) => {
                        eprintln!(
                            "{RedError}: unexpected path to binary `{}`, got `{}`",
                            dependency.bin_name,
                            path.display(),
                        );
                        eprintln!();
                        return Err(());
                    }
                    None => {
                        let mut child = match dependency.build_command().spawn() {
                            Ok(child) => child,
                            Err(err) => {
                                eprintln!("{RedError}: {err}");
                                eprintln!();
                                return Err(());
                            }
                        };

                        let exit_status = child.wait().expect("command wasn't running");
                        if !exit_status.success() {
                            eprintln!(
                                "{RedError}: compilation of dependency `{}` failed",
                                dependency.bin_name,
                            );
                            eprintln!();
                            return Err(());
                        }
                    }
                }
            }

            #[cfg(feature = "plugin")]
            if dependency.is_plugin {
                let preloaded_plugin = match dependency.preload_plugin() {
                    Ok(preloaded_plugin) => preloaded_plugin,
                    Err(err) => {
                        let err = err.to_string();
                        let mut err_chars = err.chars();
                        let first = err_chars.next().unwrap_or_default();
                        let rest = itertools::join(err_chars, "");
                        eprintln!(
                            "{RedError}: preloading `{}` failed, {first}{rest}",
                            dependency.bin_name
                        );
                        eprintln!();
                        return Err(());
                    }
                };

                println!(
                    "{} `{name}@{version}` with {sig_count} signatures",
                    Color::Green.bold().paint("   Preloaded"),
                    name = dependency.bin_name,
                    version = preloaded_plugin
                        .metadata
                        .version
                        .as_deref()
                        .unwrap_or("unknown"),
                    sig_count = preloaded_plugin.signatures.len(),
                );

                preparations
                    .preloaded_plugins
                    .insert(dependency, preloaded_plugin);
            }
        }

        Ok(preparations)
    }
}

fn target_dir() -> io::Result<PathBuf> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .spawn()?
        .wait_with_output()?;

    if !output.status.success() {
        return Err(io::Error::other(
            "`cargo metadata` did not run successfully",
        ));
    }

    let metadata: serde_json::Value = serde_json::from_slice(output.stdout.as_slice())?;
    let target_dir = metadata["target_directory"]
        .as_str()
        .expect("target_directory is a string");
    Ok(PathBuf::from(target_dir))
}

struct RedError;

impl Display for RedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Color::Red.bold().paint("error").fmt(f)
    }
}
