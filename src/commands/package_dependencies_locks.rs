use anyhow::Result;
use openfare_lib::extension::Extension;

pub fn package_dependencies_locks(
    extension: &crate::RsExtension,
    package_name: &str,
    package_version: &Option<&str>,
    _extension_args: &Vec<String>,
) -> Result<openfare_lib::extension::commands::package_dependencies_locks::PackageDependenciesLocks>
{
    let package_version = match package_version {
        Some(v) => v.to_string(),
        None => {
            log::debug!("No version argument given. Querying for latest version.");
            crate::registries::crates::get_latest_version(&package_name)?.ok_or(
                anyhow::format_err!("Failed to find latest version. Please specify version."),
            )?
        }
    };
    log::debug!("Found version: {}", package_version.to_string());

    let tmp_dir = tempdir::TempDir::new("openfare_rs")?;
    let tmp_dir = tmp_dir.path().to_path_buf();
    log::debug!("Using temporary directory: {}", tmp_dir.display());
    let package_directory = crate::registries::crates::setup_package_directory(
        &package_name,
        &package_version,
        &tmp_dir,
    )?;

    let package = crate::registries::crates::get_package(&package_name, &package_version);
    let lock = crate::registries::crates::get_lock(&package_directory)?;

    let mut dependencies_locks = dependencies_locks(&package_directory)?;
    dependencies_locks.remove(&package);

    Ok(
        openfare_lib::extension::commands::package_dependencies_locks::PackageDependenciesLocks {
            registry_host_name: extension
                .registries()
                .first()
                .ok_or(anyhow::format_err!(
                    "Code error: at least one registry host name expected."
                ))?
                .to_string(),
            package_locks: openfare_lib::package::PackageLocks {
                primary_package: Some(package),
                primary_package_lock: lock,
                dependencies_locks,
            },
        },
    )
}

fn dependencies_locks(
    package_directory: &std::path::PathBuf,
) -> Result<
    std::collections::BTreeMap<openfare_lib::package::Package, Option<openfare_lib::lock::Lock>>,
> {
    // Identify all dependency definition files.
    let dependency_files =
        match crate::registries::crates::identify_dependency_files(&package_directory) {
            Some(v) => v,
            None => {
                log::debug!("Did not identify any dependency definition files.");
                return Ok(std::collections::BTreeMap::<_, _>::new());
            }
        };
    let dependency_file = match dependency_files.first() {
        Some(f) => f,
        None => {
            log::debug!("Did not identify any dependency definition files.");
            return Ok(std::collections::BTreeMap::<_, _>::new());
        }
    };
    let dependencies_locks = crate::registries::crates::dependencies_locks(&dependency_file.path)?;
    Ok(dependencies_locks)
}
