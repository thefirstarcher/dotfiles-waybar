use crate::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Simple file-based cache for module data
pub struct Cache {
    cache_dir: PathBuf,
    ttl: Duration,
}

impl Cache {
    /// Create a new cache with specified directory and TTL
    pub fn new(cache_dir: impl AsRef<Path>, ttl: Duration) -> Result<Self> {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        fs::create_dir_all(&cache_dir)?;

        Ok(Self { cache_dir, ttl })
    }

    /// Get cache path for a key
    fn get_path(&self, key: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.json", key))
    }

    /// Check if cache entry is valid (exists and not expired)
    pub fn is_valid(&self, key: &str) -> bool {
        let path = self.get_path(key);

        if !path.exists() {
            return false;
        }

        if let Ok(metadata) = fs::metadata(&path) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                    return elapsed < self.ttl;
                }
            }
        }

        false
    }

    /// Get cached value if valid
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        if !self.is_valid(key) {
            return Ok(None);
        }

        let path = self.get_path(key);
        let content = fs::read_to_string(path)?;
        let value = serde_json::from_str(&content)?;

        Ok(Some(value))
    }

    /// Get cached value or compute it
    pub fn get_or_compute<T, F>(&self, key: &str, compute: F) -> Result<T>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Result<T>,
    {
        if let Some(cached) = self.get(key)? {
            tracing::debug!("Cache hit for key: {}", key);
            return Ok(cached);
        }

        tracing::debug!("Cache miss for key: {}, computing...", key);
        let value = compute()?;
        self.set(key, &value)?;

        Ok(value)
    }

    /// Set cache value
    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let path = self.get_path(key);
        let json = serde_json::to_string_pretty(value)?;
        fs::write(path, json)?;

        Ok(())
    }

    /// Invalidate (delete) cache entry
    pub fn invalidate(&self, key: &str) -> Result<()> {
        let path = self.get_path(key);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Clear all cache entries
    pub fn clear_all(&self) -> Result<()> {
        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                fs::remove_file(entry.path())?;
            }
        }
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let mut total_files = 0;
        let mut total_size = 0;
        let mut expired = 0;

        if let Ok(entries) = fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    total_files += 1;

                    if let Ok(metadata) = entry.metadata() {
                        total_size += metadata.len();

                        if let Ok(modified) = metadata.modified() {
                            if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                                if elapsed >= self.ttl {
                                    expired += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        CacheStats {
            total_files,
            total_size,
            expired,
        }
    }
}

/// Cache statistics
#[derive(Debug)]
pub struct CacheStats {
    pub total_files: usize,
    pub total_size: u64,
    pub expired: usize,
}

/// Extension trait for easier cache creation
pub trait CacheExt {
    fn with_ttl_secs(self, secs: u64) -> Result<Cache>;
    fn with_ttl_mins(self, mins: u64) -> Result<Cache>;
}

impl<P: AsRef<Path>> CacheExt for P {
    fn with_ttl_secs(self, secs: u64) -> Result<Cache> {
        Cache::new(self, Duration::from_secs(secs))
    }

    fn with_ttl_mins(self, mins: u64) -> Result<Cache> {
        Cache::new(self, Duration::from_secs(mins * 60))
    }
}
