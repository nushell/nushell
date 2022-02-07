<<<<<<< HEAD
mod configuration;
mod env;
mod in_sync;
=======
mod env;
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
mod nu_env;

pub mod support {
    use nu_test_support::{nu, playground::*, Outcome};

    pub struct Trusted;

    impl Trusted {
        pub fn in_path(dirs: &Dirs, block: impl FnOnce() -> Outcome) -> Outcome {
            let for_env_manifest = dirs.test().to_string_lossy();

            nu!(cwd: dirs.root(), format!("autoenv trust \"{}\"", for_env_manifest));
            let out = block();
            nu!(cwd: dirs.root(), format!("autoenv untrust \"{}\"", for_env_manifest));

            out
        }
    }
}
