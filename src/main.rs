use log::{/*error,*/ /*debug,*/ info, trace, warn};
use ignore::WalkBuilder;
use anyhow::{anyhow, Result};

use std::io::Read;

mod symstore;
mod args;

#[derive(Debug, PartialEq, Eq)]
enum FileType {
    Elf,
    PE,
    PDB,
    /* MachO, */
    Unknown
}

const ELF_MAGIC_BYTES: &[u8; 4] = b"\x7FELF";
const PDB_MAGIC_BYTES: &[u8; 28] = b"Microsoft C / C++ MSF 7.00\r\n";

fn is_object_file(path: &std::path::Path) -> std::io::Result<FileType> {
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
    } else if magic [0..2] == b"MZ"[..] || magic[0..2] == b"ZM"[..] {
        Ok(FileType::PE)
    } else if magic.starts_with(PDB_MAGIC_BYTES) {
        Ok(FileType::PDB)
    } else {
        Ok(FileType::Unknown)
    }

}

fn main() -> Result<()> {
    let matches = args::parse_args();
    initialize_logger(&matches);
    trace!("logger initialized");

    if let Some(matches) = matches.subcommand_matches(args::UPLOAD_SUBCOMMAND) {
        info!("Upload subcommand");
        upload_dbg_info(matches)
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
        4 | _ => logger.filter_level(log::LevelFilter::Trace),
    };
    logger.init();
}

fn upload_dbg_info(matches: &clap::ArgMatches) -> Result<()> {
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
            .into_iter()
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

    // Files to upload
    for file in files {
        match symstore::file::file_to_key(&file) {
            Ok(key) => {
                if let Some(key) = key {
                    println!("uploading {} -> {}", file.display(), key);
                } else {
                    warn!("{} has no key", file.display());
                }
            }
            Err(_err) => {
                println!("Error parsing: {}", file.display());
            }
        }
    }

    Ok(())
}
