use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use ignore::WalkBuilder;
use log::{debug, warn};
use symbolic_debuginfo::FileFormat;

use crate::config;
use crate::symstore;

pub fn upload(
    search_path: &Path,
    recursive: bool,
    server: &config::RemoteStorage,
    dryrun: bool,
) -> Result<()> {
    let files = find_obj_files(search_path, recursive)?;
    let files = map_files_to_keys(&files);
    match &server.storage_type {
        config::RemoteStorageType::Http(c) => Err(anyhow!(
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
        let files = vec![search_path.to_path_buf()];
        files
    };

    Ok(files)
}

fn map_files_to_keys(files: &[PathBuf]) -> HashMap<String, PathBuf> {
    let mut map: HashMap<String, PathBuf> = HashMap::new();
    for file in files {
        match symstore::file::file_to_key(&file) {
            Ok(key) => {
                if let Some(key) = key {
                    let exists = map.insert(key, file.clone());
                    if let Some(old) = exists {
                        warn!("Overwrote {} with {}", old.display(), file.display());
                    }
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

async fn upload_to_s3_helper(
    prefix: &str,
    files: &HashMap<String, PathBuf>,
    dryrun: bool,
    bucket: s3::bucket::Bucket,
) -> Result<()> {
    for file in files {
        let full_key = format!("{}{}", prefix, &file.0);
        println!(
            "uploading '{}' to s3 bucket '{}' with key '{}'",
            file.1.display(),
            bucket.name,
            full_key
        );
        if !dryrun {
            let (_head_object_result, code) = bucket.head_object(&full_key).await?;
            debug!("Head Object for {} returned {}", full_key, code);
            if code != 200 {
                bucket
                    .put_object_stream(&file.1, &full_key)
                    .await
                    .context(format!("Failed to upload '{}' to S3", file.1.display()))?;
            } else {
                warn!(
                    "Skipping {} -> {} since the key already exists on server",
                    file.1.display(),
                    full_key
                );
            }
        }
    }

    Ok(())
}

fn upload_to_s3(
    config: &config::S3Config,
    files: &HashMap<String, PathBuf>,
    dryrun: bool,
) -> Result<()> {
    let creds = s3::creds::Credentials::new(None, None, None, None, config.profile.as_deref())?;
    let region = config.region.parse()?;
    let bucket = s3::bucket::Bucket::new(&config.bucket, region, creds)?;

    let mut rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(upload_to_s3_helper(&config.prefix, files, dryrun, bucket))
}

fn upload_to_b2(
    config: &config::B2Config,
    files: &HashMap<String, PathBuf>,
    dryrun: bool,
) -> Result<()> {
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

    let mut rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(upload_to_s3_helper("", files, dryrun, bucket))
}

fn copy_to_folder(
    config: &config::PathConfig,
    files: &HashMap<String, PathBuf>,
    dryrun: bool,
) -> Result<()> {
    for file in files {
        let dest = config.path.join(&file.0);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).context(format!(
                "Failed to create destination folder '{}'",
                parent.display()
            ))?;
        }
        println!("Copying '{}' to '{}'", file.1.display(), dest.display());
        if !dryrun {
            std::fs::copy(&file.1, &dest).context(format!(
                "Failed to copy '{}' to '{}'",
                file.1.display(),
                dest.display()
            ))?;
        }
    }

    Ok(())
}
