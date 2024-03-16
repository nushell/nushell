use nu_protocol::ShellError;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

/// Implements an atomically incrementing sequential series of numbers
#[derive(Debug, Default)]
pub struct Sequence(AtomicUsize);

impl Sequence {
    /// Return the next available id from a sequence, returning an error on overflow
    #[track_caller]
    pub(crate) fn next(&self) -> Result<usize, ShellError> {
        // It's totally safe to use Relaxed ordering here, as there aren't other memory operations
        // that depend on this value having been set for safety
        //
        // We're only not using `fetch_add` so that we can check for overflow, as wrapping with the
        // identifier would lead to a serious bug - however unlikely that is.
        self.0
            .fetch_update(Relaxed, Relaxed, |current| current.checked_add(1))
            .map_err(|_| ShellError::NushellFailedHelp {
                msg: "an accumulator for identifiers overflowed".into(),
                help: format!("see {}", std::panic::Location::caller()),
            })
    }
}

#[test]
fn output_is_sequential() {
    let sequence = Sequence::default();

    for (expected, generated) in (0..1000).zip(std::iter::repeat_with(|| sequence.next())) {
        assert_eq!(expected, generated.expect("error in sequence"));
    }
}

#[test]
fn output_is_unique_even_under_contention() {
    let sequence = Sequence::default();

    std::thread::scope(|scope| {
        // Spawn four threads, all advancing the sequence simultaneously
        let threads = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    (0..100000)
                        .map(|_| sequence.next())
                        .collect::<Result<Vec<_>, _>>()
                })
            })
            .collect::<Vec<_>>();

        // Collect all of the results into a single flat vec
        let mut results = threads
            .into_iter()
            .flat_map(|thread| thread.join().expect("panicked").expect("error"))
            .collect::<Vec<usize>>();

        // Check uniqueness
        results.sort();
        let initial_length = results.len();
        results.dedup();
        let deduplicated_length = results.len();
        assert_eq!(initial_length, deduplicated_length);
    })
}
