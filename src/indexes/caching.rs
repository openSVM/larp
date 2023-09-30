// We are going to store caches here for what we have indexed already
// and what we still have to index
//
// Now the way we want to go about doing this:
// we use a fs based system and wrap it in a lock so we are okay with things

use std::path::PathBuf;

use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

// Indexes might require their own keys, we know tantivy is fucked because
// it expects a key which is unique to the doc schema which you put in...
// so if we query it with the wrong schema it blows in your face :|
// for now we care about tantivy so lets get that working
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct CacheKeys {
    tantivy: String,
}

impl CacheKeys {
    pub fn tantivy(&self) -> &str {
        &self.tantivy
    }
}

#[derive(serde::Serialize, serde::Deserialize, Eq)]
pub struct FreshValue<T> {
    // default value is `false` on deserialize
    pub(crate) fresh: bool,
    pub(crate) value: T,
}

impl<T: Default> FreshValue<T> {
    fn fresh_default() -> Self {
        Self {
            fresh: true,
            value: Default::default(),
        }
    }
}

impl<T> PartialEq for FreshValue<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(&other.value)
    }
}

impl<T> FreshValue<T> {
    fn stale(value: T) -> Self {
        Self {
            fresh: false,
            value,
        }
    }
}

impl<T> From<T> for FreshValue<T> {
    fn from(value: T) -> Self {
        Self { fresh: true, value }
    }
}

/// This is the storage for the underlying struct which we will use to store
/// anything and everything
pub struct FSStorage<T: Serialize + DeserializeOwned + PartialEq> {
    source: T,
    path: PathBuf,
    write_lock: Mutex<()>,
}

impl<T: Serialize + DeserializeOwned + PartialEq> FSStorage<T> {
    pub fn new(source: T, path: PathBuf) -> Self {
        Self {
            source,
            path,
            write_lock: Mutex::new(()),
        }
    }

    // This will store the underlying data to the path we are interested in
    pub async fn store_to_path(&self) -> Result<()> {
        // We take the lock here, since we want to be the only ones writing
        // to it for correctness sake
        let _lock = self.write_lock.lock().await;
        tokio::fs::create_dir_all(self.path.parent().unwrap()).await?;
        let mut file = tokio::fs::File::create(&self.path).await?;
        let data = serde_json::to_vec(&self.source)?;
        file.write_all(&data).await?;
        Ok(())
    }
}
