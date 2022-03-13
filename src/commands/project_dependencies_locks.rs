use anyhow::{format_err, Result};
use openfare_lib::extension::commands::project_dependencies_locks::ProjectDependenciesLocks;

pub fn project_dependencies_locks(
    working_directory: &std::path::PathBuf,
    _extension_args: &Vec<String>,
) -> Result<ProjectDependenciesLocks> {
    // Identify all dependency definition files.
    let dependency_files =
        match crate::registries::crates::identify_dependency_files(&working_directory) {
            Some(v) => v,
            None => {
                log::debug!("Did not identify any dependency definition files.");
                return Ok(ProjectDependenciesLocks::default());
            }
        };
    let dependency_file = match dependency_files.first() {
        Some(f) => f,
        None => {
            log::debug!("Did not identify any dependency definition files.");
            return Ok(ProjectDependenciesLocks::default());
        }
    };

    log::debug!(
        "Found dependency definitions file: {}",
        dependency_file.path.display()
    );

    let project_path = dependency_file
        .path
        .parent()
        .ok_or(format_err!(
            "Failed to derive parent directory from dependency file path: {}",
            dependency_file.path.display()
        ))?
        .to_path_buf();

    let primary_package = crate::registries::crates::package_from_toml(&dependency_file.path)?;
    let primary_package_lock = crate::registries::crates::get_lock(&project_path)?;

    let dependencies_locks = crate::registries::crates::dependencies_locks(&dependency_file.path)?;

    Ok(ProjectDependenciesLocks {
        project_path: project_path.to_path_buf(),
        package_locks: openfare_lib::package::PackageLocks {
            primary_package,
            primary_package_lock,
            dependencies_locks,
        },
    })
}
