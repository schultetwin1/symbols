use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use ignore::WalkBuilder;
use log::warn;
use symbolic_debuginfo::FileFormat;

use crate::config;
use crate::symstore;

pub fn upload(search_path: &Path, recursive: bool, server: &config::RemoteStorage, dryrun: bool) -> Result<()> {
    let files = find_obj_files(search_path, recursive)?;
    let files = map_files_to_keys(&files);
    match &server.storage_type {
        config::RemoteStorageType::HTTP(c) => Err(anyhow!(
            "Upload to HTTP server ({}) not yet implemented!",
            c.url
        )),
        config::RemoteStorageType::S3(c) => upload_to_s3(c, &files, dryrun),
        config::RemoteStorageType::B2(c) => upload_to_b2(c, &files, dryrun),
        config::RemoteStorageType::Path(c) => copy_to_folder(c, &files, dryrun),
    }
}

fn is_object_file(path: &Path) -> std::io::Result<FileFormat> {
    let mut file = std::fs::OpenOptions::new()
        .write(false)
        .read(true)
        .open(path)?;

    let mut magic = [0u8; 256];

    if (file.metadata().unwrap().len() as usize) < magic.len() {
        return Ok(FileFormat::Unknown);
    }

    file.read_exact(&mut magic)?;

    Ok(symbolic_debuginfo::peek(&magic, false))
}

fn find_obj_files(search_path: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
    if !search_path.exists() {
        return Err(anyhow!(
            "Path \"{}\" doest not exists",
            search_path.display()
        ));
    }

    let max_depth = if recursive { None } else { Some(1) };

    let files = if search_path.is_dir() {
        WalkBuilder::new(search_path)
            .max_depth(max_depth)
            .git_ignore(false)
            .build()
            .filter_map(|v| v.ok())
            .filter(|x| x.path().is_file())
            .filter(|x| {
                is_object_file(x.path()).unwrap_or(FileFormat::Unknown) != FileFormat::Unknown
            })
            .map(|x| x.into_path())
            .collect::<Vec<std::path::PathBuf>>()
    } else {
        let mut files = Vec::new();
        files.push(search_path.to_path_buf());
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

fn upload_to_s3(config: &config::S3Config, files: &Vec<(PathBuf, String)>, dryrun: bool) -> Result<()> {
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
        if !dryrun {
            bucket.put_object_stream_blocking(&file.0, full_key)?;
        }
    }

    Ok(())
}

fn upload_to_b2(config: &config::B2Config, files: &Vec<(PathBuf, String)>, dryrun: bool) -> Result<()> {
    let b2_creds = match &config.account_id {
        Some(id) => b2creds::Credentials::from_file(None, Some(&id))?,
        None => b2creds::Credentials::default()?,
    };

    let creds = s3::creds::Credentials::new(
        Some(&b2_creds.application_key_id),
        Some(&b2_creds.application_key),
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
        if !dryrun {
            bucket.put_object_stream_blocking(&file.0, full_key)?;
        }
    }

    Ok(())
}

fn copy_to_folder(config: &config::PathConfig, files: &Vec<(PathBuf, String)>, dryrun: bool) -> Result<()> {
    for file in files {
        let dest = config.path.join(&file.1);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).context(format!(
                "Failed to create destination folder '{}'",
                parent.display()
            ))?;
        }
        println!("Copying '{}' to '{}'", file.0.display(), dest.display());
        if !dryrun {
            std::fs::copy(&file.0, &dest).context(format!(
                "Failed to copy '{}' to '{}'",
                file.0.display(),
                dest.display()
            ))?;
        }
    }

    Ok(())
}
