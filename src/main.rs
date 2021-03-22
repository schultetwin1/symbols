use anyhow::{anyhow, Context, Result};
use log::{/*error,*/ /*debug,*/ info, trace /*warn */};

use std::{path::Path, path::PathBuf};

mod args;
mod config;
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
        let mut writable_servers = config
            .servers
            .iter()
            .filter(|server| server.access == config::RemoteStorageAccess::ReadWrite);
        let server = if let Some(name) = matches.value_of(args::UPLOAD_SERVER_NAME_ARG) {
            writable_servers.find(|server| server.name.as_ref().unwrap_or(&"".to_owned()) == name)
        } else {
            writable_servers.next()
        };
        if let Some(server) = server {
            upload::upload(search_path, recursive_search, server)
        } else {
            Err(anyhow!("No server specified in config for upload"))
        }
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
