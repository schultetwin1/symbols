use std::env;

#[derive(Debug)]
pub struct Credentials {
    pub key_id: String,
    pub key: String,
}

impl Credentials {
    pub fn from_env() -> Option<Self> {
        let key_id = env::var("B2_KEY_ID").ok()?;
        let key = env::var("B2_KEY").ok()?;

        Some(Self { key_id, key })
    }

    // Reading from ~/.b2_account_info should be added
}
