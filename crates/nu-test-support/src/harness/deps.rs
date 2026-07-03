use std::{
    borrow::Cow,
    process::{Command, Stdio},
};

#[derive(derive_more::Debug, Clone, PartialEq, Eq, Hash)]
pub struct Dependency<'a> {
    /// Name of the binary without extension.
    bin_name: Cow<'a, str>,

    /// Args to build the binary.
    /// 
    /// Do not include `cargo build` for example.
    #[debug("{:?}", format!("cargo build {build_args}"))]
    build_args: Cow<'a, str>,

    /// Whether the binary is a plugin.
    /// 
    /// Plugins get automatically loaded.
    is_plugin: bool,
}

macro_rules! dependency {
    ($dep:literal) => { pastey::paste! {
        /// Binary dependency on
        #[doc = concat!("`", $dep, "`.")]
        ///
        /// Before executing this test, we automatically run
        #[doc = concat!("`cargo build --package ", $dep, "`.")]
        /// The binary will also automatically made available to use via
        /// [`test()`](crate::prelude::test).
        pub const [<$dep:snake:upper>]: &'static Dependency<'static> = &[<$dep:snake:upper _DEP>];
        static [<$dep:snake:upper _DEP>]: Dependency<'static> =
            Dependency::new($dep, concat!("--package ", $dep));
    }}
}

dependency!("nu");
dependency!("nu_plugin_custom_values");
dependency!("nu_plugin_example");
dependency!("nu_plugin_formats");
dependency!("nu_plugin_gstat");
dependency!("nu_plugin_inc");
dependency!("nu_plugin_polars");
dependency!("nu_plugin_query");
dependency!("nu_plugin_stress_internals");

impl Dependency<'static> {
    const fn new(bin_name: &'static str, build_args: &'static str) -> Self {
        Dependency {
            bin_name: Cow::Borrowed(bin_name),
            build_args: Cow::Borrowed(build_args),
            is_plugin: bin_name.len() >= "nu_plugin".len()
                && nu_utils::const_str::eq(bin_name.split_at("nu_plugin".len()).0, "nu_plugin"),
        }
    }

    pub fn build_command(&self) -> Command {
        let mut command = Command::new("cargo");
        command
            .arg("build")
            .args(self.build_args.split(" "))
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        command
    }
}
