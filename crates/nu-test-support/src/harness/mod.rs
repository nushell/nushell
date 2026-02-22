use std::{
    num::NonZeroUsize,
    ops::Deref,
    process::ExitCode,
    sync::{LazyLock, atomic::Ordering},
};

use crate::{
    self as nu_test_support,
    harness::{
        args::{Args, Format},
        group::{GroupRunner, Grouper},
        test::TestRunner,
    },
};

use kitest::{
    filter::DefaultFilter,
    formatter::{pretty::PrettyFormatter, terse::TerseFormatter},
    group::TestGroupBTreeMap,
};
use nu_ansi_term::Color;

#[doc(hidden)]
pub use linkme;

#[doc(hidden)]
pub use kitest::prelude::*;

mod args;
mod group;
mod test;

pub use test::Extra;

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
pub static TESTS: [kitest::test::Test<Extra>];

pub fn main() -> ExitCode {
    let args = match Args::parse() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{}: {err}", Color::Red.bold().paint("error"));
            eprintln!("help: use `--help` to see valid options");
            eprintln!();
            return ExitCode::FAILURE;
        }
    };

    if args.help {
        Args::help();
        return ExitCode::SUCCESS;
    }

    if args.no_capture {
        kitest::capture::CAPTURE_OUTPUT_MACROS.store(false, Ordering::Relaxed);
    }

    let runner = TestRunner::default()
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
