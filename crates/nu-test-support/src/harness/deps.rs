use std::borrow::Cow;

#[derive(derive_more::Debug)]
pub struct Dependency<'a> {
    bin_name: Cow<'a, str>,
    #[debug("{:?}", format!("cargo build {build_args}"))]
    build_args: Cow<'a, str>,
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
            Dependency::const_new($dep, concat!("--package ", $dep));
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
    const fn const_new(bin_name: &'static str, build_args: &'static str) -> Self {
        Dependency {
            bin_name: Cow::Borrowed(bin_name),
            build_args: Cow::Borrowed(build_args),
            is_plugin: bin_name.len() >= "nu_plugin".len()
                && nu_utils::const_str::eq(bin_name.split_at("nu_plugin".len()).0, "nu_plugin"),
        }
    }
}

// pub const NU: &'static Dependency<'static> = &NU_DEP;
// static NU_DEP: Dependency<'static> = Dependency {
//     bin_name: Cow::Borrowed("nu"),
//     command: Cow::Borrowed(&[]),
//     is_plugin: "nu".len() >= "nu_plugin".len()
//         && "nu"[0.."nu_plugin".len()].as_bytes() == b"nu_plugin",
// };

// use std::{
//     io,
//     process::{Child, Command, Stdio},
// };

// #[derive(derive_more::Debug)]
// pub struct Dependency {
//     #[debug("{:?}", format!("cargo build {}", prebuild.build_args()))]
//     prebuild: &'static dyn Prebuild,
// }

// impl Dependency {
//     const fn new(prebuild: &'static dyn Prebuild) -> Self {
//         Dependency { prebuild }
//     }

//     pub fn prebuild(&self) -> io::Result<Child> {
//         Command::new("cargo")
//             .arg("build")
//             .args(self.prebuild.build_args().split(" "))
//             .stdout(Stdio::inherit())
//             .stderr(Stdio::inherit())
//             .spawn()
//     }
// }

// pub trait Binary: Sync {
//     fn bin_name(&self) -> &str;
//     fn build_args(&self) -> &str;
// }

// pub trait Prebuild: Sync {
//     fn build_args(&self) -> &str;
// }

// macro_rules! dependency {
//     ($static:ident, $build_args:literal) => {
//         pub use $static::$static;

//         #[allow(non_snake_case, reason = "makes macro simpler")]
//         mod $static {
//             /// Before executing this test, run
//             #[doc = concat!("`cargo build ", $build_args, "`.")]
//             pub const $static: super::Dependency = super::Dependency::new(&Prebuild);

//             struct Prebuild;
//             impl super::Prebuild for Prebuild {
//                 fn build_args(&self) -> &str {
//                     $build_args
//                 }
//             }
//         }
//     };
// }

// dependency!(NU, "--package nu --bin nu");
// dependency!(NU_PLUGIN_CUSTOM_VALUES, "--package nu_plugin_custom_values");
// dependency!(UTILS, "--package utils");
// dependency!(NU_PLUGIN_EXAMPLE, "--package nu_plugin_example");
// dependency!(NU_PLUGIN_FORMATS, "--package nu_plugin_formats");
// dependency!(NU_PLUGIN_GSTAT, "--package nu_plugin_gstat");
// dependency!(NU_PLUGIN_INC, "--package nu_plugin_inc");
// dependency!(NU_PLUGIN_POLARS, "--package nu_plugin_polars");
// dependency!(NU_PLUGIN_QUERY, "--package nu_plugin_query");
// dependency!(
//     NU_PLUGIN_STRESS_INTERNALS,
//     "--package nu_plugin_stress_internals"
// );
