use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use aws_config::Region;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::primitives::ByteStream;
use ignore::WalkBuilder;
use log::warn;
use serde::Serialize;
use sha2::{Digest, Sha256};
use symbolic_debuginfo::FileFormat;

use crate::config;
use crate::symstore;
use crate::symstore::file::{FileInfo, FileType, ResourceType};

#[derive(Serialize)]
struct SymbolServerUploadRequest {
    pub file_type: FileType,
    pub file_size: usize,
    pub file_name: String,
    pub identifier: String,
    pub resource_type: ResourceType,
    pub sha256: String,
}

pub fn upload(
    search_path: &Path,
    recursive: bool,
    server: &config::RemoteStorage,
    dryrun: bool,
) -> Result<()> {
    let obj_files = find_obj_files(search_path, recursive)?;
    let files = collet_file_info(&obj_files);
    match &server.storage_type {
        config::RemoteStorageType::Http(c) => Err(anyhow!(
            "Upload to HTTP server ({}) not yet implemented!",
            c.url
        )),
        config::RemoteStorageType::S3(c) => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(upload_to_s3(c, &files, dryrun))
        }
        config::RemoteStorageType::B2(c) => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(upload_to_b2(c, &files, dryrun))
        }
        config::RemoteStorageType::SymbolServer(c) => upload_to_symbolserver(c, &files, dryrun),
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
        if is_object_file(search_path).unwrap_or(FileFormat::Unknown) == FileFormat::Unknown {
            return Err(anyhow!(
                "Path \"{}\" is not a valid object file",
                search_path.display()
            ));
        }
        let files = vec![search_path.to_path_buf()];
        files
    };

    Ok(files)
}

fn collet_file_info(files: &[PathBuf]) -> Vec<FileInfo> {
    files
        .iter()
        .filter_map(|path| match symstore::file::file_to_info(path) {
            Ok(info) => {
                if let Some(info) = info {
                    Some(info)
                } else {
                    warn!("{} has no key", path.display());
                    None
                }
            }
            Err(_err) => {
                println!("Error parsing: {}", path.display());
                None
            }
        })
        .collect()
}

async fn upload_to_s3_helper(
    prefix: &str,
    files: &[FileInfo],
    dryrun: bool,
    client: aws_sdk_s3::Client,
    bucket: &String,
) -> Result<()> {
    for file in files {
        let key = file.key();
        let full_key = format!("{}{}", prefix, &key);
        println!(
            "uploading '{}' to s3 bucket '{}' with key '{}'",
            file.path.display(),
            bucket,
            full_key
        );
        if !dryrun {
            if client
                .head_object()
                .bucket(bucket)
                .key(&full_key)
                .send()
                .await
                .is_ok()
            {
                warn!(
                    "Skipping {} -> {} since the key already exists on server",
                    file.path.display(),
                    full_key
                );
                continue;
            }
            let body = ByteStream::from_path(&file.path).await?;
            client
                .put_object()
                .bucket(bucket)
                .key(&full_key)
                .body(body)
                .send()
                .await
                .context(format!("Failed to upload '{}' to S3", file.path.display()))?;
        }
    }

    Ok(())
}

async fn upload_to_s3(config: &config::S3Config, files: &[FileInfo], dryrun: bool) -> Result<()> {
    let builder = aws_config::profile::ProfileFileCredentialsProvider::builder();
    let builder = if let Some(profile) = &config.profile {
        builder.profile_name(profile)
    } else {
        builder
    };
    let provider = builder.build();
    let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .credentials_provider(provider)
        .region(Region::new(config.region.clone()))
        .load()
        .await;
    let client = aws_sdk_s3::Client::new(&sdk_config);
    upload_to_s3_helper(&config.prefix, files, dryrun, client, &config.bucket).await
}

async fn upload_to_b2(config: &config::B2Config, files: &[FileInfo], dryrun: bool) -> Result<()> {
    let b2_creds = match &config.account_id {
        Some(id) => b2creds::Credentials::from_file(None, Some(id))?,
        None => b2creds::Credentials::locate()?,
    };

    let creds = Credentials::new(
        &b2_creds.application_key_id,
        &b2_creds.application_key,
        None,
        None,
        "b2",
    );

    let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .credentials_provider(creds)
        .region("b2")
        .endpoint_url(config.endpoint.clone())
        .load()
        .await;
    let client = aws_sdk_s3::Client::new(&sdk_config);

    upload_to_s3_helper(&config.prefix, files, dryrun, client, &config.bucket).await
}

fn upload_to_symbolserver(
    config: &config::SymbolServerConfig,
    files: &[FileInfo],
    dryrun: bool,
) -> Result<()> {
    const SERVICE: &str = "com.symboserver.symbols";
    const USERNAME: &str = "symbolserver";
    let entry = keyring::Entry::new(SERVICE, USERNAME)?;
    let token = entry.get_password()?;
    let client = reqwest::blocking::Client::builder().build().unwrap();

    for file in files {
        println!(
            "uploading '{}' to symbolserver with key '{}'",
            file.path.display(),
            file.key()
        );
        if !dryrun {
            let mut f = match std::fs::File::open(&file.path) {
                Ok(f) => f,
                Err(error) => {
                    warn!(
                        "Failed to upload {}. Could not open file due to error: {}",
                        file.path.display(),
                        error
                    );
                    continue;
                }
            };
            let mut hasher = Sha256::new();
            if let Err(error) = std::io::copy(&mut f, &mut hasher) {
                warn!(
                    "Failed to read '{}' due to {:?} in order to generate hash. Skipping upload",
                    file.path.display(),
                    error
                );
                continue;
            }
            let hash = hasher.finalize();
            f.seek(std::io::SeekFrom::Start(0)).unwrap();

            let url = config
                .url
                .as_deref()
                .unwrap_or("https://api.symbolserver.com");

            let request = SymbolServerUploadRequest {
                file_name: file.path.file_name().unwrap().to_str().unwrap().to_string(),
                file_size: file.file_size,
                file_type: file.file_type,
                identifier: file.identifier.clone(),
                resource_type: file.resource_type,
                sha256: data_encoding::HEXUPPER.encode(&hash),
            };

            let res = match client
                .post(format!("{}/symbols/{}/upload/create", url, config.project))
                .bearer_auth(&token)
                .json(&request)
                .send()
            {
                Ok(response) => response,
                Err(error) => {
                    warn!(
                        "Failed to upload {}. Upload failed due to error: {}",
                        file.path.display(),
                        error
                    );
                    continue;
                }
            };

            if !res.status().is_success() {
                warn!(
                    "Upload of '{}' did not succeed. {}",
                    file.path.display(),
                    res.text().unwrap()
                );
                continue;
            }

            let signed_url = res.text().unwrap();
            let res = match client.put(&signed_url).body(f).send() {
                Ok(response) => response,
                Err(error) => {
                    warn!(
                        "Failed to upload {} to presigned url {}. Upload failed due to error: {}",
                        file.path.display(),
                        signed_url,
                        error
                    );
                    continue;
                }
            };

            if !res.status().is_success() {
                warn!(
                    "Upload of '{}' via pre-signed URL did not succeed. {}",
                    file.path.display(),
                    res.text().unwrap()
                );
                continue;
            }

            let res = match client
                .post(format!("{}/symbols/{}/upload/finish", url, config.project))
                .bearer_auth(&token)
                .json(&request)
                .send()
            {
                Ok(response) => response,
                Err(error) => {
                    warn!(
                        "Failed to upload {}. Upload failed due to error: {}",
                        file.path.display(),
                        error
                    );
                    continue;
                }
            };

            if !res.status().is_success() {
                warn!(
                    "Upload of '{}' failed to be marked successful. {}",
                    file.path.display(),
                    res.text().unwrap()
                );
                continue;
            }
        }
        println!("Uploaded '{}' to symbolserver.com", file.path.display());
    }
    Ok(())
}

fn copy_to_folder(config: &config::PathConfig, files: &[FileInfo], dryrun: bool) -> Result<()> {
    for file in files {
        let dest = config.path.join(&file.key());
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).context(format!(
                "Failed to create destination folder '{}'",
                parent.display()
            ))?;
        }
        println!("Copying '{}' to '{}'", file.path.display(), dest.display());
        if !dryrun {
            std::fs::copy(&file.path, &dest).context(format!(
                "Failed to copy '{}' to '{}'",
                file.path.display(),
                dest.display()
            ))?;
        }
    }

    Ok(())
}
