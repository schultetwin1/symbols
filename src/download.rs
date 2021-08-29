use std::path::Path;

use anyhow::{anyhow, Result};

use crate::config;

pub fn download(
  download_path: &Path,
  server: &config::RemoteStorage,
  dryrun: bool,
) -> Result<()> {
    println!("Download!!");
    match &server.storage_type {
        config::RemoteStorageType::Http(c) => Err(anyhow!(
            "Upload to HTTP server ({}) not yet implemented!",
            c.url
        )),
        config::RemoteStorageType::S3(c) => Err(anyhow!(
          "Download from S3 bucket ({}) not yet implemented!",
          c.bucket
        )),
        config::RemoteStorageType::B2(c) => Err(anyhow!(
          "Download from B2 bucket ({}) not yet implemented!",
          c.bucket
        )),
        config::RemoteStorageType::Path(c) => copy_to_folder(c, dryrun),
    }
}

fn copy_to_folder(
  config: &config::PathConfig,
  dryrun: bool
) -> Result<()> {
  /*
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
}*/

  Ok(())
}