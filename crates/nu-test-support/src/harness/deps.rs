use std::{
    borrow::Cow,
    process::{Command, Stdio},
};

#[derive(derive_more::Debug, Clone, PartialEq, Eq, Hash)]
pub struct Dependency<'a> {
    /// Name of the binary without extension.
    pub bin_name: Cow<'a, str>,

    /// Args to build the binary.
    ///
    /// Do not include `cargo build` for example.
    #[debug("{:?}", format!("cargo build {build_args}"))]
    build_args: Cow<'a, str>,

    /// Whether the binary is a plugin.
    ///
    /// Plugins get automatically loaded.
    pub is_plugin: bool,
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
        // TODO: handle build profiles

        let mut command = Command::new("cargo");
        command
            .arg("build")
            // .arg("-vv")
            .args(self.build_args.split(" "))
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        // ensure that cargo is called cleanly to avoid unnecessary rebuilds
        for (key, _) in std::env::vars() {
            #[rustfmt::skip]
            match key.as_ref() {
                "CARGO"
                | "CARGO_MANIFEST_DIR"
                | "CARGO_MANIFEST_PATH"
                | "CARGO_MANIFEST_LINKS"
                | "CARGO_CRATE_NAME"
                | "CARGO_BIN_NAME"
                | "OUT_DIR"
                | "PROFILE"
                | "OPT_LEVEL"
                | "DEBUG"
                | "HOST"
                | "TARGET" => command.env_remove(key),

                key if key.starts_with("CARGO_PKG_")
                    || key.starts_with("CARGO_CFG_")
                    || key.starts_with("CARGO_FEATURE_")
                    || key.starts_with("CARGO_BIN_EXE_")
                    || key.starts_with("DEP_") => command.env_remove(key),

                _ => &mut command,
            };
        }

        command
    }
}
