use serde::Deserialize;
use update_informer::{
    http_client::{GenericHttpClient, HttpClient},
    registry, Check, Package, Registry, Result,
};

pub struct NuShellNightly;

impl Registry for NuShellNightly {
    const NAME: &'static str = "nushell/nightly";

    fn get_latest_version<T: HttpClient>(
        http_client: GenericHttpClient<T>,
        pkg: &Package,
    ) -> Result<Option<String>> {
        #[derive(Deserialize, Debug)]
        struct Response {
            tag_name: String,
        }

        let url = format!("https://api.github.com/repos/{}/releases", pkg);
        let versions = http_client
            .add_header("Accept", "application/vnd.github.v3+json")
            .add_header("User-Agent", "update-informer")
            .get::<Vec<Response>>(&url)?;

        if let Some(v) = versions.first() {
            // The nightly repo tags look like "0.101.1-nightly.4+23dc1b6"
            // We want to return the "0.101.1-nightly.4" part because hustcer
            // is changing the cargo.toml package.version to be that syntax
            let up_through_plus = v.tag_name.split('+').next().unwrap();
            return Ok(Some(up_through_plus.to_string()));
        }

        Ok(None)
    }
}

pub fn check_for_latest_nushell_version() {
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    if current_version.contains("nightly") {
        let nightly_pkg_name = "nushell/nightly";
        let informer =
            update_informer::new(NuShellNightly, nightly_pkg_name, current_version.clone())
                .interval(std::time::Duration::ZERO);

        if let Ok(Some(new_version)) = informer.check_version() {
            println!(
                "\nA new release of nushell nightly is available: {current_version} -> {new_version}\n"
            );
        } else {
            println!(
                "You're running the latest version of the nushell nightly v{current_version}."
            );
        }
    } else {
        let normal_pkg_name = "nushell/nushell";
        let informer =
            update_informer::new(registry::GitHub, normal_pkg_name, current_version.clone())
                .interval(std::time::Duration::ZERO);

        if let Ok(Some(new_version)) = informer.check_version() {
            println!(
                "\nA new release of nushell is available: v{current_version} -> {new_version}\n"
            );
        } else {
            println!(
                "You're running the latest version of the nushell release v{current_version}."
            );
        }
    }
}
