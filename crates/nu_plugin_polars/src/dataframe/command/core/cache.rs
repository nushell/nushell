use crate::lazy_command;

// LazyCache command
// Expands to a command definition for cache
lazy_command!(
    LazyCache,
    "polars cache",
    "Caches operations in a new LazyFrame.",
    vec![Example {
        description: "Caches the result into a new LazyFrame",
        example: "[[a b]; [6 2] [4 2] [2 2]] | polars into-df | polars reverse | polars cache",
        result: None,
    }],
    cache,
    test_cache
);
