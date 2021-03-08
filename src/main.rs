use anyhow::{anyhow, Context, Result};
use ignore::WalkBuilder;
use log::{/*error,*/ /*debug,*/ info, trace, warn};

use std::{io::Read, path::Path, path::PathBuf};

mod args;
mod b2;
mod config;
mod symstore;

#[derive(Debug, PartialEq, Eq)]
enum FileType {
    Elf,
    PE,
    PDB,
    /* MachO, */
    Unknown,
}

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
        let files = find_obj_files(&matches)?;
        let files = map_files_to_keys(&files);
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
            match &server.storage_type {
                config::RemoteStorageType::HTTP(c) => Err(anyhow!(
                    "Upload to HTTP server ({}) not yet implemented!",
                    c.url
                )),
                config::RemoteStorageType::S3(c) => upload_to_s3(c, &files),
                config::RemoteStorageType::B2(c) => upload_to_b2(c, &files),
                config::RemoteStorageType::Path(c) => copy_to_folder(c, &files),
            }
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

fn is_object_file(path: &Path) -> std::io::Result<FileType> {
    const ELF_MAGIC_BYTES: &[u8; 4] = b"\x7FELF";
    const PDB_MAGIC_BYTES: &[u8; 26] = b"Microsoft C/C++ MSF 7.00\r\n";

    let mut file = std::fs::OpenOptions::new()
        .write(false)
        .read(true)
        .open(path)?;

    let mut magic = [0u8; 256];

    if (file.metadata().unwrap().len() as usize) < magic.len() {
        return Ok(FileType::Unknown);
    }

    file.read_exact(&mut magic)?;

    if magic.starts_with(ELF_MAGIC_BYTES) {
        Ok(FileType::Elf)
    } else if magic[0..2] == b"MZ"[..] || magic[0..2] == b"ZM"[..] {
        Ok(FileType::PE)
    } else if magic.starts_with(PDB_MAGIC_BYTES) {
        Ok(FileType::PDB)
    } else {
        Ok(FileType::Unknown)
    }
}

fn find_obj_files(matches: &clap::ArgMatches) -> Result<Vec<PathBuf>> {
    let path = std::path::Path::new(matches.value_of(args::UPLOAD_PATH_ARG).unwrap());

    if !path.exists() {
        return Err(anyhow!("Path \"{}\" doest not exists", path.display()));
    }

    let max_depth = if matches.is_present(args::UPLOAD_RECUSRIVE_ARG) {
        None
    } else {
        Some(1)
    };

    let files = if path.is_dir() {
        WalkBuilder::new(path)
            .max_depth(max_depth)
            .git_ignore(false)
            .build()
            .filter_map(|v| v.ok())
            .filter(|x| x.path().is_file())
            .filter(|x| is_object_file(x.path()).unwrap_or(FileType::Unknown) != FileType::Unknown)
            .map(|x| x.into_path())
            .collect::<Vec<std::path::PathBuf>>()
    } else {
        let mut files = Vec::new();
        files.push(path.to_path_buf());
        files
    };

    Ok(files)
}

fn map_files_to_keys(files: &Vec<PathBuf>) -> Vec<(PathBuf, String)> {
    let mut map: Vec<(PathBuf, String)> = Vec::new();
    for file in files {
        match symstore::file::file_to_key(&file) {
            Ok(key) => {
                if let Some(key) = key {
                    map.push((file.clone(), key));
                } else {
                    warn!("{} has no key", file.display());
                }
            }
            Err(_err) => {
                println!("Error parsing: {}", file.display());
            }
        }
    }

    map
}

fn upload_to_s3(config: &config::S3Config, files: &Vec<(PathBuf, String)>) -> Result<()> {
    let creds = s3::creds::Credentials::new(None, None, None, None, config.profile.as_deref())?;
    let region = config.region.parse()?;
    let bucket = s3::bucket::Bucket::new(&config.bucket, region, creds)?;

    // Files to upload
    for file in files {
        let full_key = format!("{}{}", &config.prefix, &file.1);
        println!(
            "uploading '{}' to s3 bucket '{}' with key '{}'",
            file.0.display(),
            config.bucket,
            full_key
        );
        bucket.put_object_stream_blocking(&file.0, full_key)?;
    }

    Ok(())
}

fn upload_to_b2(config: &config::B2Config, files: &Vec<(PathBuf, String)>) -> Result<()> {
    let b2_creds = match b2::Credentials::from_env() {
        Some(creds) => creds,
        None => {
            return Err(anyhow!("Failed to find any b2 credentials"));
        }
    };
    let creds = s3::creds::Credentials::new(
        Some(&b2_creds.key_id),
        Some(&b2_creds.key),
        None,
        None,
        None,
    )?;
    let region = s3::region::Region::Custom {
        region: "b2".to_owned(),
        endpoint: config.endpoint.clone(),
    };

    let bucket = s3::bucket::Bucket::new(&config.bucket, region, creds)?;

    // Files to upload
    for file in files {
        let full_key = format!("{}{}", &config.prefix, &file.1);
        println!(
            "uploading '{}' to b2 bucket '{}' with key '{}'",
            file.0.display(),
            config.bucket,
            full_key
        );
        bucket.put_object_stream_blocking(&file.0, full_key)?;
    }

    Ok(())
}

fn copy_to_folder(config: &config::PathConfig, files: &Vec<(PathBuf, String)>) -> Result<()> {
    for file in files {
        let dest = config.path.join(&file.1);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).context(format!(
                "Failed to create destination folder '{}'",
                parent.display()
            ))?;
        }
        println!("Copying '{}' to '{}'", file.0.display(), dest.display());
        std::fs::copy(&file.0, &dest).context(format!(
            "Failed to copy '{}' to '{}'",
            file.0.display(),
            dest.display()
        ))?;
    }

    Ok(())
}
