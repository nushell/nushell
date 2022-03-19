#[cfg(windows)]
use nu_utils::utils::enable_vt_processing;

fn main() {
    // reset vt processing, aka ansi because illbehaved externals can break it
    #[cfg(windows)]
    {
        let _ = enable_vt_processing();
    }
}
