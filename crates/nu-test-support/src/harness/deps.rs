use std::{
    borrow::Cow,
    path::PathBuf,
    process::{Command, Stdio},
};

#[cfg(feature = "plugin")]
use nu_protocol::ShellError;

use crate::harness::{BUILD_PROFILE, TARGET_DIR};

#[non_exhaustive]
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

macro_rules! testbin_dependency {
    ($bin:literal) => { pastey::paste! {
        /// Test binary dependency
        #[doc = concat!("`", $bin, "`.")]
        ///
        /// Before executing this test, we automatically run
        #[doc = concat!("`cargo build --package testbins --bin ", $bin, "`.")]
        /// The binary will also automatically made available to use via
        /// [`test()`](crate::prelude::test).
        pub const [< TESTBIN_$bin:snake:upper>]: &'static Dependency<'static> = &[<TESTBIN_ $bin:snake:upper _DEP>];
        static [<TESTBIN_ $bin:snake:upper _DEP>]: Dependency<'static> =
            Dependency::new($bin, concat!("--package testbins --bin ", $bin));
    }}
}

testbin_dependency!("chop");
testbin_dependency!("cococo");
testbin_dependency!("echo_env");
testbin_dependency!("echo_env_mixed");
testbin_dependency!("echo_env_stderr");
testbin_dependency!("echo_env_stderr_fail");
testbin_dependency!("fail");
testbin_dependency!("iecho");
testbin_dependency!("input_bytes_length");
testbin_dependency!("meow");
testbin_dependency!("meowb");
testbin_dependency!("nonu");
testbin_dependency!("relay");
testbin_dependency!("repeat_bytes");
testbin_dependency!("repeater");

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

        if BUILD_PROFILE != "debug" {
            command.arg(format!("--profile={BUILD_PROFILE}"));
        }

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

    #[track_caller]
    pub fn path(&self) -> PathBuf {
        #[cfg(not(windows))]
        let bin_name = self.bin_name.as_ref();

        #[cfg(windows)]
        let bin_name = format!("{}.exe", self.bin_name.as_ref());

        TARGET_DIR
            .get()
            .expect("TARGET_DIR is not set")
            .join(BUILD_PROFILE)
            .join(bin_name)
    }

    #[cfg(feature = "plugin")]
    pub fn preload_plugin(&self) -> Result<PreloadedPlugin, ShellError> {
        use nu_plugin_engine::{GetPlugin, PersistentPlugin};
        use nu_protocol::{PluginIdentity, RegisteredPlugin};
        use std::sync::Arc;

        let filename = self.path();
        let identity = PluginIdentity::new(filename, None).expect("valid plugin name");
        let plugin = Arc::new(PersistentPlugin::new(identity.clone(), Default::default()));

        let interface = plugin.clone().get_plugin(None)?;
        let metadata = interface.get_metadata()?;
        plugin.set_metadata(Some(metadata.clone()));
        let signatures = Arc::from(interface.get_signature()?);
        drop(interface);

        Ok(PreloadedPlugin {
            identity: Arc::new(identity),
            plugin,
            metadata,
            signatures,
        })
    }
}

impl Ord for Dependency<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.bin_name.cmp(&other.bin_name)
    }
}

impl PartialOrd for Dependency<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(feature = "plugin")]
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct PreloadedPlugin {
    pub(crate) identity: std::sync::Arc<nu_protocol::PluginIdentity>,
    pub(crate) plugin: std::sync::Arc<nu_plugin_engine::PersistentPlugin>,
    pub(crate) metadata: nu_protocol::PluginMetadata,
    pub(crate) signatures: std::sync::Arc<[nu_protocol::PluginSignature]>,
}
