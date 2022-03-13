use anyhow::{Context, Result};
use std::io::Read;
use strum::IntoEnumIterator;

pub const HOST_NAME: &'static str = "crates.io";

/// Package dependency file types.
#[derive(Debug, Copy, Clone, strum_macros::EnumIter)]
pub enum DependencyFileType {
    CargoToml,
}

impl DependencyFileType {
    /// Return file name associated with dependency type.
    pub fn file_name(&self) -> std::path::PathBuf {
        match self {
            Self::CargoToml => std::path::PathBuf::from("Cargo.toml"),
        }
    }
}

/// Package dependency file type and file path.
#[derive(Debug, Clone)]
pub struct DependencyFile {
    pub r#type: DependencyFileType,
    pub path: std::path::PathBuf,
}

/// Returns a vector of identified package dependency definition files.
///
/// Walks up the directory tree directory tree until the first positive result is found.
pub fn identify_dependency_files(
    working_directory: &std::path::PathBuf,
) -> Option<Vec<DependencyFile>> {
    assert!(working_directory.is_absolute());
    let mut working_directory = working_directory.clone();

    loop {
        // If at least one target is found, assume package is present.
        let mut found_dependency_file = false;

        let mut dependency_files: Vec<DependencyFile> = Vec::new();
        for dependency_file_type in DependencyFileType::iter() {
            let target_absolute_path = working_directory.join(dependency_file_type.file_name());
            if target_absolute_path.is_file() {
                found_dependency_file = true;
                dependency_files.push(DependencyFile {
                    r#type: dependency_file_type,
                    path: target_absolute_path,
                })
            }
        }
        if found_dependency_file {
            return Some(dependency_files);
        }

        // No need to move further up the directory tree after this loop.
        if working_directory == std::path::PathBuf::from("/") {
            break;
        }

        // Move further up the directory tree.
        working_directory.pop();
    }
    None
}

/// Given package name, return latest version.
pub fn get_latest_version(package_name: &str) -> Result<Option<String>> {
    let json = get_registry_entry_json(&package_name)?;
    let latest_version = json["crate"]["newest_version"]
        .as_str()
        .and_then(|v| Some(v.to_string()));
    Ok(latest_version)
}

fn get_registry_entry_json(package_name: &str) -> Result<serde_json::Value> {
    let handlebars_registry = handlebars::Handlebars::new();
    let json_url = handlebars_registry.render_template(
        "https://crates.io/api/v1/crates/{{package_name}}",
        &maplit::btreemap! {"package_name" => package_name},
    )?;

    let client = reqwest::blocking::Client::builder()
        .user_agent(crate::common::HTTP_USER_AGENT)
        .build()?;
    let mut result = client.get(&json_url.to_string()).send()?;

    let mut body = String::new();
    result.read_to_string(&mut body)?;

    Ok(serde_json::from_str(&body).context(format!("JSON was not well-formatted:\n{}", body))?)
}

pub fn setup_package_directory(
    package_name: &str,
    package_version: &str,
    root_directory: &std::path::PathBuf,
) -> Result<std::path::PathBuf> {
    let url = crate_download_url(&package_name, &package_version)?;
    let archive_path = root_directory.join("archive");
    openfare_lib::common::fs::archive::download(&url, &archive_path)?;

    let crate_directory = root_directory.join("crate");
    let crate_directory =
        openfare_lib::common::fs::archive::extract_tar_gz(&archive_path, &crate_directory)?;
    Ok(crate_directory)
}

fn crate_download_url(package_name: &str, package_version: &str) -> Result<url::Url> {
    let handlebars_registry = handlebars::Handlebars::new();
    let url = handlebars_registry.render_template(
        "https://crates.io/api/v1/crates/{{package_name}}/{{package_version}}/download",
        &maplit::btreemap! {"package_name" => package_name, "package_version" => package_version},
    )?;
    Ok(url::Url::parse(&url)?)
}

pub fn get_lock(
    package_directory: &std::path::PathBuf,
) -> Result<Option<openfare_lib::lock::Lock>> {
    let openfare_json_path = package_directory.join(openfare_lib::lock::FILE_NAME);
    let lock = if openfare_json_path.is_file() {
        Some(parse_lock_file(&openfare_json_path)?)
    } else {
        None
    };
    Ok(lock)
}

pub fn package_from_toml(
    cargo_toml_path: &std::path::PathBuf,
) -> Result<Option<openfare_lib::package::Package>> {
    let contents = std::fs::read_to_string(&cargo_toml_path)?;

    let manifest_toml: toml::Value = toml::from_str(&contents)?;
    let name = manifest_toml["package"]["name"]
        .as_str()
        .ok_or(anyhow::format_err!(
            "Failed to find field 'package.version'."
        ))?;
    let version = manifest_toml["package"]["version"]
        .as_str()
        .ok_or(anyhow::format_err!(
            "Failed to find field 'package.version'."
        ))?;
    Ok(Some(openfare_lib::package::Package {
        registry: HOST_NAME.to_string(),
        name: name.to_string(),
        version: version.to_string(),
    }))
}

pub fn get_package(package_name: &str, package_version: &str) -> openfare_lib::package::Package {
    openfare_lib::package::Package {
        name: package_name.to_string(),
        version: package_version.to_string(),
        registry: HOST_NAME.to_string(),
    }
}

fn parse_lock_file(path: &std::path::PathBuf) -> Result<openfare_lib::lock::Lock> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let lock: openfare_lib::lock::Lock = serde_json::from_reader(reader).context(format!(
        "Failed to parse {lock_file_name}: {path}",
        lock_file_name = openfare_lib::lock::FILE_NAME,
        path = path.display()
    ))?;
    Ok(lock)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Metadata {
    pub packages: Vec<Package>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Package {
    pub name: String,
    pub version: String,
    pub manifest_path: std::path::PathBuf,
}

pub fn dependencies_locks(
    cargo_toml_path: &std::path::PathBuf,
) -> Result<
    std::collections::BTreeMap<openfare_lib::package::Package, Option<openfare_lib::lock::Lock>>,
> {
    let config = cargo::util::config::Config::default()?;
    let workspace = cargo::core::Workspace::new(&cargo_toml_path, &config)?;
    let options = cargo::ops::OutputMetadataOptions {
        cli_features: cargo::core::resolver::features::CliFeatures::new_all(false),
        no_deps: false,
        version: 1,
        filter_platforms: vec![],
    };

    let metadata = cargo::ops::output_metadata(&workspace, &options)?;
    let metadata = serde_json::to_string_pretty(&metadata)?;
    let metadata: Metadata = serde_json::from_str(&metadata)?;

    let mut results = maplit::btreemap! {};
    for metadata_package in metadata.packages {
        let package = openfare_lib::package::Package {
            registry: HOST_NAME.to_string(),
            name: metadata_package.name.clone(),
            version: metadata_package.version.clone(),
        };
        let lock = {
            if let Some(package_directory) = metadata_package.manifest_path.parent() {
                get_lock(&package_directory.to_path_buf())?
            } else {
                None
            }
        };
        results.insert(package, lock);
    }
    Ok(results)
}
