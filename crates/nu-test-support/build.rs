use heck::ToPascalCase;
use quote::{format_ident, quote};
use std::{collections::HashSet, env, fs, path::PathBuf, process::Command, sync::LazyLock};

fn main() {
    extract_build_profile();
    build_profile_enum();
}

static OUT_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(env::var("OUT_DIR").expect("set by cargo")));

fn extract_build_profile() {
    let mut components = OUT_DIR.components().rev();
    let _ = components.next().expect("");
    let _ = components.next().expect("");

    let build_profile = OUT_DIR
        .components()
        .rev()
        .skip(3)
        .next()
        .expect("unexpected OUT_DIR path format");
    let build_profile = build_profile.as_os_str().to_string_lossy();
    println!("cargo::rustc-env=BUILD_PROFILE={build_profile}");
}

fn build_profile_enum() {
    let crate_cargo_toml = include_str!("./Cargo.toml");
    let root_cargo_toml = include_str!("../../Cargo.toml");

    fn extract_profiles(cargo_toml: &str) -> Option<Vec<String>> {
        let cargo_toml: toml::Table = cargo_toml.parse().expect("valid toml");
        let profiles = cargo_toml.get("profile")?.as_table()?;
        Some(profiles.keys().cloned().collect())
    }

    let profiles = HashSet::<String>::from_iter(
        ["debug", "release"]
            .iter()
            .map(ToString::to_string)
            .chain(extract_profiles(crate_cargo_toml).unwrap_or_default())
            .chain(extract_profiles(root_cargo_toml).unwrap_or_default()),
    );

    let variants = profiles
        .iter()
        .map(|p| p.to_pascal_case())
        .map(|p| format_ident!("{p}"));

    let as_ref = profiles
        .iter()
        .map(|p| (p.to_pascal_case(), p))
        .map(|(variant, str)| (format_ident!("{variant}"), str))
        .map(|(variant, str)| quote!(Self::#variant => #str));

    let from_str = profiles
        .iter()
        .map(|p| (p, p.to_pascal_case()))
        .map(|(str, variant)| (str, format_ident!("{variant}")))
        .map(|(str, variant)| quote!(#str => Ok(Self::#variant)));

    let content = quote! {
        #[derive(Debug)]
        pub enum BuildProfile {
            #(#variants,)*
        }

        impl AsRef<str> for BuildProfile {
            fn as_ref(&self) -> &str {
                match self {
                    #(#as_ref,)*
                }
            }
        }

        pub struct UnknownBuildProfile(pub String);

        impl<'s> std::str::FromStr for BuildProfile {
            type Err = UnknownBuildProfile;

            fn from_str(s: &str) -> Result<Self, UnknownBuildProfile> {
                match s {
                    #(#from_str,)*
                    _ => Err(UnknownBuildProfile(s.to_string())),
                }
            }
        }
    };

    let out_file = OUT_DIR.join("build_profile.rs");
    fs::write(&out_file, content.to_string()).expect("could not write to out file");

    // doesn't really matter if formatting on that file fails
    let _ = Command::new("rustfmt").arg(out_file).status();
}
