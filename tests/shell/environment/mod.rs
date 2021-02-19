mod nu_env;

pub mod support {
    use nu_test_support::{nu, playground::*, Outcome};

    pub struct Trusted;

    impl Trusted {
        pub fn in_path(dirs: &Dirs, block: impl FnOnce() -> Outcome) -> Outcome {
            let for_env_manifest = dirs.test().to_string_lossy();

            nu!(cwd: dirs.root(), format!("autoenv trust \"{}\"", for_env_manifest.to_string()));
            let out = block();
            nu!(cwd: dirs.root(), format!("autoenv untrust \"{}\"", for_env_manifest.to_string()));

            out
        }
    }
}
