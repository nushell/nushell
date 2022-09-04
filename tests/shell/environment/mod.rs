mod env;

// FIXME: nu_env tests depend on autoenv which hasn't been ported yet
// mod nu_env;

pub mod support {
    use nu_test_support::{nu, playground::*, Outcome};

    pub struct Trusted;

    impl Trusted {
        pub fn in_path(dirs: &Dirs, block: impl FnOnce() -> Outcome) -> Outcome {
            let for_env_manifest = dirs.test().to_string_lossy();

            nu!(cwd: dirs.root(), "autoenv trust \"{}\"", for_env_manifest);
            let out = block();
            nu!(cwd: dirs.root(), "autoenv untrust \"{}\"", for_env_manifest);

            out
        }
    }
}
