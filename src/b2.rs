use log::{error, /*debug,*/ /*info, */ /*trace, */ warn};
use rusqlite::OpenFlags;
use std::{
    env,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Credentials {
    pub key_id: String,
    pub key: String,
}

impl Credentials {
    pub fn from_account_id(account_id: Option<&str>) -> Option<Self> {
        match Self::from_env() {
            Some(creds) => Some(creds),
            None => {
                let info_path = if let Ok(info_path_env_str) = std::env::var("B2_ACCOUNT_INFO") {
                    PathBuf::from(info_path_env_str)
                } else {
                    let base_dirs = directories::BaseDirs::new()?;
                    PathBuf::from(base_dirs.home_dir()).join(".b2_account_info")
                };
                Self::from_file(&info_path, account_id)
            }
        }
    }

    pub fn from_env() -> Option<Self> {
        let key_id = env::var("B2_KEY_ID").ok()?;
        let key = env::var("B2_KEY").ok()?;

        Some(Self { key_id, key })
    }

    // Reading from ~/.b2_account_info should be added
    pub fn from_file(db_path: &Path, account_id: Option<&str>) -> Option<Self> {
        if !db_path.exists() {
            warn!(
                "Path to b2 database file does not exit {}",
                db_path.display()
            );
            return None;
        }

        let conn = match rusqlite::Connection::open_with_flags(
            db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        ) {
            Ok(db) => db,
            Err(e) => {
                error!(
                    "Failed tp read b2 database file at {} due to {}",
                    db_path.display(),
                    e
                );
                return None;
            }
        };

        let mut query = String::from(
            "SELECT account_id, application_key, account_id_or_app_key_id FROM account",
        );
        if account_id.is_some() {
            query = format!("{} WHERE account_id = \"{}\"", query, account_id.unwrap());
        }
        let mut stmt = match conn.prepare(&query) {
            Ok(stmt) => stmt,
            Err(e) => {
                error!("Failed to query b2 database for account: {}", e);
                return None;
            }
        };

        let mut creds_iter = match stmt.query_map(rusqlite::NO_PARAMS, |row| {
            Ok(Credentials {
                key_id: row.get(2).unwrap(),
                key: row.get(1).unwrap(),
            })
        }) {
            Ok(iter) => iter,
            Err(e) => {
                error!("Failed to map query results for b2 database {}", e);
                return None;
            }
        };

        for cred in creds_iter.next()? {
            return Some(cred);
        }

        return None;
    }
}
