use anyhow::Result;

mod commands;
mod common;
mod registries;

#[derive(Clone, Debug)]
pub struct RsExtension {
    name_: String,
    registry_host_names_: Vec<String>,
    version_: String,
}

impl openfare_lib::extension::FromLib for RsExtension {
    fn new() -> Self {
        Self {
            name_: "rs".to_string(),
            registry_host_names_: registries::HOST_NAMES
                .to_vec()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            version_: format!("CARGO_PKG_VERSION: {}", env!("CARGO_PKG_VERSION"),),
        }
    }
}

impl openfare_lib::extension::Extension for RsExtension {
    fn name(&self) -> String {
        self.name_.clone()
    }

    fn registries(&self) -> Vec<String> {
        self.registry_host_names_.clone()
    }

    fn version(&self) -> String {
        self.version_.clone()
    }

    fn package_dependencies_locks(
        &self,
        package_name: &str,
        package_version: &Option<&str>,
        extension_args: &Vec<String>,
    ) -> Result<
        openfare_lib::extension::commands::package_dependencies_locks::PackageDependenciesLocks,
    > {
        commands::package_dependencies_locks(
            &self,
            &package_name,
            &package_version,
            &extension_args,
        )
    }

    fn project_dependencies_locks(
        &self,
        working_directory: &std::path::PathBuf,
        extension_args: &Vec<String>,
    ) -> Result<
        openfare_lib::extension::commands::project_dependencies_locks::ProjectDependenciesLocks,
    > {
        commands::project_dependencies_locks(&working_directory, &extension_args)
    }
}
