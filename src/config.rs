use std::path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_cache_dir", alias="cachedir", alias="cache", alias="Cache")]
    pub cache_dir: path::PathBuf,

    #[serde(default = "default_servers")]
    pub servers: std::vec::Vec<RemoteStorage>
}

#[derive(Debug, Deserialize)]
pub struct S3Config {
    pub bucket: String,

    #[serde(default)]
    pub prefix: String,

    #[serde(default)]
    pub endpoint: String
}

#[derive(Debug, Deserialize)]
pub struct HTTPConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub enum RemoteStorageAccess {
    #[serde(alias = "read")]
    Read,
    #[serde(alias = "readwrite", alias = "write", alias = "full")]
    ReadWrite
}

#[derive(Debug, Deserialize)]
pub struct RemoteStorage {
    pub access: RemoteStorageAccess,

    #[serde(flatten)]
    pub storage_type: RemoteStorageType
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum RemoteStorageType {
    #[serde(alias = "http")]
    HTTP(HTTPConfig),

    #[serde(alias = "s3")]
    S3(S3Config)
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cache_dir: default_cache_dir(),
            servers: default_servers()
        }
    }
}

pub fn read_config(path: &path::Path) -> std::io::Result<Config> {
    let content = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

fn default_cache_dir() -> path::PathBuf {
    let xdg_dirs = xdg::BaseDirectories::new().unwrap();
    xdg_dirs.get_cache_home()

}

fn default_servers() -> std::vec::Vec<RemoteStorage> {
    vec![
        RemoteStorage {
            access: RemoteStorageAccess::Read,
            storage_type: RemoteStorageType::HTTP(HTTPConfig {
            url: "https://debuginfod.elfutils.org/".to_string()
            })
        }
    ]
}