use anyhow::{Context, Result};
use sled::Db;
use std::path::Path;

pub struct KeyStore {
    db: Db,
}

impl KeyStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path).context("Failed to open keystore database")?;
        Ok(Self { db })
    }

    pub async fn store_keypair(&self, fingerprint: &str, public_key: &str) -> Result<()> {
        self.db
            .insert(fingerprint.as_bytes(), public_key.as_bytes())
            .context("Failed to store keypair")?;

        self.db.flush_async().await?;
        Ok(())
    }

    pub async fn get_keypair(&self, fingerprint: &str) -> Result<Option<String>> {
        if let Some(data) = self.db.get(fingerprint.as_bytes())? {
            let key = String::from_utf8(data.to_vec())?;
            Ok(Some(key))
        } else {
            Ok(None)
        }
    }
}
