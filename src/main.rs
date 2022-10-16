use anyhow::{anyhow, bail, Context, Result};
use log::{/*error,*/ /*debug,*/ info, trace /*warn */};

use std::{path::Path, path::PathBuf};

use crate::config::{PathConfig, RemoteStorage, RemoteStorageType, S3Config};

mod args;
mod config;
mod login;
mod symstore;
mod upload;

fn main() -> Result<()> {
    let matches = args::parse_args();
    initialize_logger(&matches);
    trace!("logger initialized");

    let config = if let Some(path) = matches.value_of(args::CONFIG_FILE_ARG) {
        let path = PathBuf::from(path);
        config::Config::from(&path)
            .context(format!("Failed to read config from '{}'", path.display()))?
    } else {
        config::Config::init().context("Failed to read default config")?
    };

    if let Some(matches) = matches.subcommand_matches(args::UPLOAD_SUBCOMMAND) {
        info!("Upload subcommand");
        let search_path = Path::new(
            matches
                .value_of(args::UPLOAD_PATH_ARG)
                .context("Unable to find upload path argument")?,
        );
        let recursive_search = matches.is_present(args::UPLOAD_RECUSRIVE_ARG);
        let dryrun = matches.is_present(args::UPLOAD_DRY_RUN_ARG);
        let mut writable_servers = config
            .servers
            .into_iter()
            .filter(|server| server.access == config::RemoteStorageAccess::ReadWrite);
        let server = if let Some(bucket) = matches.value_of(args::UPLOAD_S3_BUCKET_ARG) {
            let region = matches.value_of(args::UPLOAD_S3_REGION_ARG).unwrap();
            Some(RemoteStorage {
                access: config::RemoteStorageAccess::ReadWrite,
                name: None,
                storage_type: RemoteStorageType::S3(S3Config {
                    bucket: bucket.to_string(),
                    region: region.to_string(),
                    prefix: "".to_string(),
                    profile: None,
                }),
            })
        } else if let Some(output_dir) = matches.value_of(args::UPLOAD_OUTPUT_DIR_ARG) {
            let output_dir = Path::new(output_dir);
            if !output_dir.is_dir() {
                bail!(
                    "Specified output directory '{}' does not exist",
                    output_dir.display()
                )
            }
            Some(RemoteStorage {
                access: config::RemoteStorageAccess::ReadWrite,
                name: None,
                storage_type: RemoteStorageType::Path(PathConfig {
                    path: output_dir.to_path_buf(),
                }),
            })
        } else if let Some(name) = matches.value_of(args::UPLOAD_SERVER_NAME_ARG) {
            writable_servers.find(|server| server.name.as_ref().unwrap_or(&"".to_owned()) == name)
        } else {
            writable_servers.next()
        };
        if let Some(server) = server {
            upload::upload(search_path, recursive_search, &server, dryrun)
        } else {
            Err(anyhow!("No server specified in config for upload"))
        }
    } else if let Some(matches) = matches.subcommand_matches(args::LOGIN_SUBCOMMAND) {
        info!("Login subcommand");
        let service_name = matches.value_of(args::LOGIN_SERVICE_ARG).unwrap();
        match service_name {
            "github" => login::github_login()?,
            "symbolserver" => login::symbolserver_login()?,
            _ => bail!("Unknown service '{}'", service_name),
        };
        Ok(())
    } else {
        Ok(())
    }
}

fn initialize_logger(matches: &clap::ArgMatches) {
    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    let mut logger = pretty_env_logger::formatted_builder();
    let logger = match matches.occurrences_of(args::VERBOSITY_ARG) {
        0 => logger.filter_level(log::LevelFilter::Error),
        1 => logger.filter_level(log::LevelFilter::Warn),
        2 => logger.filter_level(log::LevelFilter::Info),
        3 => logger.filter_level(log::LevelFilter::Debug),
        _ => logger.filter_level(log::LevelFilter::Trace),
    };
    logger.init();
}
