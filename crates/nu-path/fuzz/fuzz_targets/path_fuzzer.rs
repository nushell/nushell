#![no_main]

use libfuzzer_sys::fuzz_target;
use nu_path::{expand_path_with, expand_tilde, expand_to_real_path};

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let path = std::path::Path::new(s);

        // Fuzzing expand_to_real_path function
        let _ = expand_to_real_path(path);

        // Fuzzing expand_tilde function
        let _ = expand_tilde(path);

        // Fuzzing expand_path_with function
        // Here, we're assuming a second path for the "relative to" aspect.
        // For simplicity, we're just using the current directory.
        let current_dir = std::path::Path::new(".");
        let _ = expand_path_with(path, &current_dir, true);
    }
});
