//! Package management tools.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::OnceLock;

use parking_lot::Mutex;
use reflexo_typst::package::PackageSpec;
use reflexo_typst::typst::prelude::*;
use tinymist_world::https::HttpsRegistry;
use typst::diag::EcoString;

/// Get the packages in namespaces and their descriptions.
pub fn list_package_by_namespace(
    registry: &HttpsRegistry,
    ns: EcoString,
) -> EcoVec<(PathBuf, PackageSpec)> {
    // search packages locally. We only search in the data
    // directory and not the cache directory, because the latter is not
    // intended for storage of local packages.
    let mut packages = eco_vec![];

    log::info!(
        "searching for packages in namespace {ns} in paths {:?}",
        registry.paths()
    );
    for dir in registry.paths() {
        let local_path = dir.join(ns.as_str());
        if !local_path.exists() || !local_path.is_dir() {
            continue;
        }
        // namespace/package_name/version
        // 2. package_name
        let Some(package_names) = once_log(std::fs::read_dir(local_path), "read local pacakge")
        else {
            continue;
        };
        for package in package_names {
            let Some(package) = once_log(package, "read package name") else {
                continue;
            };
            if package.file_type().map_or(true, |ft| !ft.is_dir()) {
                continue;
            }
            if package.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            // 3. version
            let Some(versions) =
                once_log(std::fs::read_dir(package.path()), "read package versions")
            else {
                continue;
            };
            for version in versions {
                let Some(version) = once_log(version, "read package version") else {
                    continue;
                };
                if version.file_type().map_or(true, |ft| !ft.is_dir()) {
                    continue;
                }
                if version.file_name().to_string_lossy().starts_with('.') {
                    continue;
                }
                let path = version.path();
                let Some(version) = once_log(
                    version.file_name().to_string_lossy().parse(),
                    "parse package version",
                ) else {
                    continue;
                };
                let spec = PackageSpec {
                    namespace: ns.clone(),
                    name: package.file_name().to_string_lossy().into(),
                    version,
                };
                packages.push((path, spec));
            }
        }
    }

    packages
}

fn once_log<T, E: std::fmt::Display>(result: Result<T, E>, site: &'static str) -> Option<T> {
    let err = match result {
        Ok(value) => return Some(value),
        Err(err) => err,
    };

    static ONCES: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
    let mut onces = ONCES.get_or_init(Default::default).lock();
    if onces.insert(site) {
        log::error!("failed to perform {site}: {err}");
    }

    None
}
