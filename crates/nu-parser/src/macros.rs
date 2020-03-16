#[macro_export]
macro_rules! return_ok {
    ($expr:expr) => {
        match $expr {
            Ok(val) => return Ok(val),
            Err(_) => {}
        }
    };
}

#[cfg(test)]
macro_rules! equal_tokens {
    ($source:tt -> $tokens:expr) => {
        let result = apply(pipeline, "pipeline", $source);
        let (expected_tree, expected_source) = TokenTreeBuilder::build($tokens);

        if result != expected_tree {
            let debug_result = format!("{}", result.debug($source));
            let debug_expected = format!("{}", expected_tree.debug(&expected_source));

            if debug_result == debug_expected {
                assert_eq!(
                    result, expected_tree,
                    "NOTE: actual and expected had equivalent debug serializations, source={:?}, debug_expected={:?}",
                    $source,
                    debug_expected
                )
            } else {
                assert_eq!(debug_result, debug_expected)
            }
        }
    };

    (<$parser:tt> $source:tt -> $tokens:expr) => {
        let result = apply($parser, stringify!($parser), $source);

        let (expected_tree, expected_source) = TokenTreeBuilder::build($tokens);

        if result != expected_tree {
            let debug_result = format!("{}", result.debug($source));
            let debug_expected = format!("{}", expected_tree.debug(&expected_source));

            if debug_result == debug_expected {
                assert_eq!(
                    result, expected_tree,
                    "NOTE: actual and expected had equivalent debug serializations, source={:?}, debug_expected={:?}",
                    $source,
                    debug_expected
                )
            } else {
                assert_eq!(debug_result, debug_expected)
            }
        }
    };
}
