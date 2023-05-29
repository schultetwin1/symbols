use std::path::{self, PathBuf};

use log::{/*error,*/ /*debug,*/ info, /* trace,*/ warn};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(
        default = "default_cache_dir",
        alias = "cachedir",
        alias = "cache",
        alias = "Cache"
    )]
    pub cache_dir: Option<path::PathBuf>,

    #[serde(default = "default_servers")]
    pub servers: std::vec::Vec<RemoteStorage>,
}

#[derive(Debug, Deserialize)]
pub struct S3Config {
    pub bucket: String,

    #[serde(default)]
    pub prefix: String,

    pub region: String,

    pub profile: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct B2Config {
    pub bucket: String,

    #[serde(default)]
    pub prefix: String,
    pub endpoint: String,
    pub account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SymbolServerConfig {
    pub project: String,
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HttpConfig {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct PathConfig {
    pub path: PathBuf,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub enum RemoteStorageAccess {
    #[serde(alias = "read")]
    Read,
    #[serde(alias = "readwrite", alias = "write", alias = "full")]
    ReadWrite,
}

#[derive(Debug, Deserialize)]
pub struct RemoteStorage {
    pub access: RemoteStorageAccess,

    pub name: Option<String>,

    #[serde(flatten)]
    pub storage_type: RemoteStorageType,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum RemoteStorageType {
    #[serde(alias = "http")]
    Http(HttpConfig),

    #[serde(alias = "s3")]
    S3(S3Config),

    #[serde(alias = "b2")]
    B2(B2Config),

    #[serde(alias = "symsrv")]
    SymbolServer(SymbolServerConfig),

    #[serde(alias = "path")]
    Path(PathConfig),
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cache_dir: default_cache_dir(),
            servers: default_servers(),
        }
    }
}

impl Config {
    pub fn init() -> std::io::Result<Self> {
        let mut config = Self::default();
        let mut default_config_path = None;

        if let Some(dirs) = directories::ProjectDirs::from("", "", "symbols") {
            default_config_path = Some(dirs.config_dir().join("symbols.toml"));
        } else {
            warn!("Unable to find OS config path");
        }

        if let Some(path) = default_config_path {
            if path.exists() {
                config = read_config(&path)?;
            } else {
                warn!("No config file found at '{}'", path.display());
            }
        }

        Ok(config)
    }

    pub fn from(path: &path::Path) -> std::io::Result<Self> {
        read_config(path)
    }
}

pub fn read_config(path: &path::Path) -> std::io::Result<Config> {
    info!("Reading config from {}", path.display());
    let content = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content).map_err(|error| {
        warn!("Failed to read config file {}. Error: {error:?}", path.display());
        std::io::Error::new(std::io::ErrorKind::InvalidData, "Malformed config toml")
    })?;
    Ok(config)
}

fn default_cache_dir() -> Option<path::PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "symbols")?;
    Some(dirs.cache_dir().to_owned())
}

fn default_servers() -> std::vec::Vec<RemoteStorage> {
    vec![RemoteStorage {
        access: RemoteStorageAccess::Read,
        name: None,
        storage_type: RemoteStorageType::Http(HttpConfig {
            url: "https://debuginfod.elfutils.org/".to_string(),
        }),
    }]
}
